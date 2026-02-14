use crate::config::{Config, ConfigManager, Preset};
use crate::launcher::ValorantLauncher;
use crate::process::ProcessManager;
use crate::resolution::{Resolution, ResolutionManager};
use crate::startup::StartupManager;
use crate::types::{AppState, SharedBool, SharedConsoleOutput, SharedState, SharedString, Tab};
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct WideValApp {
    config: Config,
    config_manager: ConfigManager,
    startup_manager: StartupManager,
    state: SharedState,
    original_resolution: Option<Resolution>,
    available_resolutions: Vec<Resolution>,
    selected_resolution_index: usize,
    status_message: SharedString,
    presets_status_message: String,
    settings_status_message: String,
    current_tab: Tab,
    custom_width: String,
    custom_height: String,
    use_custom_resolution: bool,
    valorant_accounts: Vec<(String, std::path::PathBuf)>,
    selected_account_index: usize,
    presets: Vec<String>,
    new_preset_name: String,
    show_console: SharedBool,
    console_output: SharedConsoleOutput,
}

impl WideValApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config_manager = ConfigManager::new();
        let config = config_manager.load();
        let startup_manager = StartupManager::new();

        let available_resolutions = ResolutionManager::list_available();
        let selected_resolution_index = available_resolutions
            .iter()
            .position(|r| r.width == config.target_width && r.height == config.target_height)
            .unwrap_or(0);

        let valorant_accounts = config_manager.find_valorant_accounts();
        let presets = config_manager.list_presets();

        Self {
            config,
            config_manager,
            startup_manager,
            state: Arc::new(Mutex::new(AppState::Idle)),
            original_resolution: None,
            available_resolutions,
            selected_resolution_index,
            status_message: Arc::new(Mutex::new(String::from(
                "Ready. Configure settings and launch Valorant",
            ))),
            presets_status_message: String::new(),
            settings_status_message: String::new(),
            current_tab: Tab::Main,
            custom_width: String::from("1280"),
            custom_height: String::from("1024"),
            use_custom_resolution: false,
            valorant_accounts,
            selected_account_index: 0,
            presets,
            new_preset_name: String::new(),
            show_console: Arc::new(Mutex::new(false)),
            console_output: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn log(&self, message: String) {
        let mut output = self.console_output.lock().unwrap();
        output.push(format!(
            "[{}] {}",
            chrono::Local::now().format("%H:%M:%S"),
            message
        ));
        if output.len() > 100 {
            output.remove(0);
        }
    }

    fn start_valorant(&mut self) {
        let target_res = if self.use_custom_resolution {
            let width: u32 = self.custom_width.parse().unwrap_or(1280);
            let height: u32 = self.custom_height.parse().unwrap_or(1024);
            Resolution::new(width, height)
        } else {
            self.available_resolutions[self.selected_resolution_index]
        };

        self.log(format!(
            "Starting with resolution {}x{}",
            target_res.width, target_res.height
        ));
        self.original_resolution = ResolutionManager::get_current();

        if self.config.apply_to_all_accounts {
            let configs = self.config_manager.find_valorant_configs();
            self.log(format!("Modifying {} account configs", configs.len()));
            for config_path in &configs {
                let _ = self.config_manager.modify_valorant_config(
                    config_path,
                    target_res.width,
                    target_res.height,
                );
            }
        } else if self.selected_account_index < self.valorant_accounts.len() {
            let account_name = &self.valorant_accounts[self.selected_account_index].0;
            self.log(format!("Modifying config for account: {}", account_name));
            let config_path = &self.valorant_accounts[self.selected_account_index].1;
            let _ = self.config_manager.modify_valorant_config(
                config_path,
                target_res.width,
                target_res.height,
            );
        }

        self.log("Launching Valorant...".to_string());
        if let Err(e) = ValorantLauncher::launch() {
            *self.status_message.lock().unwrap() = format!("Failed to launch Valorant: {}", e);
            self.log(format!("Launch failed: {}", e));
            return;
        }

        *self.status_message.lock().unwrap() = "Valorant is launching...".to_string();
        self.log("Waiting for Valorant process...".to_string());

        let state = Arc::clone(&self.state);
        let status_message = Arc::clone(&self.status_message);
        let original_res = self.original_resolution;
        let config_manager = self.config_manager.clone();
        let apply_to_all = self.config.apply_to_all_accounts;
        let account_path =
            if !apply_to_all && self.selected_account_index < self.valorant_accounts.len() {
                Some(
                    self.valorant_accounts[self.selected_account_index]
                        .1
                        .clone(),
                )
            } else {
                None
            };
        let console_output = Arc::clone(&self.console_output);

        thread::spawn(move || {
            *state.lock().unwrap() = AppState::WaitingForValorant;

            ProcessManager::wait_for_valorant_start();

            {
                let mut output = console_output.lock().unwrap();
                output.push(format!(
                    "[{}] Valorant detected! Changing resolution...",
                    chrono::Local::now().format("%H:%M:%S")
                ));
            }

            if !ResolutionManager::set_resolution(target_res) {
                let mut output = console_output.lock().unwrap();
                output.push(format!(
                    "[{}] Failed to change resolution",
                    chrono::Local::now().format("%H:%M:%S")
                ));
            } else {
                let mut output = console_output.lock().unwrap();
                output.push(format!(
                    "[{}] Resolution changed to {}x{}",
                    chrono::Local::now().format("%H:%M:%S"),
                    target_res.width,
                    target_res.height
                ));
            }

            *state.lock().unwrap() = AppState::Running;

            ProcessManager::wait_for_valorant_exit();

            {
                let mut output = console_output.lock().unwrap();
                output.push(format!(
                    "[{}] Valorant closed. Restoring settings...",
                    chrono::Local::now().format("%H:%M:%S")
                ));
            }

            if let Some(original) = original_res {
                ResolutionManager::set_resolution(original);
                let mut output = console_output.lock().unwrap();
                output.push(format!(
                    "[{}] Resolution restored to {}x{}",
                    chrono::Local::now().format("%H:%M:%S"),
                    original.width,
                    original.height
                ));
            }

            if apply_to_all {
                let configs = config_manager.find_valorant_configs();
                for config_path in &configs {
                    let _ = config_manager.restore_valorant_config(&config_path);
                }
            } else if let Some(path) = account_path {
                let _ = config_manager.restore_valorant_config(&path);
            }

            {
                let mut output = console_output.lock().unwrap();
                output.push(format!(
                    "[{}] Settings restored. Ready for next launch.",
                    chrono::Local::now().format("%H:%M:%S")
                ));
            }

            *state.lock().unwrap() = AppState::Idle;
            *status_message.lock().unwrap() =
                "Ready. Configure settings and launch Valorant".to_string();
        });
    }

    fn save_current_as_preset(&mut self) {
        if self.new_preset_name.is_empty() {
            self.presets_status_message = "Please enter a preset name".to_string();
            return;
        }

        let preset = Preset {
            name: self.new_preset_name.clone(),
            resolution_width: if self.use_custom_resolution {
                self.custom_width.parse().unwrap_or(1280)
            } else {
                self.available_resolutions[self.selected_resolution_index].width
            },
            resolution_height: if self.use_custom_resolution {
                self.custom_height.parse().unwrap_or(1024)
            } else {
                self.available_resolutions[self.selected_resolution_index].height
            },
            apply_to_all_accounts: self.config.apply_to_all_accounts,
        };

        match self.config_manager.save_preset(&preset) {
            Ok(_) => {
                self.presets_status_message =
                    format!("Preset '{}' saved successfully!", preset.name);
                self.presets = self.config_manager.list_presets();
                self.new_preset_name.clear();
            }
            Err(e) => {
                self.presets_status_message = format!("Failed to save preset: {}", e);
            }
        }
    }

    fn load_preset(&mut self, index: usize) {
        if index >= self.presets.len() {
            return;
        }

        let preset_name = &self.presets[index];
        match self.config_manager.load_preset(preset_name) {
            Ok(preset) => {
                if let Some(idx) = self.available_resolutions.iter().position(|r| {
                    r.width == preset.resolution_width && r.height == preset.resolution_height
                }) {
                    self.use_custom_resolution = false;
                    self.selected_resolution_index = idx;
                } else {
                    self.use_custom_resolution = true;
                    self.custom_width = preset.resolution_width.to_string();
                    self.custom_height = preset.resolution_height.to_string();
                }

                self.config.apply_to_all_accounts = preset.apply_to_all_accounts;

                self.presets_status_message = format!("Preset '{}' loaded!", preset_name);
                self.current_tab = Tab::Main;
            }
            Err(e) => {
                self.presets_status_message = format!("Failed to load preset: {}", e);
            }
        }
    }

    fn delete_preset(&mut self, index: usize) {
        if index >= self.presets.len() {
            return;
        }

        let preset_name = &self.presets[index];
        match self.config_manager.delete_preset(preset_name) {
            Ok(_) => {
                self.presets_status_message = format!("Preset '{}' deleted", preset_name);
                self.presets = self.config_manager.list_presets();
            }
            Err(e) => {
                self.presets_status_message = format!("Failed to delete preset: {}", e);
            }
        }
    }
}

