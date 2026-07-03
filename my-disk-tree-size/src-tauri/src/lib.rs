pub mod commands;
pub mod models;
mod operation_log;

use commands::*;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir { file_name: None },
                ))
                .build(),
        )
        .setup(|app| {
            app.manage(AppData::load(app.handle()));
            install_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            get_settings,
            set_settings,
            check_update_mock,
            scan_disk_tree_shallow,
            scan_disk_subtree,
            save_scan_tree,
            scan_disk_tree,
            list_scan_runs,
            find_duplicate_files,
            classify_videos,
            move_classified_video,
            get_operation_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running MyDiskTreeSize");
}

fn install_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "隐藏", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &hide, &quit])?;
    let tray_icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(tray_icon)
        .icon_as_template(false)
        .tooltip("MyDiskTreeSize")
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

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
