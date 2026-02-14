use std::sync::{Arc, Mutex};

#[derive(PartialEq, Copy, Clone)]
pub enum AppState {
    Idle,
    WaitingForValorant,
    Running,
}

#[derive(PartialEq)]
pub enum Tab {
    Main,
    Presets,
    Settings,
}

pub type SharedState = Arc<Mutex<AppState>>;
pub type SharedString = Arc<Mutex<String>>;
pub type SharedBool = Arc<Mutex<bool>>;
pub type SharedConsoleOutput = Arc<Mutex<Vec<String>>>;