impl eframe::App for WideValApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let current_state = *self.state.lock().unwrap();

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Main, "Main");
                ui.selectable_value(&mut self.current_tab, Tab::Presets, "Presets");
                ui.selectable_value(&mut self.current_tab, Tab::Settings, "Settings");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("📋 Console").clicked() {
                        let mut show = self.show_console.lock().unwrap();
                        *show = !*show;
                    }
                });
            });
            ui.add_space(1.0);
        });

        if *self.show_console.lock().unwrap() {
            let console_output = Arc::clone(&self.console_output);
            let show_console = Arc::clone(&self.show_console);

            ctx.show_viewport_deferred(
                egui::ViewportId::from_hash_of("console_window"),
                egui::ViewportBuilder::default()
                    .with_title("Console Output")
                    .with_inner_size([600.0, 400.0])
                    .with_active(true),
                move |ctx, class| {
                    assert!(class == egui::ViewportClass::Deferred, "Unknown error idk");

                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Clear").clicked() {
                                console_output.lock().unwrap().clear();
                            }
                            ui.label(format!(
                                "{} log entries",
                                console_output.lock().unwrap().len()
                            ));
                        });

                        ui.separator();

                        egui::ScrollArea::vertical()
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                let output = console_output.lock().unwrap();
                                if output.is_empty() {
                                    ui.label("No console output yet");
                                } else {
                                    for line in output.iter() {
                                        ui.label(line);
                                    }
                                }
                            });
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        *show_console.lock().unwrap() = false;
                    }
                },
            );
        } else {
            ctx.send_viewport_cmd_to(
                egui::ViewportId::from_hash_of("console_window"),
                egui::ViewportCommand::Close,
            );
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("WideVal");
            ui.add_space(10.0);

            match self.current_tab {
                Tab::Main => self.show_main_tab(ui, current_state),
                Tab::Presets => self.show_presets_tab(ui),
                Tab::Settings => self.show_settings_tab(ui),
            }
        });

        ctx.request_repaint();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let current_state = *self.state.lock().unwrap();

        if current_state == AppState::Running {
            if let Some(original) = self.original_resolution {
                ResolutionManager::set_resolution(original);
            }

            if self.config.apply_to_all_accounts {
                let configs = self.config_manager.find_valorant_configs();
                for config_path in &configs {
                    let _ = self.config_manager.restore_valorant_config(&config_path);
                }
            }
        }
    }
}

