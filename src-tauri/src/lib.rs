mod commands;
mod database;
mod http_client;
mod match_status;
mod models;
mod open_data;

use commands::*;
use database::Database;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(
            tauri_plugin_log::Builder::default()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir { file_name: None },
                ))
                .build(),
        )
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let db = Database::new(&app_dir).map_err(|err| {
                log::error!("database init failed: {err}");
                err
            })?;
            app.manage(db);

            install_tray(app)?;
            install_shortcut(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            refresh_matches,
            get_matches,
            get_standings,
            get_bracket,
            get_teams,
            toggle_favorite_team,
            save_prediction,
            get_predictions,
            get_settings,
            set_settings,
            test_football_data_token,
            test_ai_model_config,
            generate_ai_evaluation,
            toggle_spoiler_mode,
            open_floating_scorebar,
            close_floating_scorebar,
            check_update_mock
        ])
        .run(tauri::generate_context!())
        .expect("error while running WorldCupIssue");
}

fn install_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示 WorldCupIssue", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "隐藏到托盘", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &hide, &quit])?;
    let tray_icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(tray_icon)
        .icon_as_template(false)
        .tooltip("WorldCupIssue（世界杯组手）")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn install_shortcut(app: &tauri::App) -> tauri::Result<()> {
    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyC);
    if let Err(err) = app.global_shortcut().on_shortcut(shortcut, |app, _shortcut, _event| {
        show_main(app);
        let _ = app.emit("worldcupissue://hotkey-open", ());
    }) {
        log::warn!("failed to register Ctrl+Alt+C shortcut: {err}");
    }
    Ok(())
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn ensure_scorebar(app: &tauri::AppHandle) -> tauri::Result<()> {
    if app.get_webview_window("scorebar").is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "scorebar", WebviewUrl::App("/?scorebar=1".into()))
        .title("WorldCupIssue Scorebar")
        .inner_size(420.0, 86.0)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .transparent(true)
        .skip_taskbar(true)
        .build()?;
    Ok(())
}
