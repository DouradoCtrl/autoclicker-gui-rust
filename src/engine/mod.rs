use crate::config::{AppConfig, ClickMode, TriggerType};
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, EventType, InputEvent, Key, RelativeAxisType};
use rand::Rng;
use rand_distr::{Distribution, Normal};
use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    EventDetected {
        key_code: u16,
        device_name: String,
        device_path: String,
    },
}

pub enum EngineCommand {
    UpdateActiveConfigs(Vec<AppConfig>),
    SetMasterState(bool),
    StartListeningGlobal,
    StartListeningSpecific(String),
    StopListening,
}

pub fn set_nonblocking(fd: std::os::unix::io::RawFd) -> std::io::Result<()> {
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags < 0 {
            return Err(std::io::Error::last_os_error());
        }
        if libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

pub fn scan_devices() -> Vec<DeviceInfo> {
    let mut devices = Vec::new();
    for (path, device) in evdev::enumerate() {
        let name = device.name().unwrap_or("Dispositivo Desconhecido").to_string();
        if name.contains("Humanized AutoClicker") {
            continue;
        }
        devices.push(DeviceInfo { name, path });
    }
    devices
}

pub fn check_uinput_permission() -> bool {
    fs::OpenOptions::new().write(true).open("/dev/uinput").is_ok()
}

pub fn check_input_permission() -> bool {
    if let Ok(entries) = fs::read_dir("/dev/input") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.to_string_lossy().contains("event") {
                if fs::OpenOptions::new().read(true).open(&path).is_ok() {
                    return true;
                }
            }
        }
    }
    false
}

struct ListeningDevice {
    path: String,
    name: String,
    device: evdev::Device,
}

struct ActiveProfileState {
    config: AppConfig,
    trigger_device: Option<evdev::Device>,
    virtual_device: Option<VirtualDevice>,
    is_clicking: bool,
    click_counter: u64,
    last_click: Instant,
    next_delay: Duration,
    arm_time: Option<Instant>,
}

pub struct AutoClickerEngine {
    cmd_rx: Receiver<EngineCommand>,
    ui_tx: relm4::Sender<crate::ui::AppMsg>,
    active_configs: Vec<AppConfig>,
    active_runners: Vec<ActiveProfileState>,
    is_master_active: bool,
    is_listening: bool,
    listening_devices: Vec<ListeningDevice>,
}

impl AutoClickerEngine {
    pub fn spawn(cmd_rx: Receiver<EngineCommand>, ui_tx: relm4::Sender<crate::ui::AppMsg>) {
        thread::spawn(move || {
            let mut engine = AutoClickerEngine {
                cmd_rx,
                ui_tx,
                active_configs: Vec::new(),
                active_runners: Vec::new(),
                is_master_active: false,
                is_listening: false,
                listening_devices: Vec::new(),
            };
            engine.run();
        });
    }

    fn create_virtual_device(action_code: u16) -> Option<VirtualDevice> {
        let mut keys = AttributeSet::<Key>::new();
        // Always include mouse buttons
        keys.insert(Key::BTN_LEFT);
        keys.insert(Key::BTN_RIGHT);
        keys.insert(Key::BTN_MIDDLE);
        // Include the specific action key from the profile
        keys.insert(Key::new(action_code));
        
        // Mouse-like relative axes so Linux treats this as a real pointer device
        let mut rel_axes = AttributeSet::<RelativeAxisType>::new();
        rel_axes.insert(RelativeAxisType::REL_X);
        rel_axes.insert(RelativeAxisType::REL_Y);
        
        let result = evdev::uinput::VirtualDeviceBuilder::new()
            .ok()?
            .name("Humanized AutoClicker Virtual Device")
            .with_keys(&keys)
            .ok()?
            .with_relative_axes(&rel_axes)
            .ok()?
            .build()
            .ok();
        
        if result.is_some() {
            eprintln!("[ENGINE] Virtual device created for action_code={}", action_code);
        } else {
            eprintln!("[ENGINE] FAILED to create virtual device!");
        }
        result
    }

    fn initialize_runners(&mut self) {
        self.active_runners.clear();
        for cfg in &self.active_configs {
            let trigger_device = if let Some(ref path) = cfg.trigger_device_path {
                match evdev::Device::open(path) {
                    Ok(dev) => {
                        let _ = set_nonblocking(dev.as_raw_fd());
                        eprintln!("[ENGINE] Trigger device opened: {}", path);
                        Some(dev)
                    }
                    Err(e) => {
                        eprintln!("[ENGINE] FAILED to open trigger device {}: {}", path, e);
                        None
                    }
                }
            } else {
                None
            };

            let virtual_device = Self::create_virtual_device(cfg.target_action_code);

            self.active_runners.push(ActiveProfileState {
                config: cfg.clone(),
                trigger_device,
                virtual_device,
                is_clicking: false,
                click_counter: 0,
                last_click: Instant::now(),
                next_delay: Duration::from_millis(100),
                arm_time: Some(Instant::now()),
            });
        }
    }

