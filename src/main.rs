slint::include_modules!();

mod config;
mod printer;
mod api;
mod system;

use clap::Parser;
use config::{KConfig, MenuAction, MenuRouter};
use printer::state::PrinterState;
use api::websocket::{spawn_moonraker_client, MoonrakerCommand};
use api::rest::extract_thumbnail_from_gcode;
use system::l10n::{init_global_translator, get_translation};
use system::power::PowerController;

use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{info, warn, error};

#[derive(Debug, PartialEq)]
pub enum DisplayStack {
    Wayland,
    X11,
    DirectKms,
}

pub fn detect_display_stack(env_wayland: Option<&str>, env_x11: Option<&str>) -> DisplayStack {
    if env_wayland.is_some() {
        DisplayStack::Wayland
    } else if env_x11.is_some() {
        DisplayStack::X11
    } else {
        DisplayStack::DirectKms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_display_stack_wayland() {
        assert_eq!(detect_display_stack(Some("wayland-0"), None), DisplayStack::Wayland);
        assert_eq!(detect_display_stack(Some("wayland-0"), Some(":0")), DisplayStack::Wayland);
    }

    #[test]
    fn test_detect_display_stack_x11() {
        assert_eq!(detect_display_stack(None, Some(":0")), DisplayStack::X11);
    }

    #[test]
    fn test_detect_display_stack_kms() {
        assert_eq!(detect_display_stack(None, None), DisplayStack::DirectKms);
    }
}

#[derive(Parser, Debug)]
#[command(name = "rklipperscreen", version = "0.1.0", about = "rKlipperScreen Slint Port")]
struct CliArgs {
    #[arg(short, long, default_value = "/home/pi/printer_data/config/KlipperScreen.conf")]
    config: String,

    #[arg(short, long, default_value = "/tmp/KlipperScreen.log")]
    logfile: String,

    #[arg(short, long, default_value = "127.0.0.1")]
    address: String,

    #[arg(short, long, default_value_t = 7125)]
    port: u16,

    #[arg(long, default_value_t = false)]
    systemd: bool,
}

fn init_logging(logfile_path: &str) {
    use tracing_subscriber::prelude::*;
    
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
        
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout);

    // Expand tilde or home variable if present in path
    let resolved_path = if logfile_path.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/pi".to_string());
        logfile_path.replacen('~', &home, 1)
    } else {
        logfile_path.to_string()
    };

    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&resolved_path)
    {
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(Mutex::new(file));
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(stdout_layer)
            .with(file_layer)
            .try_init();
    } else {
        // Fallback to stdout only if file is not writable
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(stdout_layer)
            .try_init();
        warn!("Could not open logfile at {}, logging to stdout only", resolved_path);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Parse Command Line Arguments
    let args = CliArgs::parse();

    // 2. Initialize dual logging (stdout + file)
    init_logging(&args.logfile);
    info!("Starting rKlipperScreen...");

    // 3. Single-Instance Enforcement
    let mut lock = fslock::LockFile::open("/tmp/KlipperScreen.lock")?;
    if !lock.try_lock()? {
        error!("Another instance of rKlipperScreen is already running!");
        std::process::exit(1);
    }

    // 4. Load cascading config
    let config = KConfig::load_cascading(Some(&args.config));

    // 5. Initialize Translator
    init_global_translator(&config.language);

    let wayland = std::env::var("WAYLAND_DISPLAY").ok();
    let x11 = std::env::var("DISPLAY").ok();

    let display_stack = detect_display_stack(wayland.as_deref(), x11.as_deref());

    if let DisplayStack::DirectKms = display_stack {
        info!("No X11 or Wayland session detected. Initializing Direct KMS backend.");
        slint::platform::set_platform(Box::new(
            i_slint_backend_linuxkms::BackendBuilder::default().build().map_err(|e| e.to_string())?
        )).map_err(|e| e.to_string())?;
    } else {
        info!("Desktop environment detected (Wayland: {}, X11: {}), using default winit backend.", wayland.is_some(), x11.is_some());
    }

    // 6. Build Slint MainApp
    let app = MainApp::new()?;

    if let DisplayStack::DirectKms = display_stack {
        let app_weak = app.as_weak();
        tokio::spawn(async move {
            system::touch::start_touch_router(app_weak).await;
        });
    }

    // Dynamic icon loader helper based on active theme
    let load_theme_icon = {
        let theme_name = config.theme.clone();
        move |icon_name: &str| -> slint::Image {
            let path = format!("styles/{}/images/{}.svg", theme_name, icon_name);
            if Path::new(&path).exists() {
                slint::Image::load_from_path(Path::new(&path)).unwrap_or_default()
            } else {
                let fallback = format!("styles/z-bolt/images/{}.svg", icon_name);
                slint::Image::load_from_path(Path::new(&fallback)).unwrap_or_default()
            }
        }
    };

    // Load main navigation sidebar icons
    app.set_icon_home(load_theme_icon("home"));
    app.set_icon_back(load_theme_icon("back"));
    app.set_icon_settings(load_theme_icon("settings"));
    app.set_icon_estop(load_theme_icon("emergency"));

    // Power management controller
    let power_controller = Arc::new(PowerController::new());

    // Channels for async task orchestration
    let (ui_update_tx, mut ui_update_rx) = mpsc::channel::<()>(10);
    let (cmd_tx, cmd_rx) = mpsc::channel::<MoonrakerCommand>(30);

    // Dynamic state container shared with Moonraker websocket
    let printer_state = Arc::new(Mutex::new(PrinterState::new()));

    // Spawn Moonraker Client
    spawn_moonraker_client(
        args.address,
        args.port,
        "".to_string(), // API key
        printer_state.clone(),
        ui_update_tx.clone(),
        cmd_rx,
    );

    // Virtual Menu Router
    let menu_router = Arc::new(Mutex::new(MenuRouter::new(&config)));

    // Bind L10n translation global callback
    app.global::<L10n>().on_translate(move |text| {
        get_translation(text.as_str()).into()
    });

    // Populate Initial Menu Items model in Slint
    let sync_menu_items = {
        let app_weak = app.as_weak();
        let menu_router_clone = menu_router.clone();
        let printer_state_clone = printer_state.clone();
        let load_icon = load_theme_icon.clone();
        move || {
            if let Some(app) = app_weak.upgrade() {
                let router = menu_router_clone.lock().unwrap();
                let p_state = printer_state_clone.lock().unwrap();
                let active_items = router.get_active_menu_items(&p_state.raw_state);

                let slint_items: Vec<MenuItemInfo> = active_items
                    .into_iter()
                    .map(|item| {
                        let icon_img = load_icon(&item.icon_name);
                        let is_submenu = match item.action {
                            MenuAction::SubMenu(_) => true,
                            _ => false,
                        };
                        let panel_name = match &item.action {
                            MenuAction::NavigateToPanel(p) => p.clone(),
                            MenuAction::SubMenu(path) => path.clone(),
                            _ => String::new(),
                        };

                        MenuItemInfo {
                            name: item.name.into(),
                            icon_name: item.icon_name.into(),
                            icon: icon_img,
                            panel_name: panel_name.into(),
                            is_submenu,
                        }
                    })
                    .collect();

                let model = ModelRc::new(VecModel::from(slint_items));
                app.set_menu_items(model);
                app.set_menu_title(router.get_active_menu_path().into());
            }
        }
    };
    sync_menu_items();

    // Menu callback bindings
    let menu_router_for_clicks = menu_router.clone();
    let printer_state_for_clicks = printer_state.clone();
    let sync_menu_for_clicks = sync_menu_items.clone();
    let app_weak_for_clicks = app.as_weak();
    let cmd_tx_for_clicks = cmd_tx.clone();

    app.on_menu_item_clicked(move |idx| {
        if let Some(app) = app_weak_for_clicks.upgrade() {
            let mut router = menu_router_for_clicks.lock().unwrap();
            let p_state = printer_state_for_clicks.lock().unwrap();
            let active_items = router.get_active_menu_items(&p_state.raw_state);

            if let Some(item) = active_items.get(idx as usize) {
                match &item.action {
                    MenuAction::SubMenu(path) => {
                        router.navigate_to(path.clone());
                        drop(router);
                        sync_menu_for_clicks();
                    }
                    MenuAction::NavigateToPanel(panel) => {
                        app.set_current_panel(panel.as_str().into());
                        app.set_menu_title(panel.as_str().into());
                    }
                    MenuAction::ExecuteGcode(gcode) => {
                        let _ = cmd_tx_for_clicks.try_send(MoonrakerCommand::SendGcode(gcode.clone()));
                    }
                    MenuAction::SystemCommand(cmd) => {
                        info!("Executing system command: {}", cmd);
                        let _ = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(cmd)
                            .status();
                    }
                }
            }
        }
    });

    let menu_router_for_back = menu_router.clone();
    let sync_menu_for_back = sync_menu_items.clone();
    let app_weak_for_back = app.as_weak();
    app.on_menu_go_back(move || {
        if let Some(app) = app_weak_for_back.upgrade() {
            if app.get_current_panel() != "main_menu" {
                app.set_current_panel("main_menu".into());
                sync_menu_for_back();
            } else {
                let mut router = menu_router_for_back.lock().unwrap();
                if router.go_back() {
                    drop(router);
                    sync_menu_for_back();
                }
            }
        }
    });

    let menu_router_for_home = menu_router.clone();
    let sync_menu_for_home = sync_menu_items.clone();
    let app_weak_for_home = app.as_weak();
    app.on_menu_go_home(move || {
        if let Some(app) = app_weak_for_home.upgrade() {
            app.set_current_panel("main_menu".into());
            let mut router = menu_router_for_home.lock().unwrap();
            router.go_home();
            drop(router);
            sync_menu_for_home();
        }
    });

    // Temperature controls callback
    let cmd_tx_for_temp = cmd_tx.clone();
    app.on_set_temperature(move |device, temp| {
        let gcode = if device == "extruder" {
            format!("M104 S{}", temp)
        } else {
            format!("M140 S{}", temp)
        };
        let _ = cmd_tx_for_temp.try_send(MoonrakerCommand::SendGcode(gcode));
    });

    // Jog controls callback
    let cmd_tx_for_jog = cmd_tx.clone();
    app.on_jog_move(move |axis, dist| {
        let gcode = format!("G91\nG1 {}{} F3000\nG90", axis, dist);
        let _ = cmd_tx_for_jog.try_send(MoonrakerCommand::SendGcode(gcode));
    });

    // Homing callback
    let cmd_tx_for_home = cmd_tx.clone();
    app.on_home_axis(move |axis| {
        let gcode = if axis == "all" {
            "G28".to_string()
        } else {
            format!("G28 {}", axis.to_uppercase())
        };
        let _ = cmd_tx_for_home.try_send(MoonrakerCommand::SendGcode(gcode));
    });

    // Console script submission
    let cmd_tx_for_console = cmd_tx.clone();
    let app_weak_for_console = app.as_weak();
    app.on_send_gcode(move |cmd| {
        let _ = cmd_tx_for_console.try_send(MoonrakerCommand::SendGcode(cmd.to_string()));
        if let Some(app) = app_weak_for_console.upgrade() {
            let lines = app.get_console_lines();
            let mut items: Vec<ConsoleLine> = lines.iter().collect();
            items.push(ConsoleLine {
                text: cmd.clone(),
                is_input: true,
            });
            if items.len() > 100 {
                items.remove(0);
            }
            app.set_console_lines(ModelRc::new(VecModel::from(items)));
        }
    });

    // E-STOP click
    let cmd_tx_for_estop = cmd_tx.clone();
    app.on_estop_clicked(move || {
        let _ = cmd_tx_for_estop.try_send(MoonrakerCommand::EmergencyStop);
    });

    // File Manager Explorer State
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/home/pi".to_string());
    let gcode_dir_base = PathBuf::from(home_dir).join("printer_data/gcodes");
    if !gcode_dir_base.exists() {
        let _ = fs::create_dir_all(&gcode_dir_base);
        let dummy_file = gcode_dir_base.join("test_print.gcode");
        let _ = fs::write(
            dummy_file,
            "; thumbnail begin 32x32 100\n; iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAFElEQVR4nGP8z8Dwn4GBgYGJAQoAHxcCAk+Uzr4AAAAASUVORK5CYII=\n; thumbnail end\nM104 S200\nG28\nG1 Z10\nM104 S0\n"
        );
    }

    let active_dir = Arc::new(Mutex::new(gcode_dir_base.clone()));
    let app_weak_for_files = app.as_weak();
    let active_dir_clone = active_dir.clone();
    let gcode_dir_base_clone = gcode_dir_base.clone();
    let folder_icon = load_theme_icon("folder");
    let file_icon = load_theme_icon("file");
    let cmd_tx_for_files = cmd_tx.clone();

    // Populate file list helper
    let sync_gcode_files = move |app: &MainApp, current_path: &Path| {
        let mut list_items = Vec::new();
        if let Ok(entries) = fs::read_dir(current_path) {
            for entry in entries.filter_map(Result::ok) {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                
                let size_str = if size > 1024 * 1024 {
                    format!("{:.1} MB", size as f32 / (1024.0 * 1024.0))
                } else {
                    format!("{:.1} KB", size as f32 / 1024.0)
                };

                let thumbnail_img = if is_dir {
                    folder_icon.clone()
                } else {
                    extract_thumbnail_from_gcode(entry.path()).unwrap_or_else(|| file_icon.clone())
                };

                list_items.push(GCodeFileInfo {
                    name: name.into(),
                    is_dir,
                    size: size_str.into(),
                    thumbnail: thumbnail_img,
                });
            }
        }
        
        app.set_gcode_files(ModelRc::new(VecModel::from(list_items)));
    };

    // Initial load of gcode files
    sync_gcode_files(&app, &active_dir_clone.lock().unwrap());

    app.on_file_action(move |action, filename| {
        if let Some(app) = app_weak_for_files.upgrade() {
            let mut current = active_dir_clone.lock().unwrap();
            match action.as_str() {
                "back" => {
                    if current.parent().is_some() && *current != gcode_dir_base_clone {
                        *current = current.parent().unwrap().to_path_buf();
                        sync_gcode_files(&app, &current);
                    }
                }
                "select" => {
                    let next_dir = current.join(filename.as_str());
                    if next_dir.is_dir() {
                        *current = next_dir;
                        sync_gcode_files(&app, &current);
                    }
                }
                "print" => {
                    let relative_path = current.join(filename.as_str());
                    if let Ok(rel) = relative_path.strip_prefix(&gcode_dir_base_clone) {
                        let name_str = rel.to_string_lossy().to_string();
                        let _ = cmd_tx_for_files.try_send(MoonrakerCommand::StartPrint(name_str));
                        app.set_current_panel("main_menu".into());
                    }
                }
                _ => {}
            }
        }
    });

    // Tokio Async Event loop processor
    let app_weak_for_updates = app.as_weak();
    let printer_state_for_updates = printer_state.clone();
    let sync_menu_for_updates = sync_menu_items.clone();
    tokio::spawn(async move {
        while let Some(_) = ui_update_rx.recv().await {
            let state = printer_state_for_updates.lock().unwrap().clone();
            let _ = slint::invoke_from_event_loop({
                let app_weak = app_weak_for_updates.clone();
                let sync_menu = sync_menu_for_updates.clone();
                move || {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_printer_state(state.printer_state.into());
                        app.set_extruder_temp(state.extruder_temp);
                        app.set_extruder_target(state.extruder_target);
                        app.set_bed_temp(state.bed_temp);
                        app.set_bed_target(state.bed_target);
                        app.set_fan_speed(state.fan_speed);
                        app.set_print_progress(state.print_progress);
                        app.set_print_filename(state.print_filename.into());

                        sync_menu();
                    }
                }
            });
        }
    });

    // Run Slint Event Loop
    app.run()?;

    // Ensure display power is restored on graceful exit
    power_controller.set_screen_power(true);

    Ok(())
}
