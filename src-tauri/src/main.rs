#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod settings;
mod updates;
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
fn valid_accent_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value.as_bytes()[1..]
            .iter()
            .all(|byte| byte.is_ascii_hexdigit())
}
fn same_connection(left: &settings::Settings, right: &settings::Settings) -> bool {
    left.endpoint == right.endpoint
        && left.auth_mode == right.auth_mode
        && left.password == right.password
        && left.proxy_url == right.proxy_url
        && left.allow_private_http == right.allow_private_http
        && left.allow_invalid_certificates == right.allow_invalid_certificates
}
pub(crate) fn quit(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(client) = app.state::<AppState>().client.read().await.clone() {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(1), client.logout()).await;
        }
        app.exit(0);
    });
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
    value.accent_color = value.accent_color.trim().to_ascii_lowercase();
    if !value.accent_color.is_empty() && !valid_accent_color(&value.accent_color) {
        return Err("主题色需为 #RRGGBB 格式".into());
    }
    let old_settings = state.settings.lock().unwrap().clone();
    if value.password.is_empty()
        && !clear_password
        && value.auth_mode == old_settings.auth_mode
        && value.endpoint.trim().trim_end_matches('/') == old_settings.endpoint
    {
        value.password = old_settings.password.clone()
    }
    value.x = old_settings.x;
    value.y = old_settings.y;
    value.proxy_url = value.proxy_url.trim().to_string();
    value.widget_font = value.widget_font.trim().to_string();
    if value.widget_font.is_empty() {
        value.widget_font = "HarmonyOS Sans SC".into();
    }
    value.endpoint = value.endpoint.trim().trim_end_matches('/').to_string();
    value.has_password = !value.password.is_empty();
    let reconnect = !same_connection(&value, &old_settings) || state.client.read().await.is_none();
    let client = if reconnect {
        let client = Arc::new(Keeper::connect_with_tls(
            &value.endpoint,
            &value.password,
            value.allow_private_http,
            &value.proxy_url,
            value.auth_mode,
            value.allow_invalid_certificates,
        )?);
        client.login().await?; // Verify before overwriting working credentials.
        Some(client)
    } else {
        None
    };
    settings::save(&value)?;
    let (revision, previous) = if let Some(client) = client {
        let mut current = state.client.write().await;
        let mut scope = state.scope.write().await;
        *scope = Scope {
            revision: scope.revision + 1,
            ..Default::default()
        };
        let revision = scope.revision;
        let previous = current.replace(client);
        *state.last.lock().unwrap() = Value::Null;
        (revision, previous)
    } else {
        (state.scope.read().await.revision, None)
    };
    *state.settings.lock().unwrap() = value.clone();
    windows::apply_behavior_settings(&app);
    let _ = app.emit(
        "configured",
        json!({"settings":value,"revision":revision,"connectionChanged":reconnect}),
    );
    let _ = window.hide();
    windows::show_widget(&app);
    if let Some(previous) = previous {
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), previous.logout()).await;
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
                windows::prepare_drag(&app, &window);
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
        "quit" => quit(&app),
        _ => return Err("未知窗口操作".into()),
    }
    Ok(())
}
#[tauri::command]
fn widget_edge_state(state: State<AppState>) -> windows::WidgetEdgeState {
    let result = windows::edge_state(&state.hover.lock().unwrap());
    result
}
fn main() {
    if updates::handle_update_helper_args() {
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            windows::show_widget(app);
            windows::show_settings(app);
        }))
        .setup(|app| {
            let config = settings::load();
            let client = if config.endpoint.is_empty() {
                None
            } else {
                Keeper::connect_with_tls(
                    &config.endpoint,
                    &config.password,
                    config.allow_private_http,
                    &config.proxy_url,
                    config.auth_mode,
                    config.allow_invalid_certificates,
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
            app.manage(updates::UpdateState::default());
            windows::create(app.handle())?;
            if need_config {
                windows::show_settings(app.handle());
            }
            windows::track(app.handle().clone());
            updates::track(app.handle().clone());
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
            window_action,
            widget_edge_state,
            updates::pending_update,
            updates::check_update,
            updates::skip_update,
            updates::install_update
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

#[cfg(test)]
mod tests {
    use super::{same_connection, valid_accent_color};
    use crate::settings::Settings;

    #[test]
    fn accent_color_accepts_only_complete_hex_colors() {
        assert!(valid_accent_color("#1756a9"));
        assert!(valid_accent_color("#ABCDEF"));
        assert!(!valid_accent_color("#123"));
        assert!(!valid_accent_color("1756a9"));
        assert!(!valid_accent_color("#gggggg"));
    }

    #[test]
    fn only_connection_fields_require_a_new_keeper_session() {
        let original = Settings {
            endpoint: "https://keeper.example/usage".into(),
            password: "secret".into(),
            ..Default::default()
        };
        let mut appearance = original.clone();
        appearance.theme = "dark".into();
        appearance.poll_seconds = 15;
        assert!(same_connection(&original, &appearance));
        let mut changed = original.clone();
        changed.proxy_url = "socks5://127.0.0.1:1080".into();
        assert!(!same_connection(&original, &changed));
    }
}
