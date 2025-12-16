use auto_launch::AutoLaunch;
use std::env;

pub struct StartupManager {
    auto_launch: AutoLaunch,
}

impl StartupManager {
    pub fn new() -> Self {
        let exe_path = env::current_exe().unwrap_or_default();
        let auto_launch = AutoLaunch::new(
            "WideVal",
            &exe_path.to_string_lossy(),
            &[] as &[&str],
        );

        Self { auto_launch }
    }

    pub fn is_enabled(&self) -> bool {
        self.auto_launch.is_enabled().unwrap_or(false)
    }

    pub fn enable(&self) -> Result<(), String> {
        self.auto_launch
            .enable()
            .map_err(|e| format!("Failed to enable startup: {}", e))
    }

    pub fn disable(&self) -> Result<(), String> {
        self.auto_launch
            .disable()
            .map_err(|e| format!("Failed to disable startup: {}", e))
    }
}
