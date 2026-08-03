use crate::config::{AppConfig, ClickMode, TriggerType};
use crate::engine::{check_input_permission, check_uinput_permission, AutoClickerEngine, EngineCommand, EngineEvent};
use crate::profiles::{Profile, ProfileManager};
use gtk4::prelude::*;
use libadwaita::prelude::*;
use relm4::{
    adw, gtk, ComponentParts, ComponentSender, SimpleComponent, RelmWidgetExt,
};
use std::sync::mpsc::{channel, Sender as MpscSender};

#[derive(PartialEq)]
pub enum ViewState {
    Main,
    Wizard,
    ConfirmDelete,
}

#[derive(PartialEq, Clone, Copy)]
pub enum WizardStep {
    DetectTargetDevice,
    DetectTargetAction,
    DetectTrigger,
    SelectModeAndSpeed,
}

pub struct AppModel {
    is_active: bool,
    profiles: Vec<Profile>,
    selected_profile_index: usize,
    has_uinput_permission: bool,
    has_input_permission: bool,
    
    view_state: ViewState,
    wizard_step: WizardStep,
    wizard_config: AppConfig,
    wizard_target_device_name: String,
    wizard_action_name: String,
    wizard_trigger_name: String,
    wizard_profile_name: String,
    
    profile_strings: gtk::StringList,
    engine_tx: MpscSender<EngineCommand>,
    profile_manager: ProfileManager,
    profile_list: relm4::factory::FactoryVecDeque<ProfileRow>,
}

#[derive(Debug)]
pub enum AppMsg {
    ToggleMaster(bool),
    StartWizard,
    CancelWizard,
    NextWizardStep,
    PrevWizardStep,
    RetryDetection,
    FinishWizard,
    
    AskDeleteProfile(usize),
    CancelDelete,
    ConfirmDelete,
    ToggleProfile(usize, bool),
    
    WizardSetMode(TriggerType),
    WizardSetClickMode(ClickMode),
    WizardSetCps(u32),
    WizardSetName(String),
    
    CheckPermissions,
    
    EngineEvent(EngineEvent),
}

fn format_profile_desc(p: &Profile) -> String {
    let mode_str = match p.config.click_mode {
        ClickMode::Humanized => "Humanizado",
        ClickMode::Fixed => "Fixo",
        ClickMode::DoubleClick => "Duplo Clique",
    };
    let trigger_str = match p.config.trigger_type {
        TriggerType::Hold => "Segurar",
        TriggerType::Toggle => "Alternar",
    };
    format!("⚡ {} CPS ({}) • Gatilho: {} ({})", p.config.target_cps, mode_str, p.config.trigger_device_name, trigger_str)
}

impl AppModel {
    fn reload_profile_list(&mut self) {
        let mut guard = self.profile_list.guard();
        guard.clear();
        for p in &self.profiles {
            let desc = format_profile_desc(p);
            guard.push_back((p.name.clone(), p.enabled, desc));
        }
    }