impl WideValApp {
    fn show_main_tab(&mut self, ui: &mut egui::Ui, current_state: AppState) {
        ui.group(|ui| {
            ui.label("Resolution:");

            ui.radio_value(&mut self.use_custom_resolution, false, "Preset resolution");
            if !self.use_custom_resolution {
                egui::ComboBox::from_id_salt("resolution_combo")
                    .selected_text(format!(
                        "{}x{}",
                        self.available_resolutions[self.selected_resolution_index].width,
                        self.available_resolutions[self.selected_resolution_index].height
                    ))
                    .show_ui(ui, |ui| {
                        for (i, res) in self.available_resolutions.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.selected_resolution_index,
                                i,
                                format!("{}x{}", res.width, res.height),
                            );
                        }
                    });
            }

            ui.radio_value(&mut self.use_custom_resolution, true, "Custom resolution");
            if self.use_custom_resolution {
                ui.horizontal(|ui| {
                    ui.label("Width:");
                    ui.add(egui::TextEdit::singleline(&mut self.custom_width).desired_width(80.0));
                    ui.label("Height:");
                    ui.add(egui::TextEdit::singleline(&mut self.custom_height).desired_width(80.0));
                });
            }
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label("Valorant Accounts:");
            ui.radio_value(
                &mut self.config.apply_to_all_accounts,
                true,
                "Apply to all accounts",
            );

            ui.radio_value(
                &mut self.config.apply_to_all_accounts,
                false,
                "Specific account",
            );
            if !self.config.apply_to_all_accounts && !self.valorant_accounts.is_empty() {
                egui::ComboBox::from_id_salt("account_combo")
                    .selected_text(&self.valorant_accounts[self.selected_account_index].0)
                    .show_ui(ui, |ui| {
                        for (i, (name, _)) in self.valorant_accounts.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_account_index, i, name);
                        }
                    });
            } else if !self.config.apply_to_all_accounts {
                ui.label("No Valorant accounts found");
            }
        });

        ui.add_space(20.0);

        let button_enabled = current_state == AppState::Idle;

        if ui
            .add_enabled(
                button_enabled,
                egui::Button::new("Launch Valorant with Stretch"),
            )
            .clicked()
        {
            let target_res = if self.use_custom_resolution {
                let width = self.custom_width.parse().unwrap_or(1280);
                let height = self.custom_height.parse().unwrap_or(1024);
                Resolution::new(width, height)
            } else {
                self.available_resolutions[self.selected_resolution_index]
            };

            self.config.target_width = target_res.width;
            self.config.target_height = target_res.height;
            let _ = self.config_manager.save(&self.config);

            self.start_valorant();
        }

        ui.add_space(10.0);

        match current_state {
            AppState::Idle => {
                let msg = self.status_message.lock().unwrap();
                ui.label(egui::RichText::new(&*msg).color(egui::Color32::GRAY));
            }
            AppState::WaitingForValorant => {
                ui.label(
                    egui::RichText::new("⏳ Waiting for Valorant to start...")
                        .color(egui::Color32::YELLOW),
                );
            }
            AppState::Running => {
                ui.label(
                    egui::RichText::new("✅ Valorant is running with stretched resolution")
                        .color(egui::Color32::GREEN),
                );
            }
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("Current resolution:");
            if let Some(current) = ResolutionManager::get_current() {
                ui.label(format!("{}x{}", current.width, current.height));
            }
        });
    }

    fn show_presets_tab(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("Save Current Settings as Preset:");
            ui.horizontal(|ui| {
                ui.label("Preset Name:");
                ui.text_edit_singleline(&mut self.new_preset_name);
                if ui.button("💾 Save").clicked() {
                    self.save_current_as_preset();
                }
            });
        });

        ui.add_space(15.0);

        ui.group(|ui| {
            ui.label("Saved Presets:");

            if self.presets.is_empty() {
                ui.label("No presets saved yet");
            } else {
                for (i, preset_name) in self.presets.clone().iter().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.button("📋 Load").clicked() {
                            self.load_preset(i);
                        }
                        if ui.button("🗑 Delete").clicked() {
                            self.delete_preset(i);
                        }
                        ui.label(preset_name);
                    });
                }
            }
        });

        ui.add_space(10.0);

        if !self.presets_status_message.is_empty() {
            ui.label(egui::RichText::new(&self.presets_status_message).color(egui::Color32::GRAY));
        }
    }

    fn show_settings_tab(&mut self, ui: &mut egui::Ui) {
        let is_in_startup = self.startup_manager.is_enabled();

        ui.group(|ui| {
            ui.label("Startup Settings:");
            ui.horizontal(|ui| {
                let button_text = if is_in_startup {
                    "Remove from Startup"
                } else {
                    "Add to Startup"
                };

                if ui.button(button_text).clicked() {
                    if is_in_startup {
                        if let Err(e) = self.startup_manager.disable() {
                            self.settings_status_message =
                                format!("Failed to remove from startup: {}", e);
                        } else {
                            self.settings_status_message = "Removed from startup".to_string();
                        }
                    } else {
                        if let Err(e) = self.startup_manager.enable() {
                            self.settings_status_message =
                                format!("Failed to add to startup: {}", e);
                        } else {
                            self.settings_status_message = "Added to startup".to_string();
                        }
                    }
                    self.config.auto_launch = !is_in_startup;
                    let _ = self.config_manager.save(&self.config);
                }

                if is_in_startup {
                    ui.label("✅ In startup");
                } else {
                    ui.label("❌ Not in startup");
                }
            });
        });

        ui.add_space(10.0);

        if !self.settings_status_message.is_empty() {
            ui.label(egui::RichText::new(&self.settings_status_message).color(egui::Color32::GRAY));
        }

        ui.add_space(20.0);

        ui.group(|ui| {
            ui.heading("About WideVal");
            ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
            ui.horizontal(|ui| {
                ui.label("Author:");
                ui.hyperlink_to(env!("CARGO_PKG_AUTHORS"), "https://github.com/kirushkinx");
            });
            ui.label("Modern Valorant resolution stretcher");
            ui.add_space(5.0);
            ui.label("Features:");
            ui.label("• Automatic resolution switching");
            ui.label("• Multiple account support");
            ui.label("• Preset system");
            ui.label("• Custom resolutions");
        });
    }
}