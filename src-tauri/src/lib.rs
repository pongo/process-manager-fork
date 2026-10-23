mod commands;
mod config_handler;
mod log_handler;
mod monitoring;
mod process_manager;
mod terminal;
mod types;

use commands::AppState;
use config_handler::ConfigHandler;
use log_handler::LogHandler;
use process_manager::ProcessManager;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use sysinfo;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager, WebviewWindowBuilder,
};

#[derive(Default)]
struct ExitState {
    in_progress: AtomicBool,
    allowed: AtomicBool,
}

#[derive(Default)]
struct WindowState {
    ready: bool,
    show_requested: bool,
}

#[tauri::command]
fn main_window_ready(window: tauri::WebviewWindow) -> tauri::Result<()> {
    let state = window.state::<std::sync::Mutex<WindowState>>();
    let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
    state.ready = true;
    if state.show_requested {
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    let state = app.state::<std::sync::Mutex<WindowState>>();
    let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
    state.show_requested = true;
    if let Some(window) = app.get_webview_window("main") {
        if !state.ready {
            return Ok(());
        }
        window.unminimize()?;
        window.maximize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    state.ready = false;
    WebviewWindowBuilder::from_config(app, config)?.build()?;
    Ok(())
}

fn quit_app(app: &tauri::AppHandle) {
    if app.state::<ExitState>().in_progress.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = app.state::<AppState>().manager.lock()
            .unwrap_or_else(|e| e.into_inner())
            .shutdown();
        if let Err(error) = result {
            app.state::<ExitState>().in_progress.store(false, Ordering::SeqCst);
            report_process_error("Quit Failed", &error);
            return;
        }
        let handle = app.clone();
        if let Err(error) = app.run_on_main_thread(move || finish_quit(&handle)) {
            app.state::<ExitState>().in_progress.store(false, Ordering::SeqCst);
            report_process_error("Quit Failed", &error.to_string());
        }
    });
}

fn report_process_error(title: &str, error: &str) {
    eprintln!("{title}: {error}");
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        let message: Vec<u16> = error.encode_utf16().chain(Some(0)).collect();
        unsafe {
            MessageBoxW(std::ptr::null_mut(), message.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
        }
    }
}

fn finish_quit(app: &tauri::AppHandle) {
    app.state::<ExitState>().allowed.store(true, Ordering::SeqCst);
    let windows = app.webview_windows();
    if windows.is_empty() {
        app.exit(0);
        return;
    }

    // Destroy WebViews before the event loop exits; the last window triggers shutdown.
    for window in windows.values() {
        if let Err(error) = window.destroy() {
            eprintln!("Failed to destroy window {}: {error}", window.label());
            app.exit(1);
            return;
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize config and log directories
    let log_dir = ConfigHandler::get_logs_dir()
        .expect("Failed to get logs directory");
    let log_handler = LogHandler::new(log_dir)
        .expect("Failed to initialize log handler");

    let process_manager = Arc::new(std::sync::Mutex::new(ProcessManager::new()));
    let log_handler = Arc::new(log_handler);
    
    // Cache total RAM; process CPU snapshots are collected on demand.
    let sys = {
        let mut s = sysinfo::System::new();
        s.refresh_memory();
        s
    };

    let app_state = AppState {
        manager: Arc::clone(&process_manager),
        log_handler: Arc::clone(&log_handler),
        system: Arc::new(std::sync::Mutex::new(sys)),
        terminal: Arc::new(std::sync::Mutex::new(terminal::TerminalManager::new())),
    };

    tauri::Builder::default()
        .manage(app_state)
        .manage(ExitState::default())
        .manage(std::sync::Mutex::new(WindowState {
            ready: false,
            show_requested: !std::env::args().any(|arg| arg == "--start-in-tray"),
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let configs = ConfigHandler::load_configs()?;
            let state = app.state::<AppState>();
            let mut manager = state.manager.lock().unwrap_or_else(|e| e.into_inner());
            for config in configs {
                let instance = process_manager::ProcessInstance::from_config(&config);
                manager.processes.insert(instance.id.clone(), instance);
            }
            drop(manager);

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = commands::start_auto_start(
                    app_handle.state::<AppState>(),
                    app_handle.clone(),
                )
                .await
                {
                    eprintln!("Failed to start configured processes: {error}");
                }
            });

            // Build system tray menu
            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
            let start_all_item = MenuItem::with_id(app, "start_all", "Start All", true, None::<&str>)?;
            let stop_all_item = MenuItem::with_id(app, "stop_all", "Stop All", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let sep2 = PredefinedMenuItem::separator(app)?;

            let menu = Menu::with_items(
                app,
                &[&show, &hide, &sep1, &start_all_item, &stop_all_item, &sep2, &quit],
            )?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    // Left-click on tray icon → show window.
                    // Right-click is handled automatically by the menu.
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let _ = show_main_window(&tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        let _ = show_main_window(app);
                    }
                    "hide" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.destroy();
                        }
                    }
                    "start_all" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(error) = commands::start_all(app.state::<AppState>(), app.clone()).await {
                                eprintln!("Failed to start all processes: {error}");
                            }
                        });
                    }
                    "stop_all" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(error) = commands::stop_all(app.state::<AppState>(), app.clone()).await {
                                report_process_error("Stop All Failed", &error);
                            }
                        });
                    }
                    "quit" => {
                        quit_app(app);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Destroy the WebView2 instance while keeping the Rust process and tray alive.
                // The tray recreates the window on the next Show/click action.
                api.prevent_close();
                let _ = window.destroy();
            }
        })
        .invoke_handler(tauri::generate_handler![
            main_window_ready,
            commands::get_processes,
            commands::start_process,
            commands::stop_process,
            commands::restart_process,
            commands::add_process,
            commands::remove_process,
            commands::update_process,
            commands::get_metrics,
            commands::get_logs,
            commands::clear_logs,
            commands::save_config,
            commands::load_config,
            commands::start_all,
            commands::stop_all,
            commands::terminal_run,
            commands::terminal_kill,
            commands::terminal_set_cwd,
            commands::terminal_get_cwd,
            commands::terminal_add_process,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, code: None, .. } = event {
                if !app.state::<ExitState>().allowed.load(Ordering::SeqCst) {
                    api.prevent_exit();
                }
            }
        });
}

