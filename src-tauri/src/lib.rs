mod catalog;
mod import;
mod library;
mod queue;
mod settings;
mod tagger;
mod tools;
mod ytdlp;

use settings::Settings;
use std::{collections::HashMap, sync::Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::WindowEvent;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

pub struct AppState {
    pub http: reqwest::Client,
    pub settings: Mutex<Settings>,
    pub queue: Mutex<queue::Queue>,
    pub covers: Mutex<HashMap<String, Vec<u8>>>,
    pub library: Mutex<library::Cache>,
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    app.state::<AppState>().settings.lock().unwrap().clone()
}

#[tauri::command]
fn set_settings(app: AppHandle, settings: Settings) {
    settings::save(&app, &settings);
    *app.state::<AppState>().settings.lock().unwrap() = settings;
    queue::pump(&app);
}

#[tauri::command]
fn open_path(app: AppHandle, path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if p.is_file() {
        app.opener().reveal_item_in_dir(p).map_err(|e| e.to_string())
    } else {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        app.opener().open_path(path, None::<&str>).map_err(|e| e.to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let h = app.handle().clone();
            let mut s = settings::load(&h);
            if s.output_dir.is_empty() {
                s.output_dir = h
                    .path()
                    .audio_dir()
                    .or_else(|_| h.path().home_dir())
                    .map(|p| p.join("mpthree-dl"))
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
            }
            let http = reqwest::Client::builder()
                .user_agent("mpthree-dl/0.1")
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("http client");
            app.manage(AppState {
                http,
                settings: Mutex::new(s),
                queue: Mutex::new(queue::Queue::default()),
                covers: Mutex::new(HashMap::new()),
                library: Mutex::new(None),
            });

            let show = MenuItem::with_id(app, "show", "Show mpthree-dl", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .tooltip("mpthree-dl")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, ev| match ev.id.as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, ev| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                        show_main(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            queue::restore(&h);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.app_handle().state::<AppState>().settings.lock().unwrap().close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            open_path,
            library::scan_library,
            tools::tools_status,
            tools::install_ytdlp,
            tools::update_ytdlp,
            tools::install_ffmpeg,
            catalog::search_catalog,
            catalog::artist_releases,
            catalog::release_tracks,
            import::import_playlist,
            ytdlp::probe_url,
            ytdlp::search_web,
            ytdlp::stream_url,
            queue::enqueue,
            queue::enqueue_releases,
            queue::list_jobs,
            queue::cancel_job,
            queue::cancel_all,
            queue::retry_job,
            queue::clear_finished,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