    fn update_engine_configs(&self) {
        if self.is_active {
            let active_configs: Vec<AppConfig> = self.profiles
                .iter()
                .filter(|p| p.enabled)
                .map(|p| p.config.clone())
                .collect();
            let _ = self.engine_tx.send(EngineCommand::UpdateActiveConfigs(active_configs));
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
        adw::Window {
            set_title: Some("AutoClicker Humanizado"),
            set_default_width: 550,
            set_default_height: 650,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &gtk::Label {
                        set_markup: "<b>AutoClicker Humanizado</b>",
                    },
                },

                adw::Banner {
                    set_title: "Aviso: Sem permissão para ler dispositivos de entrada (/dev/input). No terminal, rode: 'sudo usermod -aG input $USER' e reinicie sua sessão.",
                    set_button_label: Some("Verificar Novamente"),
                    set_revealed: !model.has_uinput_permission || !model.has_input_permission,
                    connect_button_clicked => AppMsg::CheckPermissions,
                },

                gtk::Stack {
                    set_transition_type: gtk::StackTransitionType::SlideLeftRight,
                    #[watch]
                    set_visible_child_name: match model.view_state {
                        ViewState::Main => "main",
                        ViewState::Wizard => "wizard",
                        ViewState::ConfirmDelete => "confirm",
                    },

                    // PÁGINA PRINCIPAL
                    add_named[Some("main")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_all: 20,
                        set_spacing: 16,

                        adw::PreferencesGroup {
                            set_title: "Perfis Configurados",
                            set_description: Some("Ative os perfis que deseja manter em execução"),
                            #[watch]
                            set_visible: !model.profiles.is_empty(),

                            #[local_ref]
                            profile_list_widget -> gtk::ListBox {
                                set_css_classes: &["boxed-list"],
                                set_selection_mode: gtk::SelectionMode::None,
                            }
                        },

                        gtk::Box {
                            set_vexpand: true,
                            set_valign: gtk::Align::Center,
                            set_halign: gtk::Align::Center,
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 12,

                            gtk::ToggleButton {
                                #[watch]
                                set_label: if model.is_active { "DESATIVAR AUTOCLICKER" } else { "ATIVAR AUTOCLICKER" },
                                #[watch]
                                set_sensitive: !model.profiles.is_empty(),
                                #[watch]
                                set_css_classes: if model.is_active { &["destructive-action", "pill"] } else { &["pill", "suggested-action"] },
                                set_width_request: 250,
                                set_height_request: 60,
                                connect_toggled[sender] => move |btn| {
                                    sender.input(AppMsg::ToggleMaster(btn.is_active()));
                                }
                            }
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 16,
                            set_vexpand: true,
                            set_valign: gtk::Align::Center,
                            #[watch]
                            set_visible: model.profiles.is_empty(),
                            
                            gtk::Label {
                                set_label: "Nenhum Perfil Criado",
                                set_css_classes: &["title-1", "dim-label"],
                            },
                            gtk::Label {
                                set_label: "Clique no botão abaixo para criar a sua primeira macro.",
                                set_css_classes: &["dim-label"],
                            }
                        },

                        gtk::Button {
                            set_label: "Criar Novo Perfil",
                            set_halign: gtk::Align::Center,
                            set_margin_bottom: 10,
                            set_css_classes: &["pill", "suggested-action"],
                            connect_clicked => AppMsg::StartWizard,
                        }
                    },
                    
                    // CONFIRM DELETE
                    add_named[Some("confirm")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::Center,
                        set_spacing: 20,
                        
                        gtk::Label {
                            set_label: "Deseja realmente excluir este perfil?",
                            set_css_classes: &["title-2"],
                        },
                        gtk::Label {
                            #[watch]
                            set_label: &format!("Perfil: {}", model.profiles.get(model.selected_profile_index).map(|p| p.name.as_str()).unwrap_or("")),
                        },
                        gtk::Box {
                            set_halign: gtk::Align::Center,
                            set_spacing: 16,
                            
                            gtk::Button {
                                set_label: "Cancelar",
                                connect_clicked => AppMsg::CancelDelete,
                            },
                            gtk::Button {
                                set_label: "Excluir Definitivamente",
                                set_css_classes: &["destructive-action"],
                                connect_clicked => AppMsg::ConfirmDelete,
                            }
                        }
                    },

                    // WIZARD
                    add_named[Some("wizard")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_all: 20,
                        set_spacing: 20,

                        gtk::Label {
                            set_label: "Assistente de Criação de Macro",
                            set_css_classes: &["title-1"],
                            set_halign: gtk::Align::Center,
                        },

                        gtk::Stack {
                            set_vexpand: true,
                            set_transition_type: gtk::StackTransitionType::Crossfade,
                            #[watch]
                            set_visible_child_name: match model.wizard_step {
                                WizardStep::DetectTargetDevice => "step1",
                                WizardStep::DetectTargetAction => "step2",
                                WizardStep::DetectTrigger => "step3",
                                WizardStep::SelectModeAndSpeed => "step4",
                            },

                            // STEP 1: DETECT TARGET DEVICE
                            add_named[Some("step1")] = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 16,
                                set_valign: gtk::Align::Center,

                                gtk::Label {
                                    set_label: "Passo 1: Qual equipamento vamos usar?",
                                    set_css_classes: &["title-2"],
                                },
                                gtk::Label {
                                    set_label: "Pressione QUALQUER botão do mouse ou teclado que você deseja que o autoclicker simule.",
                                    set_wrap: true,
                                    set_justify: gtk::Justification::Center,
                                },
                                gtk::Spinner {
                                    #[watch]
                                    set_spinning: model.wizard_target_device_name.is_empty(),
                                    set_size_request: (40, 40),
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &{
                                        if model.wizard_target_device_name.is_empty() { 
                                            "Aguardando clique...".to_string()
                                        } else { 
                                            format!("Selecionado: {}", model.wizard_target_device_name) 
                                        }
                                    },
                                    #[watch]
                                    set_css_classes: if model.wizard_target_device_name.is_empty() { &["dim-label"] } else { &["success"] },
                                },
                                gtk::Box {
                                    set_halign: gtk::Align::Center,
                                    set_spacing: 10,
                                    gtk::Button {
                                        set_label: "Tentar de Novo",
                                        #[watch]
                                        set_visible: !model.wizard_target_device_name.is_empty(),
                                        connect_clicked => AppMsg::RetryDetection,
                                    },
                                    gtk::Button {
                                        set_label: "Avançar",
                                        #[watch]
                                        set_sensitive: !model.wizard_target_device_name.is_empty(),
                                        set_css_classes: &["suggested-action", "pill"],
                                        connect_clicked => AppMsg::NextWizardStep,
                                    }
                                }
                            },

                            // STEP 2: DETECT TARGET ACTION
                            add_named[Some("step2")] = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 16,
                                set_valign: gtk::Align::Center,

                                gtk::Label {
                                    set_label: "Passo 2: Qual botão deve ser metralhado?",
                                    set_css_classes: &["title-2"],
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &format!("No equipamento ({}), PRESSIONE a tecla/botão exato que será repetido.", model.wizard_target_device_name),
                                    set_wrap: true,
                                    set_justify: gtk::Justification::Center,
                                },
                                gtk::Spinner {
                                    #[watch]
                                    set_spinning: model.wizard_action_name.is_empty(),
                                    set_size_request: (40, 40),
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &{
                                        if model.wizard_action_name.is_empty() { 
                                            "Aguardando... Aperte o botão a ser replicado.".to_string()
                                        } else { 
                                            format!("Registrado: {}", model.wizard_action_name) 
                                        }
                                    },
                                    #[watch]
                                    set_css_classes: if model.wizard_action_name.is_empty() { &["dim-label"] } else { &["success"] },
                                },
                                gtk::Box {
                                    set_halign: gtk::Align::Center,
                                    set_spacing: 10,
                                    gtk::Button {
                                        set_label: "Voltar",
                                        connect_clicked => AppMsg::PrevWizardStep,
                                    },
                                    gtk::Button {
                                        set_label: "Tentar de Novo",
                                        #[watch]
                                        set_visible: !model.wizard_action_name.is_empty(),
                                        connect_clicked => AppMsg::RetryDetection,
                                    },
                                    gtk::Button {
                                        set_label: "Avançar",
                                        #[watch]
                                        set_sensitive: !model.wizard_action_name.is_empty(),
                                        set_css_classes: &["suggested-action"],
                                        connect_clicked => AppMsg::NextWizardStep,
                                    }
                                }
                            },

                            // STEP 3: DETECT TRIGGER
                            add_named[Some("step3")] = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 16,
                                set_valign: gtk::Align::Center,

                                gtk::Label {
                                    set_label: "Passo 3: Como vamos Ligar/Desligar?",
                                    set_css_classes: &["title-2"],
                                },
                                gtk::Label {
                                    set_label: "Pressione QUALQUER botão em QUALQUER periférico para servir de gatilho.",
                                    set_wrap: true,
                                    set_justify: gtk::Justification::Center,
                                },
                                gtk::Spinner {
                                    #[watch]
                                    set_spinning: model.wizard_trigger_name.is_empty(),
                                    set_size_request: (40, 40),
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &{
                                        if model.wizard_trigger_name.is_empty() { 
                                            "Aguardando... Aperte o gatilho.".to_string()
                                        } else { 
                                            format!("Gatilho Registrado: {}", model.wizard_trigger_name) 
                                        }
                                    },
                                    #[watch]
                                    set_css_classes: if model.wizard_trigger_name.is_empty() { &["dim-label"] } else { &["success"] },
                                },
                                gtk::Box {
                                    set_halign: gtk::Align::Center,
                                    set_spacing: 10,
                                    gtk::Button {
                                        set_label: "Voltar",
                                        connect_clicked => AppMsg::PrevWizardStep,
                                    },
                                    gtk::Button {
                                        set_label: "Tentar de Novo",
                                        #[watch]
                                        set_visible: !model.wizard_trigger_name.is_empty(),
                                        connect_clicked => AppMsg::RetryDetection,
                                    },
                                    gtk::Button {
                                        set_label: "Avançar",
                                        #[watch]
                                        set_sensitive: !model.wizard_trigger_name.is_empty(),
                                        set_css_classes: &["suggested-action"],
                                        connect_clicked => AppMsg::NextWizardStep,
                                    }
                                }
                            },

                            // STEP 4: SPEED AND SAVE
                            add_named[Some("step4")] = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 16,
                                set_valign: gtk::Align::Center,

                                gtk::Label {
                                    set_label: "Passo 4: Velocidade e Salvar",
                                    set_css_classes: &["title-2"],
                                },
                                adw::PreferencesGroup {
                                    adw::ComboRow {
                                        set_title: "Modo",
                                        set_model: Some(&gtk::StringList::new(&[
                                            "Segurar (Hold) - Clica enquanto estiver pressionado",
                                            "Alternar (Toggle) - Pressione para ligar, pressione para desligar",
                                        ])),
                                        set_selected: match model.wizard_config.trigger_type {
                                            TriggerType::Hold => 0,
                                            TriggerType::Toggle => 1,
                                        },
                                        connect_selected_notify[sender] => move |combo| {
                                            let tt = if combo.selected() == 0 { TriggerType::Hold } else { TriggerType::Toggle };
                                            sender.input(AppMsg::WizardSetMode(tt));
                                        }
                                    },
                                    adw::ComboRow {
                                        set_title: "Modelo do Autoclicker",
                                        set_model: Some(&gtk::StringList::new(&[
                                            "Humanizado (Variação natural de tempo)",
                                            "Fixo (Velocidade constante/robótica)",
                                            "Clique Duplo",
                                        ])),
                                        set_selected: match model.wizard_config.click_mode {
                                            crate::config::ClickMode::Humanized => 0,
                                            crate::config::ClickMode::Fixed => 1,
                                            crate::config::ClickMode::DoubleClick => 2,
                                        },
                                        connect_selected_notify[sender] => move |combo| {
                                            let cm = match combo.selected() {
                                                0 => crate::config::ClickMode::Humanized,
                                                1 => crate::config::ClickMode::Fixed,
                                                2 => crate::config::ClickMode::DoubleClick,
                                                _ => crate::config::ClickMode::Humanized,
                                            };
                                            sender.input(AppMsg::WizardSetClickMode(cm));
                                        }
                                    },
                                    adw::ActionRow {
                                        set_title: "Cliques por Segundo (CPS)",
                                        #[watch]
                                        set_subtitle: &format!("{}", model.wizard_config.target_cps),
                                        add_suffix = &gtk::Scale {
                                            set_orientation: gtk::Orientation::Horizontal,
                                            set_range: (1.0, 100.0),
                                            set_value: model.wizard_config.target_cps as f64,
                                            set_hexpand: true,
                                            set_width_request: 200,
                                            connect_value_changed[sender] => move |scale| {
                                                sender.input(AppMsg::WizardSetCps(scale.value() as u32));
                                            }
                                        },
                                    },
                                    adw::EntryRow {
                                        set_title: "Nome do Perfil",
                                        set_text: &model.wizard_profile_name,
                                        connect_changed[sender] => move |entry| {
                                            sender.input(AppMsg::WizardSetName(entry.text().to_string()));
                                        }
                                    }
                                },
                                gtk::Box {
                                    set_halign: gtk::Align::Center,
                                    set_spacing: 10,
                                    gtk::Button {
                                        set_label: "Voltar",
                                        connect_clicked => AppMsg::PrevWizardStep,
                                    },
                                    gtk::Button {
                                        set_label: "Finalizar e Salvar",
                                        set_css_classes: &["suggested-action"],
                                        connect_clicked => AppMsg::FinishWizard,
                                    }
                                }
                            },
                        },
                        
