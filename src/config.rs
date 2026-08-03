use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClickMode {
    Humanized,
    Fixed,
    DoubleClick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerType {
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub target_device_path: Option<String>,
    #[serde(default)]
    pub target_device_name: String,
    pub target_action_code: u16, 
    
    pub trigger_device_path: Option<String>,
    #[serde(default)]
    pub trigger_device_name: String,
    pub trigger_code: u16, 
    
    pub click_mode: ClickMode,
    pub trigger_type: TriggerType,
    pub target_cps: u32,
    pub double_click_delay_ms: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            target_device_path: None,
            target_device_name: "Qualquer".to_string(),
            target_action_code: 272, // BTN_LEFT default
            
            trigger_device_path: None,
            trigger_device_name: "Qualquer".to_string(),
            trigger_code: 275, // BTN_SIDE / M4 default
            
            click_mode: ClickMode::Humanized,
            trigger_type: TriggerType::Hold,
            target_cps: 10,
            double_click_delay_ms: 30,
        }
    }
}
