use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub target_width: u32,
    pub target_height: u32,
    pub auto_launch: bool,
    pub apply_to_all_accounts: bool,
    pub selected_account: Option<String>,
    #[serde(default = "default_restore_after_close")]
    pub restore_after_close: bool,
}

fn default_restore_after_close() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            target_width: 1280,
            target_height: 1024,
            auto_launch: false,
            apply_to_all_accounts: true,
            selected_account: None,
            restore_after_close: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub apply_to_all_accounts: bool,
    #[serde(default = "default_restore_after_close")]
    pub restore_after_close: bool,
}

#[derive(Clone)]
pub struct ConfigManager {
    config_path: PathBuf,
    presets_path: PathBuf,
    valorant_config_base: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let config_path = PathBuf::from(&app_data).join("WideVal").join("config.json");

        let presets_path = PathBuf::from(&app_data).join("WideVal").join("presets");

        let valorant_config_base = PathBuf::from(&app_data).join("VALORANT\\Saved\\Config");

        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let _ = fs::create_dir_all(&presets_path);

        Self {
            config_path,
            presets_path,
            valorant_config_base,
        }
    }

    pub fn load(&self) -> Config {
        if let Ok(content) = fs::read_to_string(&self.config_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Config::default()
        }
    }

    pub fn save(&self, config: &Config) -> io::Result<()> {
        let json = serde_json::to_string_pretty(config)?;
        fs::write(&self.config_path, json)
    }

    pub fn find_valorant_configs(&self) -> Vec<PathBuf> {
        let mut configs = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.valorant_config_base) {
            for entry in entries.flatten() {
                let path = entry.path();

                if !path.is_dir() {
                    continue;
                }

                let folder_name = entry.file_name().to_string_lossy().to_string();

                if folder_name == "WindowsClient"
                    || folder_name == "CrashReportClient"
                    || folder_name.starts_with("989e4975")
                {
                    continue;
                }

                let config_file = path.join("WindowsClient").join("GameUserSettings.ini");

                if config_file.exists() {
                    configs.push(config_file);
                }
            }
        }

        configs
    }

    pub fn find_valorant_accounts(&self) -> Vec<(String, PathBuf)> {
        let mut accounts = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.valorant_config_base) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Only process directories
                if !path.is_dir() {
                    continue;
                }

                let folder_name = entry.file_name().to_string_lossy().to_string();

                // Skip known non-account folders
                if folder_name == "WindowsClient"
                    || folder_name == "CrashReportClient"
                    || folder_name.starts_with("989e4975")
                {
                    continue;
                }

                let config_file = path.join("WindowsClient").join("GameUserSettings.ini");

                if config_file.exists() {
                    accounts.push((folder_name, config_file));
                }
            }
        }

        accounts
    }

    pub fn list_presets(&self) -> Vec<String> {
        let mut presets = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.presets_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".json") {
                        presets.push(name.trim_end_matches(".json").to_string());
                    }
                }
            }
        }

        presets
    }

    pub fn save_preset(&self, preset: &Preset) -> io::Result<()> {
        let preset_path = self.presets_path.join(format!("{}.json", preset.name));
        let json = serde_json::to_string_pretty(preset)?;
        fs::write(preset_path, json)
    }

    pub fn load_preset(&self, name: &str) -> io::Result<Preset> {
        let preset_path = self.presets_path.join(format!("{}.json", name));
        let content = fs::read_to_string(preset_path)?;
        let preset: Preset = serde_json::from_str(&content)?;
        Ok(preset)
    }

    pub fn delete_preset(&self, name: &str) -> io::Result<()> {
        let preset_path = self.presets_path.join(format!("{}.json", name));
        fs::remove_file(preset_path)
    }

    pub fn modify_valorant_config(
        &self,
        config_path: &Path,
        width: u32,
        height: u32,
    ) -> io::Result<()> {
        Self::set_readonly(config_path, false)?;

        let content = fs::read_to_string(config_path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        lines.retain(|line| !line.starts_with("FullscreenMode="));

        for i in 0..lines.len() {
            let line = &lines[i];

            if line.starts_with("ResolutionSizeX=") {
                lines[i] = format!("ResolutionSizeX={}", width);
            } else if line.starts_with("ResolutionSizeY=") {
                lines[i] = format!("ResolutionSizeY={}", height);
            }
        }

        let mut insert_position = None;
        let mut in_shooter_section = false;

        for i in 0..lines.len() {
            if lines[i].starts_with("[/Script/ShooterGame.ShooterGameUserSettings]") {
                in_shooter_section = true;
            } else if in_shooter_section && lines[i].starts_with("[") {
                if i > 0 && lines[i - 1].is_empty() {
                    insert_position = Some(i - 1);
                } else {
                    insert_position = Some(i);
                }
                break;
            }
        }

        if let Some(pos) = insert_position {
            lines.insert(pos, "FullscreenMode=2".to_string());
        } else {
            lines.push("".to_string());
            lines.push("FullscreenMode=2".to_string());
        }

        let mut file = fs::File::create(config_path)?;
        for line in lines {
            writeln!(file, "{}", line)?;
        }

        Self::set_readonly(config_path, true)?;

        Ok(())
    }

    pub fn restore_valorant_config(&self, config_path: &Path) -> io::Result<()> {
        Self::set_readonly(config_path, false)?;

        let content = fs::read_to_string(config_path)?;
        let lines: Vec<String> = content
            .lines()
            .filter(|line| !line.starts_with("FullscreenMode="))
            .map(|s| s.to_string())
            .collect();

        let mut file = fs::File::create(config_path)?;
        for line in lines {
            writeln!(file, "{}", line)?;
        }

        Self::set_readonly(config_path, true)?;

        Ok(())
    }

    fn set_readonly(path: &Path, readonly: bool) -> io::Result<()> {
        let metadata = fs::metadata(path)?;
        let mut permissions = metadata.permissions();
        permissions.set_readonly(readonly);
        fs::set_permissions(path, permissions)
    }
}