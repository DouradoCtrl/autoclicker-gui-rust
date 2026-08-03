use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    pub config: AppConfig,
}

pub struct ProfileManager {
    config_dir: PathBuf,
}

impl ProfileManager {
    pub fn new() -> Self {
        let mut config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
        config_dir.push("humanized-autoclicker");
        config_dir.push("profiles");

        let _ = fs::create_dir_all(&config_dir);

        Self { config_dir }
    }

    pub fn list_profiles(&self) -> Vec<Profile> {
        let mut profiles = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.config_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(profile) = serde_json::from_str::<Profile>(&content) {
                            profiles.push(profile);
                        }
                    }
                }
            }
        }
        profiles
    }

    pub fn save_profile(&self, profile: &Profile) -> Result<(), String> {
        let mut path = self.config_dir.clone();
        let filename = format!("{}.json", profile.name.to_lowercase().replace(' ', "_"));
        path.push(filename);

        let json = serde_json::to_string_pretty(profile).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }

    #[allow(dead_code)]
    pub fn delete_profile(&self, name: &str) -> Result<(), String> {
        let mut path = self.config_dir.clone();
        let filename = format!("{}.json", name.to_lowercase().replace(' ', "_"));
        path.push(filename);

        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }
}
