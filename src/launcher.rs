use std::process::Command;
use winreg::enums::*;
use winreg::RegKey;
use std::path::PathBuf;
use std::fs;

pub struct ValorantLauncher;

impl ValorantLauncher {
    pub fn launch() -> Result<(), String> {
        if let Some(riot_client_path) = Self::find_riot_client() {
            println!("Found Riot Client at: {}", riot_client_path);
            return Self::launch_via_riot_client(&riot_client_path);
        }

        if let Some(shortcut_target) = Self::find_via_shortcut() {
            println!("Found via shortcut: {}", shortcut_target);
            return Self::launch_via_riot_client(&shortcut_target);
        }

        Err("Riot Client not found. Tried:\n\
             1. Registry: HKLM\\SOFTWARE\\Riot Games\\Riot Client\n\
             2. Registry: HKLM\\SOFTWARE\\WOW6432Node\\Riot Games\\Riot Client\n\
             3. Start Menu shortcuts\n\
             4. Default paths\n\
             Please make sure Valorant is installed.".to_string())
    }

    fn find_riot_client() -> Option<String> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        let registry_paths = vec![
            r"SOFTWARE\Riot Games\Riot Client",
            r"SOFTWARE\WOW6432Node\Riot Games\Riot Client",
        ];

        for path in registry_paths {
            if let Ok(key) = hklm.open_subkey(path) {
                if let Ok(install_folder) = key.get_value::<String, _>("InstallFolder") {
                    let riot_client_path = PathBuf::from(&install_folder)
                        .join("RiotClientServices.exe");

                    if riot_client_path.exists() {
                        return Some(riot_client_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        let default_paths = vec![
            r"C:\Riot Games\Riot Client\RiotClientServices.exe",
            r"C:\Program Files\Riot Games\Riot Client\RiotClientServices.exe",
            r"C:\Program Files (x86)\Riot Games\Riot Client\RiotClientServices.exe",
        ];

        for path in default_paths {
            if std::path::Path::new(path).exists() {
                return Some(path.to_string());
            }
        }

        None
    }

    fn find_via_shortcut() -> Option<String> {
        let program_data = std::env::var("PROGRAMDATA").ok()?;
        let appdata = std::env::var("APPDATA").ok()?;

        let shortcut_locations = vec![
            PathBuf::from(program_data).join(r"Microsoft\Windows\Start Menu\Programs\Riot Games"),
            PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs\Riot Games"),
        ];

        for location in shortcut_locations {
            if let Some(target) = Self::search_shortcuts_in_folder(&location) {
                return Some(target);
            }
        }

        None
    }

    fn search_shortcuts_in_folder(folder: &PathBuf) -> Option<String> {
        if !folder.exists() {
            return None;
        }

        fn walk_dir_recursive(dir: &PathBuf) -> Option<String> {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();

                    if path.is_file() {
                        if let Some(filename) = path.file_name() {
                            let filename_str = filename.to_string_lossy().to_lowercase();
                            if filename_str.ends_with(".lnk") && filename_str.contains("valorant") {
                                if let Some(target) = ValorantLauncher::get_shortcut_target(&path) {
                                    return Some(target);
                                }
                            }
                        }
                    } else if path.is_dir() {
                        if let Some(target) = walk_dir_recursive(&path) {
                            return Some(target);
                        }
                    }
                }
            }
            None
        }

        walk_dir_recursive(folder)
    }

    fn get_shortcut_target(shortcut_path: &PathBuf) -> Option<String> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let output = Command::new("powershell")
            .creation_flags(CREATE_NO_WINDOW)
            .args(&[
                "-NoProfile",
                "-Command",
                &format!(
                    "$ws = New-Object -ComObject WScript.Shell; $s = $ws.CreateShortcut('{}'); $s.TargetPath",
                    shortcut_path.to_string_lossy()
                ),
            ])
            .output()
            .ok()?;

        if output.status.success() {
            let target = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !target.is_empty() && std::path::Path::new(&target).exists() {
                return Some(target);
            }
        }

        None
    }

    fn launch_via_riot_client(path: &str) -> Result<(), String> {
        Command::new(path)
            .args(&["--launch-product=valorant", "--launch-patchline=live"])
            .spawn()
            .map_err(|e| format!("Failed to launch Valorant: {}", e))?;

        std::thread::sleep(std::time::Duration::from_secs(2));

        Ok(())
    }
}