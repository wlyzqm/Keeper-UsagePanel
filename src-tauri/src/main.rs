#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod settings;
mod windows;
use keeper_core::{Keeper, Query};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State, WebviewWindow};
use tokio::sync::RwLock;

struct AppState {
    settings: Mutex<settings::Settings>,
    client: RwLock<Option<Arc<Keeper>>>,
    last: Mutex<Value>,
    hover: Mutex<windows::Hover>,
}
#[tauri::command]
fn get_settings(state: State<AppState>) -> settings::Settings {
    state.settings.lock().unwrap().clone()
}
#[tauri::command]
async fn save_settings(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    mut value: settings::Settings,
    clear_password: bool,
) -> Result<(), String> {
    if window.label() != "settings" {
        return Err("请通过连接设置修改配置".into());
    }
    if !(1..=60).contains(&value.poll_seconds) {
        return Err("刷新间隔需为 1–60 秒".into());
    }
    if !matches!(value.theme.as_str(), "light" | "dark") {
        value.theme = "light".into();
    }
    {
        let old = state.settings.lock().unwrap();
        if value.password.is_empty() && !clear_password {
            value.password = old.password.clone()
        }
        value.x = old.x;
        value.y = old.y;
    }
    value.proxy_url = value.proxy_url.trim().to_string();
    value.widget_font = value.widget_font.trim().to_string();
    if value.widget_font.is_empty() {
        value.widget_font = "HarmonyOS Sans SC".into();
    }
    value.endpoint = value.endpoint.trim().trim_end_matches('/').to_string();
    value.has_password = !value.password.is_empty();
    let client = Arc::new(Keeper::with_proxy(
        &value.endpoint,
        &value.password,
        value.allow_private_http,
        &value.proxy_url,
    )?);
    client.login().await?; // Verify before overwriting working credentials.
    settings::save(&value)?;
    *state.client.write().await = Some(client);
    *state.settings.lock().unwrap() = value.clone();
    *state.last.lock().unwrap() = Value::Null;
    let _ = app.emit("configured", &value);
    let _ = window.hide();
    if let Some(w) = app.get_webview_window("widget") {
        let _ = w.show();
    }
    Ok(())
}
#[tauri::command]
async fn sample(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    if window.label() != "widget" {
        return Err("仅悬浮球执行全局轮询".into());
    }
    if !window.is_visible().unwrap_or(false) {
        return Err("悬浮球已隐藏，暂停采样".into());
    }
    if windows::session_locked() {
        return Err("Windows 已锁定，暂停采样".into());
    }
    let client = state.client.read().await.clone().ok_or("尚未配置 Keeper")?;
    let result = tokio::time::timeout(std::time::Duration::from_secs(15), client.sample())
        .await
        .map_err(|_| "采样超时，保留上次基线".to_string())
        .and_then(|r| r);
    let current = state.client.read().await;
    if !current.as_ref().is_some_and(|c| Arc::ptr_eq(c, &client)) {
        return Err("连接配置已更新".into());
    }
    match &result {
        Ok(value) => {
            *state.last.lock().unwrap() = value.clone();
            let _ = app.emit("sample", value);
        }
        Err(error) => {
            let _ = app.emit("connection-error", error);
        }
    }
    result
}
#[tauri::command]
fn last_sample(state: State<AppState>) -> Value {
    state.last.lock().unwrap().clone()
}
#[tauri::command]
async fn get_view(state: State<'_, AppState>, view: String, query: Query) -> Result<Value, String> {
    let client = state
        .client
        .read()
        .await
        .clone()
        .ok_or("先连接你的 Keeper，即可查看用量")?;
    client.view(&view, query).await
}
#[tauri::command]
fn window_action(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: State<AppState>,
    action: String,
) -> Result<(), String> {
    match action.as_str() {
        "settings" => windows::show_settings(&app),
        "detail" => windows::show_detail(&app),
        "close-detail" => {
            state.hover.lock().unwrap().suppressed = true;
            windows::hide_detail(&app);
        }
        "close-settings" => {
            if window.label() == "settings" {
                let _ = window.hide();
            }
        }
        "drag" => {
            if window.label() == "widget" {
                state.hover.lock().unwrap().dragging = true;
                windows::hide_detail(&app);
                window.start_dragging().map_err(|e| e.to_string())?;
            }
        }
        "hide" => windows::hide(&app),
        "quit" => app.exit(0),
        _ => return Err("未知窗口操作".into()),
    }
    Ok(())
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_webview_window("widget") {
                let _ = w.show();
            }
            windows::show_settings(app);
        }))
        .setup(|app| {
            let config = settings::load();
            let client = if config.endpoint.is_empty() {
                None
            } else {
                Keeper::with_proxy(
                    &config.endpoint,
                    &config.password,
                    config.allow_private_http,
                    &config.proxy_url,
                )
                .ok()
                .map(Arc::new)
            };
            let need_config =
                client.is_none() || (!config.remember_password && config.has_password == false);
            app.manage(AppState {
                settings: Mutex::new(config),
                client: RwLock::new(client),
                last: Mutex::new(json!(null)),
                hover: Mutex::new(Default::default()),
            });
            windows::create(app.handle())?;
            if need_config {
                windows::show_settings(app.handle());
            }
            windows::track(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            sample,
            last_sample,
            get_view,
            window_action
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window.label() == "detail" {
                    windows::hide_detail(window.app_handle());
                } else {
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("无法启动 Keeper UsagePanel");
}