    fn run(&mut self) {
        loop {
            while let Ok(cmd) = self.cmd_rx.try_recv() {
                match cmd {
                    EngineCommand::UpdateActiveConfigs(configs) => {
                        self.active_configs = configs;
                        if self.is_master_active {
                            self.initialize_runners();
                        }
                    }
                    EngineCommand::SetMasterState(active) => {
                        eprintln!("[ENGINE] SetMasterState: {}", active);
                        self.is_master_active = active;
                        if active {
                            self.initialize_runners();
                        } else {
                            self.active_runners.clear();
                            eprintln!("[ENGINE] Deactivated, devices closed.");
                        }
                    }
                    EngineCommand::StartListeningGlobal => {
                        self.is_listening = true;
                        self.listening_devices.clear();
                        for info in scan_devices() {
                            if let Ok(dev) = evdev::Device::open(&info.path) {
                                let _ = set_nonblocking(dev.as_raw_fd());
                                self.listening_devices.push(ListeningDevice {
                                    path: info.path.to_string_lossy().to_string(),
                                    name: info.name,
                                    device: dev,
                                });
                            }
                        }
                    }
                    EngineCommand::StartListeningSpecific(path) => {
                        self.is_listening = true;
                        self.listening_devices.clear();
                        if let Ok(dev) = evdev::Device::open(&path) {
                            let _ = set_nonblocking(dev.as_raw_fd());
                            let name = dev.name().unwrap_or("Dispositivo").to_string();
                            self.listening_devices.push(ListeningDevice {
                                path,
                                name,
                                device: dev,
                            });
                        }
                    }
                    EngineCommand::StopListening => {
                        self.is_listening = false;
                        self.listening_devices.clear();
                    }
                }
            }

            // Wizard detection mode
            if self.is_listening {
                let mut found = false;
                for ld in &mut self.listening_devices {
                    if let Ok(events) = ld.device.fetch_events() {
                        for ev in events {
                            if ev.event_type() == EventType::KEY && ev.value() == 1 {
                                let key_code = ev.code();
                                let _ = self.ui_tx.send(crate::ui::AppMsg::EngineEvent(EngineEvent::EventDetected {
                                    key_code,
                                    device_name: ld.name.clone(),
                                    device_path: ld.path.clone(),
                                }));
                                self.is_listening = false;
                                found = true;
                                break;
                            }
                        }
                    }
                    if found {
                        break;
                    }
                }
                if found {
                    self.listening_devices.clear();
                }
            }

            // AutoClick active mode
            if self.is_master_active && !self.is_listening {
                for runner in &mut self.active_runners {
                    // Read trigger device events
                    if let Some(ref mut dev) = runner.trigger_device {
                        if let Ok(events) = dev.fetch_events() {
                            // Safety: don't trigger within 500ms of arming
                            let can_trigger = runner.arm_time
                                .map(|t| t.elapsed() > Duration::from_millis(500))
                                .unwrap_or(true);
                                
                            for ev in events {
                                if ev.event_type() == EventType::KEY && ev.code() == runner.config.trigger_code {
                                    if can_trigger {
                                        match runner.config.trigger_type {
                                            TriggerType::Hold => {
                                                let new_state = ev.value() != 0;
                                                if new_state != runner.is_clicking {
                                                    eprintln!("[ENGINE] Hold trigger: clicking={}", new_state);
                                                }
                                                runner.is_clicking = new_state;
                                            }
                                            TriggerType::Toggle => {
                                                if ev.value() == 1 {
                                                    runner.is_clicking = !runner.is_clicking;
                                                    eprintln!("[ENGINE] Toggle trigger: clicking={}", runner.is_clicking);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Emit clicks
                    if runner.is_clicking {
                        let now = Instant::now();
                        if now.duration_since(runner.last_click) >= runner.next_delay {
                            if let Some(ref mut vdev) = runner.virtual_device {
                                Self::emit_click(vdev, runner.config.click_mode, runner.config.target_action_code, runner.config.double_click_delay_ms);
                            }
                            runner.last_click = Instant::now();
                            runner.next_delay = Self::calculate_runner_delay(&runner.config, &mut runner.click_counter);
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(2));
        }
    }

    fn emit_click(vdev: &mut VirtualDevice, mode: ClickMode, action_code: u16, double_click_delay_ms: u64) {
        let press = InputEvent::new(EventType::KEY, action_code, 1);
        let release = InputEvent::new(EventType::KEY, action_code, 0);
        let sync = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);

        match mode {
            ClickMode::Humanized | ClickMode::Fixed => {
                let _ = vdev.emit(&[press, sync]);
                thread::sleep(Duration::from_millis(5));
                let _ = vdev.emit(&[release, sync]);
            }
            ClickMode::DoubleClick => {
                let _ = vdev.emit(&[press, sync]);
                thread::sleep(Duration::from_millis(5));
                let _ = vdev.emit(&[release, sync]);
                thread::sleep(Duration::from_millis(double_click_delay_ms));
                let _ = vdev.emit(&[press, sync]);
                thread::sleep(Duration::from_millis(5));
                let _ = vdev.emit(&[release, sync]);
            }
        }
    }

    fn calculate_runner_delay(config: &AppConfig, click_counter: &mut u64) -> Duration {
        *click_counter += 1;
        let base_interval_ms = 1000.0 / (config.target_cps.max(1) as f64);

        match config.click_mode {
            ClickMode::Fixed | ClickMode::DoubleClick => {
                Duration::from_secs_f64(base_interval_ms / 1000.0)
            }
            ClickMode::Humanized => {
                let mut rng = rand::thread_rng();
                
                if *click_counter > rng.gen_range(40..70) {
                    *click_counter = 0;
                    let pause_ms = rng.gen_range(180.0..380.0);
                    return Duration::from_millis(pause_ms as u64);
                }

                let std_dev = base_interval_ms * 0.15;
                if let Ok(normal) = Normal::new(base_interval_ms, std_dev) {
                    let jittered = normal.sample(&mut rng).max(10.0);
                    Duration::from_millis(jittered as u64)
                } else {
                    Duration::from_secs_f64(base_interval_ms / 1000.0)
                }
            }
        }
    }
}