                        gtk::Button {
                            set_label: "Cancelar Criação",
                            set_halign: gtk::Align::Center,
                            set_css_classes: &["destructive-action", "flat"],
                            connect_clicked => AppMsg::CancelWizard,
                        }
                    }
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let (tx, rx) = channel();
        
        AutoClickerEngine::spawn(rx, sender.input_sender().clone());

        let profile_manager = ProfileManager::new();
        let profiles = profile_manager.list_profiles();
        let has_uinput_permission = check_uinput_permission();
        let has_input_permission = check_input_permission();
        let profile_strings = gtk::StringList::new(&profiles.iter().map(|p| p.name.as_str()).collect::<Vec<&str>>());

        let mut profile_list = relm4::factory::FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), |output| output);

        {
            let mut guard = profile_list.guard();
            for p in &profiles {
                let desc = format_profile_desc(p);
                guard.push_back((p.name.clone(), p.enabled, desc));
            }
        }

        let model = AppModel {
            is_active: false,
            profiles,
            selected_profile_index: 0,
            has_uinput_permission,
            has_input_permission,
            
            view_state: ViewState::Main,
            wizard_step: WizardStep::DetectTargetDevice,
            wizard_config: AppConfig::default(),
            wizard_target_device_name: String::new(),
            wizard_action_name: String::new(),
            wizard_trigger_name: String::new(),
            wizard_profile_name: "Meu Perfil".to_string(),
            
            profile_strings,
            engine_tx: tx,
            profile_manager,
            profile_list,
        };

        let profile_list_widget = model.profile_list.widget();

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::ToggleMaster(active) => {
                println!("TOGGLE MASTER CALLED: {}", active);
                self.is_active = active;
                let _ = self.engine_tx.send(EngineCommand::SetMasterState(active));
                if active {
                    self.update_engine_configs();
                }
            }
            AppMsg::StartWizard => {
                self.view_state = ViewState::Wizard;
                self.wizard_step = WizardStep::DetectTargetDevice;
                self.wizard_config = AppConfig::default();
                self.wizard_target_device_name.clear();
                self.wizard_action_name.clear();
                self.wizard_trigger_name.clear();
                
                // Start listening for the target device globally
                let _ = self.engine_tx.send(EngineCommand::StartListeningGlobal);
            }
            AppMsg::CancelWizard => {
                self.view_state = ViewState::Main;
                let _ = self.engine_tx.send(EngineCommand::StopListening);
            }
            AppMsg::NextWizardStep => {
                match self.wizard_step {
                    WizardStep::DetectTargetDevice => {
                        self.wizard_step = WizardStep::DetectTargetAction;
                        // Listen ONLY on the selected device for the action key
                        if let Some(path) = &self.wizard_config.target_device_path {
                            let _ = self.engine_tx.send(EngineCommand::StartListeningSpecific(path.clone()));
                        }
                    }
                    WizardStep::DetectTargetAction => {
                        self.wizard_step = WizardStep::DetectTrigger;
                        // Listen globally again for the trigger
                        let _ = self.engine_tx.send(EngineCommand::StartListeningGlobal);
                    }
                    WizardStep::DetectTrigger => {
                        self.wizard_step = WizardStep::SelectModeAndSpeed;
                    }
                    WizardStep::SelectModeAndSpeed => {}
                }
            }
            AppMsg::PrevWizardStep => {
                match self.wizard_step {
                    WizardStep::DetectTargetDevice => {}
                    WizardStep::DetectTargetAction => {
                        self.wizard_step = WizardStep::DetectTargetDevice;
                        let _ = self.engine_tx.send(EngineCommand::StartListeningGlobal);
                    }
                    WizardStep::DetectTrigger => {
                        self.wizard_step = WizardStep::DetectTargetAction;
                        if let Some(path) = &self.wizard_config.target_device_path {
                            let _ = self.engine_tx.send(EngineCommand::StartListeningSpecific(path.clone()));
                        }
                    }
                    WizardStep::SelectModeAndSpeed => {
                        self.wizard_step = WizardStep::DetectTrigger;
                        let _ = self.engine_tx.send(EngineCommand::StartListeningGlobal);
                    }
                }
            }
            AppMsg::RetryDetection => {
                match self.wizard_step {
                    WizardStep::DetectTargetDevice => {
                        self.wizard_target_device_name.clear();
                        self.wizard_config.target_device_path = None;
                        let _ = self.engine_tx.send(EngineCommand::StartListeningGlobal);
                    }
                    WizardStep::DetectTargetAction => {
                        self.wizard_action_name.clear();
                        if let Some(path) = &self.wizard_config.target_device_path {
                            let _ = self.engine_tx.send(EngineCommand::StartListeningSpecific(path.clone()));
                        }
                    }
                    WizardStep::DetectTrigger => {
                        self.wizard_trigger_name.clear();
                        self.wizard_config.trigger_device_path = None;
                        let _ = self.engine_tx.send(EngineCommand::StartListeningGlobal);
                    }
                    _ => {}
                }
            }
            AppMsg::FinishWizard => {
                let new_profile = Profile {
                    name: self.wizard_profile_name.clone(),
                    enabled: true,
                    config: self.wizard_config.clone(),
                };
                let _ = self.profile_manager.save_profile(&new_profile);
                self.profiles = self.profile_manager.list_profiles();
                
                self.profile_strings.append(&new_profile.name);
                self.reload_profile_list();
                self.update_engine_configs();
                
                self.view_state = ViewState::Main;
            }
            AppMsg::AskDeleteProfile(index) => {
                self.selected_profile_index = index;
                self.view_state = ViewState::ConfirmDelete;
            }
            AppMsg::CancelDelete => {
                self.view_state = ViewState::Main;
            }
            AppMsg::ConfirmDelete => {
                if !self.profiles.is_empty() {
                    let profile = &self.profiles[self.selected_profile_index];
                    let _ = self.profile_manager.delete_profile(&profile.name);
                    
                    self.profiles = self.profile_manager.list_profiles();
                    self.profile_strings.remove(self.selected_profile_index as u32);
                    
                    self.reload_profile_list();
                    self.update_engine_configs();
                }
                self.view_state = ViewState::Main;
            }
            AppMsg::WizardSetMode(tm) => {
                self.wizard_config.trigger_type = tm;
            }
            AppMsg::WizardSetClickMode(cm) => {
                self.wizard_config.click_mode = cm;
            }
            AppMsg::WizardSetCps(cps) => {
                self.wizard_config.target_cps = cps;
            }
            AppMsg::WizardSetName(name) => {
                self.wizard_profile_name = name;
            }
            AppMsg::ToggleProfile(idx, enabled) => {
                if let Some(profile) = self.profiles.get_mut(idx) {
                    profile.enabled = enabled;
                    let _ = self.profile_manager.save_profile(profile);
                    self.update_engine_configs();
                }
            }
            AppMsg::CheckPermissions => {
                self.has_uinput_permission = check_uinput_permission();
                self.has_input_permission = check_input_permission();
            }
            AppMsg::EngineEvent(event) => {
                match event {
                    EngineEvent::EventDetected { key_code, device_name, device_path } => {
                        match self.wizard_step {
                            WizardStep::DetectTargetDevice => {
                                self.wizard_config.target_device_path = Some(device_path);
                                self.wizard_target_device_name = device_name.clone();
                                self.wizard_config.target_device_name = device_name;
                            }
                            WizardStep::DetectTargetAction => {
                                self.wizard_config.target_action_code = key_code;
                                self.wizard_action_name = format!("Código da Tecla/Botão: {}", key_code);
                            }
                            WizardStep::DetectTrigger => {
                                self.wizard_config.trigger_device_path = Some(device_path);
                                self.wizard_config.trigger_code = key_code;
                                self.wizard_trigger_name = format!("{} (Código {})", device_name.clone(), key_code);
                                self.wizard_config.trigger_device_name = device_name;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum ProfileRowMsg {
    Toggle(bool),
    Delete,
}

pub struct ProfileRow {
    index: relm4::factory::DynamicIndex,
    name: String,
    enabled: bool,
    description: String,
}

#[relm4::factory(pub)]
impl relm4::factory::FactoryComponent for ProfileRow {
    type Init = (String, bool, String);
    type Input = ProfileRowMsg;
    type Output = AppMsg;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            set_title: &self.name,
            #[watch]
            set_subtitle: &self.description,
            set_activatable: false,

            add_prefix = &gtk::Image {
                set_icon_name: Some("input-gaming-symbolic"),
                set_pixel_size: 20,
                set_css_classes: &["dim-label"],
            },

            add_suffix = &gtk::Switch {
                #[watch]
                set_active: self.enabled,
                set_valign: gtk::Align::Center,
                connect_active_notify[sender] => move |switch| {
                    sender.input(ProfileRowMsg::Toggle(switch.is_active()));
                }
            },

            add_suffix = &gtk::Button {
                set_icon_name: "user-trash-symbolic",
                set_css_classes: &["flat", "error", "circular"],
                set_valign: gtk::Align::Center,
                set_tooltip_text: Some("Excluir Perfil"),
                connect_clicked[sender] => move |_| {
                    sender.input(ProfileRowMsg::Delete);
                }
            }
        }
    }

    fn init_model(
        init: Self::Init,
        index: &relm4::factory::DynamicIndex,
        _sender: relm4::factory::FactorySender<Self>,
    ) -> Self {
        Self {
            index: index.clone(),
            name: init.0,
            enabled: init.1,
            description: init.2,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::factory::FactorySender<Self>) {
        match msg {
            ProfileRowMsg::Toggle(val) => {
                self.enabled = val;
                let _ = sender.output(AppMsg::ToggleProfile(self.index.current_index(), val));
            }
            ProfileRowMsg::Delete => {
                let _ = sender.output(AppMsg::AskDeleteProfile(self.index.current_index()));
            }
        }
    }
}
