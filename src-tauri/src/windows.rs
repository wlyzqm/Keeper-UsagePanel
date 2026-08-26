use crate::AppState;
use std::time::{Duration, Instant};
use tauri::{Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};
#[derive(Default)]
pub struct Hover {
    pub dragging: bool,
    pub suppressed: bool,
    entered: Option<Instant>,
    left: Option<Instant>,
}

pub fn create(app: &tauri::AppHandle) -> tauri::Result<()> {
    for (label, w, h) in [
        ("widget", 224., 112.),
        ("detail", 552., 712.),
        ("settings", 488., 684.),
    ] {
        WebviewWindowBuilder::new(
            app,
            label,
            WebviewUrl::App(format!("index.html?window={label}").into()),
        )
        .title("Keeper UsagePanel")
        .inner_size(w, h)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(label == "widget")
        .focused(false)
        .build()?;
    }
    let widget = app.get_webview_window("widget").unwrap();
    let config = app.state::<AppState>().settings.lock().unwrap().clone();
    if let (Some(x), Some(y)) = (config.x, config.y) {
        let _ = widget.set_position(PhysicalPosition::new(x, y));
    }
    let area = work_area(&widget);
    let size = widget.outer_size()?;
    let x = config
        .x
        .unwrap_or(area.2 - size.width as i32 - 24)
        .clamp(area.0, (area.2 - size.width as i32).max(area.0));
    let y = config
        .y
        .unwrap_or(area.1 + 180)
        .clamp(area.1, (area.3 - size.height as i32).max(area.1));
    widget.set_position(PhysicalPosition::new(x, y))?;
    use tauri::menu::{Menu, MenuItem};
    let show = MenuItem::with_id(app, "show", "显示悬浮球", true, None::<&str>)?;
    let setup = MenuItem::with_id(app, "settings", "连接设置", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "隐藏悬浮球", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &setup, &hide_item, &quit])?;
    tauri::tray::TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Keeper · 用量面板")
        .menu(&menu)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("widget") {
                    let _ = w.show();
                }
            }
            "settings" => show_settings(app),
            "hide" => hide(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
pub fn hide_detail(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("detail") {
        if w.is_visible().unwrap_or(false) {
            #[cfg(windows)]
            unsafe {
                if let Ok(hwnd) = w.hwnd() {
                    windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                        hwnd.0 as _,
                        windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE,
                    );
                }
            }
            #[cfg(not(windows))]
            let _ = w.hide();
            let _ = w.emit("detail-close", ());
        }
    }
}
pub fn hide(app: &tauri::AppHandle) {
    hide_detail(app);
    if let Some(w) = app.get_webview_window("widget") {
        let _ = w.hide();
    }
}
pub fn show_settings(app: &tauri::AppHandle) {
    hide_detail(app);
    if let Some(w) = app.get_webview_window("settings") {
        let scale = w.scale_factor().unwrap_or(1.);
        let area = work_area(&w);
        let _ = w.set_size(LogicalSize::new(
            488_f64.min((area.2 - area.0) as f64 / scale),
            684_f64.min((area.3 - area.1) as f64 / scale),
        ));
        let _ = w.center();
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.emit("settings-open", ());
    }
}
pub fn show_detail(app: &tauri::AppHandle) {
    let (Some(widget), Some(panel)) = (
        app.get_webview_window("widget"),
        app.get_webview_window("detail"),
    ) else {
        return;
    };
    if panel.is_visible().unwrap_or(false) {
        return;
    }
    let Ok(p) = widget.outer_position() else {
        return;
    };
    let Ok(size) = widget.outer_size() else {
        return;
    };
    let scale = widget.scale_factor().unwrap_or(1.);
    let area = work_area(&widget);
    let width = 552.;
    let height = 712_f64.min((area.3 - area.1) as f64 / scale);
    let pw = (width * scale) as i32;
    let ph = (height * scale) as i32;
    let right = p.x + size.width as i32 - 4;
    let left = p.x - pw + 4;
    let x =
        if right + pw <= area.2 { right } else { left }.clamp(area.0, (area.2 - pw).max(area.0));
    let y = (p.y - 20).clamp(area.1, (area.3 - ph).max(area.1));
    let _ = panel.set_position(PhysicalPosition::new(x, y));
    let _ = panel.set_size(LogicalSize::new(width, height));
    // Showing a hover panel must not steal keyboard focus from the user's app.
    #[cfg(windows)]
    unsafe {
        if let Ok(hwnd) = panel.hwnd() {
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                hwnd.0 as _,
                windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNOACTIVATE,
            );
        }
    }
    #[cfg(not(windows))]
    let _ = panel.show();
    let _ = panel.emit("detail-open", ());
}
fn work_area(window: &tauri::WebviewWindow) -> (i32, i32, i32, i32) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        };
        if let Ok(hwnd) = window.hwnd() {
            let monitor = MonitorFromWindow(hwnd.0 as _, MONITOR_DEFAULTTONEAREST);
            let mut info = std::mem::zeroed::<MONITORINFO>();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(monitor, &mut info) != 0 {
                return (
                    info.rcWork.left,
                    info.rcWork.top,
                    info.rcWork.right,
                    info.rcWork.bottom,
                );
            }
        }
    }
    if let Ok(Some(m)) = window.current_monitor() {
        let p = m.position();
        let s = m.size();
        (p.x, p.y, p.x + s.width as i32, p.y + s.height as i32)
    } else {
        (0, 0, 1920, 1080)
    }
}
pub fn session_locked() -> bool {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::StationsAndDesktops::{
            CloseDesktop, OpenInputDesktop, DESKTOP_READOBJECTS,
        };
        let d = OpenInputDesktop(0, 0, DESKTOP_READOBJECTS);
        if d.is_null() {
            return true;
        }
        CloseDesktop(d);
    }
    false
}
pub fn track(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(80));
        let Some(widget) = app.get_webview_window("widget") else {
            break;
        };
        if !widget.is_visible().unwrap_or(false) {
            continue;
        }
        let state = app.state::<AppState>();
        let dragging = state.hover.lock().unwrap().dragging;
        let Ok(cursor) = widget.cursor_position() else {
            continue;
        };
        if dragging {
            #[cfg(windows)]
            let down =
                unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(1) } < 0;
            #[cfg(not(windows))]
            let down = false;
            if down {
                continue;
            }
            {
                let mut hover = state.hover.lock().unwrap();
                hover.dragging = false;
                hover.suppressed = true;
            }
            if let (Ok(p), Ok(size)) = (widget.outer_position(), widget.outer_size()) {
                let area = work_area(&widget);
                let scale = widget.scale_factor().unwrap_or(1.);
                let gap = (8. * scale) as i32;
                let max_x = (area.2 - size.width as i32).max(area.0);
                let max_y = (area.3 - size.height as i32).max(area.1);
                let mut x = p.x.clamp(area.0, max_x);
                let y = p.y.clamp(area.1, max_y);
                if x - area.0 < 32 {
                    x = area.0 + gap;
                } else if max_x - x < 32 {
                    x = max_x - gap;
                }
                let _ = widget.set_position(PhysicalPosition::new(x, y));
                crate::settings::position(x, y);
                let mut config = state.settings.lock().unwrap();
                config.x = Some(x);
                config.y = Some(y);
            }
        }
        let inside = |w: &tauri::WebviewWindow, margin: f64| -> bool {
            if !w.is_visible().unwrap_or(false) {
                return false;
            }
            let (Ok(p), Ok(s)) = (w.outer_position(), w.outer_size()) else {
                return false;
            };
            let m = margin * w.scale_factor().unwrap_or(1.);
            cursor.x >= p.x as f64 + m
                && cursor.x <= (p.x + s.width as i32) as f64 - m
                && cursor.y >= p.y as f64 + m
                && cursor.y <= (p.y + s.height as i32) as f64 - m
        };
        let in_ball = inside(&widget, 10.);
        let panel = app.get_webview_window("detail");
        let in_panel = panel.as_ref().is_some_and(|w| inside(w, 10.));
        let bridge = panel.as_ref().is_some_and(|w| {
            if !w.is_visible().unwrap_or(false) {
                return false;
            }
            let (Ok(a), Ok(b), Ok(sa), Ok(sb)) = (
                widget.outer_position(),
                w.outer_position(),
                widget.outer_size(),
                w.outer_size(),
            ) else {
                return false;
            };
            let (left, right) = if a.x < b.x {
                (a.x + sa.width as i32 - 14, b.x + 14)
            } else {
                (b.x + sb.width as i32 - 14, a.x + 14)
            };
            cursor.x >= left as f64
                && cursor.x <= right as f64
                && cursor.y >= a.y.max(b.y) as f64
                && cursor.y <= (a.y + sa.height as i32).min(b.y + sb.height as i32) as f64
        });
        if app
            .get_webview_window("settings")
            .is_some_and(|w| w.is_visible().unwrap_or(false))
        {
            continue;
        }
        // Do not hold the hover mutex across calls dispatched to the UI thread.
        let (show, hide) = {
            let mut hover = state.hover.lock().unwrap();
            if in_ball || in_panel || bridge {
                hover.left = None;
                let show = !hover.suppressed
                    && hover.entered.get_or_insert_with(Instant::now).elapsed()
                        >= Duration::from_millis(180);
                (show, false)
            } else {
                hover.entered = None;
                hover.suppressed = false;
                (
                    false,
                    hover.left.get_or_insert_with(Instant::now).elapsed()
                        >= Duration::from_millis(320),
                )
            }
        };
        if show {
            show_detail(&app);
        } else if hide {
            hide_detail(&app);
        }
    });
}
