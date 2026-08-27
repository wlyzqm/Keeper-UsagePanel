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
    scope: RwLock<Scope>,
}
#[derive(Clone, Default, serde::Serialize)]
struct Scope {
    api_key_id: String,
    label: String,
    revision: u64,
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
    if value.display_hold_seconds > 300 {
        return Err("非零数据保留时间需为 0–300 秒".into());
    }
    if !matches!(value.theme.as_str(), "light" | "dark") {
        value.theme = "light".into();
    }
    {
        let old = state.settings.lock().unwrap();
        if value.password.is_empty()
            && !clear_password
            && value.auth_mode == old.auth_mode
            && value.endpoint.trim().trim_end_matches('/') == old.endpoint
        {
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
    let client = Arc::new(Keeper::connect(
        &value.endpoint,
        &value.password,
        value.allow_private_http,
        &value.proxy_url,
        value.auth_mode,
    )?);
    client.login().await?; // Verify before overwriting working credentials.
    settings::save(&value)?;
    let mut current = state.client.write().await;
    let mut scope = state.scope.write().await;
    *scope = Scope {
        revision: scope.revision + 1,
        ..Default::default()
    };
    let revision = scope.revision;
    *current = Some(client);
    *state.last.lock().unwrap() = Value::Null;
    drop(scope);
    drop(current);
    *state.settings.lock().unwrap() = value.clone();
    let _ = app.emit("configured", json!({"settings":value,"revision":revision}));
    let _ = window.hide();
    if let Some(w) = app.get_webview_window("widget") {
        windows::show_inactive(&w);
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
        return Err("仅悬浮球执行用量轮询".into());
    }
    if !windows::visible(&window) {
        return Err("悬浮球已隐藏，暂停采样".into());
    }
    if windows::session_locked() {
        return Err("Windows 已锁定，暂停采样".into());
    }
    let client = state.client.read().await.clone().ok_or("尚未配置 Keeper")?;
    let scope = state.scope.read().await.clone();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        client.sample_scoped(&scope.api_key_id, scope.revision),
    )
    .await
    .map_err(|_| "采样超时，保留上次基线".to_string())
    .and_then(|r| r);
    let current = state.client.read().await;
    let active_scope = state.scope.read().await;
    if active_scope.revision != scope.revision
        || !current.as_ref().is_some_and(|c| Arc::ptr_eq(c, &client))
    {
        return Err("连接配置已更新".into());
    }
    let result = result.map(|mut value| {
        value["revision"] = json!(scope.revision);
        value["api_key_id"] = json!(scope.api_key_id);
        value
    });
    match &result {
        Ok(value) => {
            *state.last.lock().unwrap() = value.clone();
            let _ = app.emit("sample", value);
        }
        Err(error) => {
            let _ = app.emit(
                "connection-error",
                json!({"message":error,"revision":scope.revision}),
            );
        }
    }
    result
}
#[tauri::command]
fn last_sample(state: State<AppState>) -> Value {
    state.last.lock().unwrap().clone()
}
#[tauri::command]
async fn get_access(state: State<'_, AppState>) -> Result<Value, String> {
    let current = state.client.read().await;
    let client = current.as_ref().ok_or("请先配置 Keeper 连接")?;
    let mut access = client.access().await?;
    access["scope"] = json!(*state.scope.read().await);
    Ok(access)
}
#[tauri::command]
async fn set_scope(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    api_key_id: String,
) -> Result<Value, String> {
    if window.label() != "detail" {
        return Err("请在面板中选择 Key owner".into());
    }
    let current = state.client.read().await;
    let client = current.as_ref().ok_or("请先连接 Keeper")?;
    let mut access = client.access().await?;
    if client.is_viewer() {
        return Err("sk 登录不能切换 Key owner".into());
    }
    let label = if api_key_id.is_empty() {
        "全部 Key".to_string()
    } else {
        let keys = client.view("keys", Query::default()).await?;
        keys["options"]
            .as_array()
            .and_then(|items| items.iter().find(|k| k["id"].as_str() == Some(&api_key_id)))
            .and_then(|k| k["label"].as_str())
            .ok_or("Key 已不存在，请重新连接刷新列表")?
            .to_string()
    };
    let mut scope = state.scope.write().await;
    if scope.api_key_id != api_key_id {
        *scope = Scope {
            api_key_id,
            label,
            revision: scope.revision + 1,
        };
        *state.last.lock().unwrap() = Value::Null;
    }
    access["scope"] = json!(*scope);
    let _ = app.emit("scope-changed", &access);
    Ok(access)
}
#[tauri::command]
async fn get_view(
    state: State<'_, AppState>,
    view: String,
    mut query: Query,
    revision: u64,
) -> Result<Value, String> {
    let client = state
        .client
        .read()
        .await
        .clone()
        .ok_or("先连接你的 Keeper，即可查看用量")?;
    let scope = state.scope.read().await.clone();
    if revision != scope.revision {
        return Err("统计范围已更新".into());
    }
    // IPC callers cannot override the shared scope or request another key as a viewer.
    query.api_key_id = scope.api_key_id.clone();
    let result = client.view(&view, query).await;
    let current = state.client.read().await;
    if state.scope.read().await.revision != revision
        || !current.as_ref().is_some_and(|c| Arc::ptr_eq(c, &client))
    {
        return Err("连接或统计范围已更新".into());
    }
    result
}
#[tauri::command]
async fn open_console(
    window: WebviewWindow,
    state: State<'_, AppState>,
    target: String,
) -> Result<(), String> {
    if window.label() != "detail" {
        return Err("请通过面板打开控制台".into());
    }
    if target == "usage" {
        let endpoint = state.settings.lock().unwrap().endpoint.clone();
        let url = keeper_core::browser_url(&endpoint)?;
        return windows::open_browser(url.as_str());
    }
    if target != "cpa" {
        return Err("未知控制台".into());
    }
    let client = state.client.read().await.clone().ok_or("请先连接 Keeper")?;
    let url = client.console_url().await?;
    let current = state.client.read().await;
    if !current.as_ref().is_some_and(|c| Arc::ptr_eq(c, &client)) {
        return Err("连接已更新，请重新打开控制台".into());
    }
    let url = keeper_core::browser_url(&url)?;
    // Open the user's browser; never forward the in-app cookie, password or sk.
    windows::open_browser(url.as_str())
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
                if let Err(error) = window.start_dragging() {
                    state.hover.lock().unwrap().dragging = false;
                    let _ = window.emit("drag-finished", ());
                    return Err(error.to_string());
                }
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
                windows::show_inactive(&w);
            }
            windows::show_settings(app);
        }))
        .setup(|app| {
            let config = settings::load();
            let client = if config.endpoint.is_empty() {
                None
            } else {
                Keeper::connect(
                    &config.endpoint,
                    &config.password,
                    config.allow_private_http,
                    &config.proxy_url,
                    config.auth_mode,
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
                scope: RwLock::new(Scope::default()),
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
            get_access,
            set_scope,
            open_console,
            window_action
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window.label() == "detail" {
                    windows::hide_detail(window.app_handle());
                } else if window.label() == "widget" {
                    windows::hide(window.app_handle());
                } else {
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("无法启动 Keeper UsagePanel");
}
