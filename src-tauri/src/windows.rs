use crate::AppState;
use std::time::{Duration, Instant};
use tauri::{Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

const WIDGET_WIDTH: f64 = 216.;
const WIDGET_HEIGHT: f64 = 74.;
const EDGE_WIDGET_WIDTH: f64 = 34.;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
struct Dock {
    side: EdgeSide,
    boundary: i32,
    expanded: bool,
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct WidgetEdgeState {
    side: Option<EdgeSide>,
    collapsed: bool,
}

#[derive(Default)]
pub struct Hover {
    pub dragging: bool,
    pub suppressed: bool,
    entered: Option<Instant>,
    left: Option<Instant>,
    dock: Option<Dock>,
    fullscreen_hidden: bool,
    fullscreen_checked: Option<Instant>,
}

impl Hover {
    fn update(&mut self, inside: bool, now: Instant) -> (bool, bool) {
        if inside {
            self.left = None;
            (
                !self.suppressed
                    && now.duration_since(*self.entered.get_or_insert(now))
                        >= Duration::from_millis(180),
                false,
            )
        } else {
            self.entered = None;
            self.suppressed = false;
            (
                false,
                now.duration_since(*self.left.get_or_insert(now)) >= Duration::from_millis(320),
            )
        }
    }
}

pub fn edge_state(hover: &Hover) -> WidgetEdgeState {
    WidgetEdgeState {
        side: hover.dock.map(|dock| dock.side),
        collapsed: hover.dock.is_some_and(|dock| !dock.expanded),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DisplayRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

fn edge_is_exposed(
    displays: &[DisplayRect],
    side: EdgeSide,
    boundary: i32,
    top: i32,
    bottom: i32,
) -> bool {
    !displays.iter().any(|display| {
        let vertical_overlap = display.top < bottom && display.bottom > top;
        let reaches_across = match side {
            EdgeSide::Left => display.left < boundary && display.right >= boundary - 2,
            EdgeSide::Right => display.right > boundary && display.left <= boundary + 2,
        };
        vertical_overlap && reaches_across
    })
}

fn dock_side(
    window_left: i32,
    window_right: i32,
    area_left: i32,
    area_right: i32,
    threshold: i32,
) -> Option<EdgeSide> {
    let near_left = window_left <= area_left + threshold;
    let near_right = window_right >= area_right - threshold;
    match (near_left, near_right) {
        (true, false) => Some(EdgeSide::Left),
        (false, true) => Some(EdgeSide::Right),
        (true, true) => {
            if i64::from(window_left) + i64::from(window_right)
                <= i64::from(area_left) + i64::from(area_right)
            {
                Some(EdgeSide::Left)
            } else {
                Some(EdgeSide::Right)
            }
        }
        (false, false) => None,
    }
}

fn point_inside(rect: DisplayRect, x: f64, y: f64, margin: f64) -> bool {
    x >= rect.left as f64 + margin
        && x <= rect.right as f64 - margin
        && y >= rect.top as f64 + margin
        && y <= rect.bottom as f64 - margin
}

fn rect_covers_display(rect: DisplayRect, display: DisplayRect, tolerance: i32) -> bool {
    rect.left <= display.left + tolerance
        && rect.top <= display.top + tolerance
        && rect.right >= display.right - tolerance
        && rect.bottom >= display.bottom - tolerance
}

fn dock_strip_contains(dock: Dock, top: i32, bottom: i32, scale: f64, x: f64, y: f64) -> bool {
    let width = (EDGE_WIDGET_WIDTH * scale).round() as i32;
    let (left, right) = match dock.side {
        EdgeSide::Left => (dock.boundary, dock.boundary + width),
        EdgeSide::Right => (dock.boundary - width, dock.boundary),
    };
    point_inside(
        DisplayRect {
            left,
            top,
            right,
            bottom,
        },
        x,
        y,
        0.,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hover_recovers_after_suppression_and_long_idle() {
        let mut hover = Hover::default();
        let now = Instant::now();
        assert_eq!(hover.update(true, now), (false, false));
        assert_eq!(
            hover.update(true, now + Duration::from_millis(180)),
            (true, false)
        );
        hover.suppressed = true;
        assert_eq!(
            hover.update(true, now + Duration::from_secs(1)),
            (false, false)
        );
        assert_eq!(
            hover.update(false, now + Duration::from_secs(2)),
            (false, false)
        );
        assert_eq!(
            hover.update(false, now + Duration::from_secs(3)),
            (false, true)
        );
        let later = now + Duration::from_secs(86400);
        assert_eq!(hover.update(true, later), (false, false));
        assert_eq!(
            hover.update(true, later + Duration::from_millis(180)),
            (true, false)
        );
    }

    #[test]
    fn only_exterior_horizontal_edges_can_collapse() {
        let primary = DisplayRect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        assert!(edge_is_exposed(&[primary], EdgeSide::Left, 0, 200, 274));
        assert!(edge_is_exposed(&[primary], EdgeSide::Right, 1920, 200, 274));

        let right = DisplayRect {
            left: 1920,
            top: 0,
            right: 3840,
            bottom: 1080,
        };
        assert!(!edge_is_exposed(
            &[primary, right],
            EdgeSide::Right,
            1920,
            200,
            274
        ));
        assert!(!edge_is_exposed(
            &[primary, right],
            EdgeSide::Left,
            1920,
            200,
            274
        ));
    }

    #[test]
    fn staggered_monitors_only_block_collapse_where_they_join() {
        let primary = DisplayRect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let lower_right = DisplayRect {
            left: 1920,
            top: 500,
            right: 3840,
            bottom: 1580,
        };
        assert!(edge_is_exposed(
            &[primary, lower_right],
            EdgeSide::Right,
            1920,
            200,
            274
        ));
        assert!(!edge_is_exposed(
            &[primary, lower_right],
            EdgeSide::Right,
            1920,
            700,
            774
        ));
    }

    #[test]
    fn a_vertical_taskbar_is_not_treated_as_the_physical_screen_edge() {
        let display = DisplayRect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        assert!(!edge_is_exposed(&[display], EdgeSide::Left, 48, 200, 274));
    }

    #[test]
    fn a_window_thrown_past_either_edge_still_docks() {
        assert_eq!(dock_side(-420, -204, 0, 1920, 32), Some(EdgeSide::Left));
        assert_eq!(dock_side(2200, 2416, 0, 1920, 32), Some(EdgeSide::Right));
        assert_eq!(dock_side(500, 716, 0, 1920, 32), None);
    }

    #[test]
    fn an_expanded_dock_keeps_the_outermost_screen_pixel_hoverable() {
        let expanded = DisplayRect {
            left: 1704,
            top: 200,
            right: 1920,
            bottom: 274,
        };
        let dock = Dock {
            side: EdgeSide::Right,
            boundary: 1920,
            expanded: true,
        };
        assert!(!point_inside(expanded, 1919., 230., 8.));
        assert!(dock_strip_contains(dock, 200, 274, 1., 1919., 230.));
    }

    #[test]
    fn fullscreen_geometry_covers_the_complete_monitor_not_just_the_work_area() {
        let display = DisplayRect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        assert!(rect_covers_display(display, display, 2));
        assert!(rect_covers_display(
            DisplayRect {
                left: -1,
                top: -1,
                right: 1921,
                bottom: 1081,
            },
            display,
            2,
        ));
        assert!(!rect_covers_display(
            DisplayRect {
                bottom: 1040,
                ..display
            },
            display,
            2,
        ));
    }
}

pub fn create(app: &tauri::AppHandle) -> tauri::Result<()> {
    for (label, w, h) in [
        ("widget", WIDGET_WIDTH, WIDGET_HEIGHT),
        ("detail", 640., 640.),
        ("settings", 488., 640.),
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
        .always_on_top(label != "settings")
        .skip_taskbar(label != "settings")
        .resizable(false)
        .visible(label == "widget")
        .focused(false)
        .build()?;
    }
    let widget = app.get_webview_window("widget").unwrap();
    let config = app.state::<AppState>().settings.lock().unwrap().clone();
    let restore_edge = config.x.is_some();
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
    // Restore an old snapped position; a first-run default remains expanded.
    if restore_edge && config.edge_auto_collapse {
        dock_if_near(app, &widget, 12.);
    }
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
                show_widget(app);
            }
            "settings" => show_settings(app),
            "hide" => hide(app),
            "quit" => crate::quit(app),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
// Tauri/Tao visibility flags are not updated by native SW_SHOWNOACTIVATE.
// Use the actual HWND state for both show/hide and pointer tracking.
pub fn visible(w: &tauri::WebviewWindow) -> bool {
    #[cfg(windows)]
    unsafe {
        return w.hwnd().is_ok_and(|hwnd| {
            windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd.0 as _) != 0
        });
    }
    #[cfg(not(windows))]
    w.is_visible().unwrap_or(false)
}
fn foreground(w: &tauri::WebviewWindow) -> bool {
    #[cfg(windows)]
    unsafe {
        return w.hwnd().is_ok_and(|hwnd| {
            windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() == hwnd.0 as _
        });
    }
    #[cfg(not(windows))]
    w.is_focused().unwrap_or(false)
}
pub fn show_inactive(w: &tauri::WebviewWindow) {
    #[cfg(windows)]
    unsafe {
        if let Ok(hwnd) = w.hwnd() {
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                hwnd.0 as _,
                windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNOACTIVATE,
            );
        }
    }
    #[cfg(not(windows))]
    let _ = w.show();
}
pub fn show_widget(app: &tauri::AppHandle) {
    app.state::<AppState>()
        .hover
        .lock()
        .unwrap()
        .fullscreen_hidden = false;
    if let Some(widget) = app.get_webview_window("widget") {
        show_inactive(&widget);
    }
}
fn hide_native(w: &tauri::WebviewWindow) {
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
}
pub fn hide_detail(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("detail") {
        if visible(&w) {
            hide_native(&w);
            let _ = w.emit("detail-close", ());
        }
    }
}
pub fn hide(app: &tauri::AppHandle) {
    app.state::<AppState>()
        .hover
        .lock()
        .unwrap()
        .fullscreen_hidden = false;
    hide_detail(app);
    if let Some(w) = app.get_webview_window("widget") {
        hide_native(&w);
    }
}
pub fn show_settings(app: &tauri::AppHandle) {
    hide_detail(app);
    if let Some(w) = app.get_webview_window("settings") {
        if visible(&w) {
            // Explicit user action may focus it, but must not reset an in-progress password entry.
            let _ = w.set_focus();
            return;
        }
        let scale = w.scale_factor().unwrap_or(1.);
        let area = work_area(&w);
        let _ = w.set_size(LogicalSize::new(
            488_f64.min((area.2 - area.0) as f64 / scale),
            640_f64.min((area.3 - area.1) as f64 / scale),
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
    expand_docked(app, &widget);
    if visible(&panel) {
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
    let width = 640_f64.min((area.2 - area.0) as f64 / scale);
    let height = 640_f64.min((area.3 - area.1) as f64 / scale);
    let pw = (width * scale) as i32;
    let ph = (height * scale) as i32;
    let overlap = (4. * scale) as i32;
    let right = p.x + size.width as i32 - overlap;
    let left = p.x - pw + overlap;
    let x =
        if right + pw <= area.2 { right } else { left }.clamp(area.0, (area.2 - pw).max(area.0));
    let y = (p.y - (8. * scale) as i32).clamp(area.1, (area.3 - ph).max(area.1));
    let _ = panel.set_position(PhysicalPosition::new(x, y));
    let _ = panel.set_size(LogicalSize::new(width, height));
    app.state::<AppState>().hover.lock().unwrap().left = None;
    show_inactive(&panel);
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

fn display_rects(window: &tauri::WebviewWindow) -> Vec<DisplayRect> {
    window
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            DisplayRect {
                left: position.x,
                top: position.y,
                right: position.x + size.width as i32,
                bottom: position.y + size.height as i32,
            }
        })
        .collect()
}

fn emit_edge(window: &tauri::WebviewWindow, state: WidgetEdgeState) {
    let _ = window.emit("widget-edge", state);
}

fn collapse_docked(app: &tauri::AppHandle, widget: &tauri::WebviewWindow) {
    if !app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .edge_auto_collapse
    {
        return;
    }
    let state = app.state::<AppState>();
    let Some(dock) = state.hover.lock().unwrap().dock else {
        return;
    };
    if !dock.expanded {
        return;
    }
    let Ok(position) = widget.outer_position() else {
        return;
    };
    let scale = widget.scale_factor().unwrap_or(1.);
    let collapsed_width = (EDGE_WIDGET_WIDTH * scale).round() as i32;
    let x = match dock.side {
        EdgeSide::Left => dock.boundary,
        EdgeSide::Right => dock.boundary - collapsed_width,
    };
    if widget
        .set_size(LogicalSize::new(EDGE_WIDGET_WIDTH, WIDGET_HEIGHT))
        .is_err()
    {
        return;
    }
    let _ = widget.set_position(PhysicalPosition::new(x, position.y));
    let next = {
        let mut hover = state.hover.lock().unwrap();
        if let Some(current) = hover.dock.as_mut() {
            current.expanded = false;
        }
        edge_state(&hover)
    };
    emit_edge(widget, next);
}

fn expand_docked(app: &tauri::AppHandle, widget: &tauri::WebviewWindow) {
    let state = app.state::<AppState>();
    let Some(dock) = state.hover.lock().unwrap().dock else {
        return;
    };
    if dock.expanded {
        return;
    }
    let Ok(position) = widget.outer_position() else {
        return;
    };
    let scale = widget.scale_factor().unwrap_or(1.);
    let full_width = (WIDGET_WIDTH * scale).round() as i32;
    let x = match dock.side {
        EdgeSide::Left => dock.boundary,
        EdgeSide::Right => dock.boundary - full_width,
    };
    if widget
        .set_size(LogicalSize::new(WIDGET_WIDTH, WIDGET_HEIGHT))
        .is_err()
    {
        return;
    }
    let _ = widget.set_position(PhysicalPosition::new(x, position.y));
    let next = {
        let mut hover = state.hover.lock().unwrap();
        if let Some(current) = hover.dock.as_mut() {
            current.expanded = true;
        }
        hover.entered = None;
        edge_state(&hover)
    };
    emit_edge(widget, next);
}

fn clear_dock(app: &tauri::AppHandle, widget: &tauri::WebviewWindow) {
    expand_docked(app, widget);
    let state = app.state::<AppState>();
    let changed = state.hover.lock().unwrap().dock.take().is_some();
    if changed {
        emit_edge(widget, edge_state(&state.hover.lock().unwrap()));
    }
}

pub fn prepare_drag(app: &tauri::AppHandle, widget: &tauri::WebviewWindow) {
    expand_docked(app, widget);
    let state = app.state::<AppState>();
    let changed = {
        let mut hover = state.hover.lock().unwrap();
        let changed = hover.dock.take().is_some();
        hover.entered = None;
        hover.left = None;
        changed
    };
    if changed {
        emit_edge(widget, edge_state(&state.hover.lock().unwrap()));
    }
}

fn dock_if_near(app: &tauri::AppHandle, widget: &tauri::WebviewWindow, threshold_dip: f64) -> bool {
    let edge_auto_collapse = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .edge_auto_collapse;
    if !edge_auto_collapse {
        clear_dock(app, widget);
    }
    let (Ok(position), Ok(size)) = (widget.outer_position(), widget.outer_size()) else {
        return false;
    };
    let area = work_area(widget);
    let scale = widget.scale_factor().unwrap_or(1.);
    let threshold = (threshold_dip * scale).round() as i32;
    let side = edge_auto_collapse
        .then(|| {
            dock_side(
                position.x,
                position.x + size.width as i32,
                area.0,
                area.2,
                threshold,
            )
        })
        .flatten();
    let max_x = (area.2 - size.width as i32).max(area.0);
    let max_y = (area.3 - size.height as i32).max(area.1);
    let y = position.y.clamp(area.1, max_y);
    let displays = display_rects(widget);
    if let Some(side) = side {
        let boundary = match side {
            EdgeSide::Left => area.0,
            EdgeSide::Right => area.2,
        };
        // An unavailable monitor list must never turn a multi-screen seam into a dock target.
        if !displays.is_empty()
            && edge_is_exposed(&displays, side, boundary, y, y + size.height as i32)
        {
            let full_width = (WIDGET_WIDTH * scale).round() as i32;
            let x = match side {
                EdgeSide::Left => boundary,
                EdgeSide::Right => boundary - full_width,
            };
            let _ = widget.set_position(PhysicalPosition::new(x, y));
            app.state::<AppState>().hover.lock().unwrap().dock = Some(Dock {
                side,
                boundary,
                expanded: true,
            });
            collapse_docked(app, widget);
            return true;
        }
    }
    let x = position.x.clamp(area.0, max_x);
    let _ = widget.set_position(PhysicalPosition::new(x, y));
    false
}

fn persist_widget_position(app: &tauri::AppHandle, widget: &tauri::WebviewWindow) {
    let Ok(position) = widget.outer_position() else {
        return;
    };
    let scale = widget.scale_factor().unwrap_or(1.);
    let dock = app.state::<AppState>().hover.lock().unwrap().dock;
    let x = dock.map_or(position.x, |dock| match dock.side {
        EdgeSide::Left => dock.boundary,
        EdgeSide::Right => dock.boundary - (WIDGET_WIDTH * scale).round() as i32,
    });
    crate::settings::position(x, position.y);
    let state = app.state::<AppState>();
    let mut config = state.settings.lock().unwrap();
    config.x = Some(x);
    config.y = Some(position.y);
}

fn finish_drag(app: &tauri::AppHandle, widget: &tauri::WebviewWindow) {
    dock_if_near(app, widget, 32.);
    persist_widget_position(app, widget);
}

#[cfg(windows)]
fn foreground_is_fullscreen(app: &tauri::AppHandle) -> bool {
    unsafe {
        use windows_sys::Win32::{
            Foundation::RECT,
            Graphics::Gdi::{
                GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONULL,
            },
            UI::WindowsAndMessaging::{
                GetDesktopWindow, GetForegroundWindow, GetShellWindow, GetWindowRect, IsIconic,
            },
        };
        let foreground = GetForegroundWindow();
        if foreground.is_null()
            || foreground == GetDesktopWindow()
            || foreground == GetShellWindow()
            || IsIconic(foreground) != 0
        {
            return false;
        }
        if ["widget", "detail", "settings"].into_iter().any(|label| {
            app.get_webview_window(label)
                .and_then(|window| window.hwnd().ok())
                .is_some_and(|hwnd| hwnd.0 as isize == foreground as isize)
        }) {
            return false;
        }
        let monitor = MonitorFromWindow(foreground, MONITOR_DEFAULTTONULL);
        if monitor.is_null() {
            return false;
        }
        let mut info = std::mem::zeroed::<MONITORINFO>();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let mut rect = std::mem::zeroed::<RECT>();
        if GetMonitorInfoW(monitor, &mut info) == 0 || GetWindowRect(foreground, &mut rect) == 0 {
            return false;
        }
        rect_covers_display(
            DisplayRect {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            },
            DisplayRect {
                left: info.rcMonitor.left,
                top: info.rcMonitor.top,
                right: info.rcMonitor.right,
                bottom: info.rcMonitor.bottom,
            },
            2,
        )
    }
}

#[cfg(not(windows))]
fn foreground_is_fullscreen(_: &tauri::AppHandle) -> bool {
    false
}

pub fn apply_behavior_settings(app: &tauri::AppHandle) {
    let (edge_auto_collapse, fullscreen_auto_hide) = {
        let settings = app.state::<AppState>();
        let settings = settings.settings.lock().unwrap();
        (settings.edge_auto_collapse, settings.fullscreen_auto_hide)
    };
    let Some(widget) = app.get_webview_window("widget") else {
        return;
    };
    if edge_auto_collapse {
        dock_if_near(app, &widget, 12.);
    } else {
        clear_dock(app, &widget);
    }
    if !fullscreen_auto_hide {
        let should_restore = {
            let state = app.state::<AppState>();
            let mut hover = state.hover.lock().unwrap();
            std::mem::take(&mut hover.fullscreen_hidden)
        };
        if should_restore {
            show_inactive(&widget);
        }
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
        let state = app.state::<AppState>();
        let now = Instant::now();
        let check_fullscreen = {
            let mut hover = state.hover.lock().unwrap();
            if hover
                .fullscreen_checked
                .is_some_and(|checked| now.duration_since(checked) < Duration::from_millis(320))
            {
                false
            } else {
                hover.fullscreen_checked = Some(now);
                true
            }
        };
        if check_fullscreen {
            let enabled = state.settings.lock().unwrap().fullscreen_auto_hide;
            let fullscreen = enabled && foreground_is_fullscreen(&app);
            let hidden_for_fullscreen = state.hover.lock().unwrap().fullscreen_hidden;
            if fullscreen && !hidden_for_fullscreen && visible(&widget) {
                hide_detail(&app);
                hide_native(&widget);
                state.hover.lock().unwrap().fullscreen_hidden = true;
                continue;
            }
            if !fullscreen && hidden_for_fullscreen {
                state.hover.lock().unwrap().fullscreen_hidden = false;
                show_inactive(&widget);
                if state.hover.lock().unwrap().update_prompt {
                    show_detail(&app);
                }
            }
        }
        if !visible(&widget) {
            continue;
        }
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
                hover.entered = None;
            }
            let _ = widget.emit("drag-finished", ());
            finish_drag(&app, &widget);
        }
        let inside = |w: &tauri::WebviewWindow, margin: f64| -> bool {
            if !visible(&w) {
                return false;
            }
            let (Ok(p), Ok(s)) = (w.outer_position(), w.outer_size()) else {
                return false;
            };
            let m = margin * w.scale_factor().unwrap_or(1.);
            point_inside(
                DisplayRect {
                    left: p.x,
                    top: p.y,
                    right: p.x + s.width as i32,
                    bottom: p.y + s.height as i32,
                },
                cursor.x,
                cursor.y,
                m,
            )
        };
        let dock = state.hover.lock().unwrap().dock;
        let in_dock_strip = dock.is_some_and(|dock| {
            let (Ok(position), Ok(size)) = (widget.outer_position(), widget.outer_size()) else {
                return false;
            };
            dock_strip_contains(
                dock,
                position.y,
                position.y + size.height as i32,
                widget.scale_factor().unwrap_or(1.),
                cursor.x,
                cursor.y,
            )
        });
        let in_ball = inside(&widget, 8.) || in_dock_strip;
        if dock.is_some_and(|dock| !dock.expanded) {
            let suppressed = state.hover.lock().unwrap().suppressed;
            if !in_ball {
                state.hover.lock().unwrap().suppressed = false;
            } else if !suppressed {
                expand_docked(&app, &widget);
            }
            continue;
        }
        let panel = app.get_webview_window("detail");
        let in_panel = panel.as_ref().is_some_and(|w| inside(w, 8.));
        let bridge = panel.as_ref().is_some_and(|w| {
            if !visible(&w) {
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
            let margin = (12. * widget.scale_factor().unwrap_or(1.)) as i32;
            let (left, right) = if a.x < b.x {
                (a.x + sa.width as i32 - margin, b.x + margin)
            } else {
                (b.x + sb.width as i32 - margin, a.x + margin)
            };
            cursor.x >= left as f64
                && cursor.x <= right as f64
                && cursor.y >= a.y.max(b.y) as f64
                && cursor.y <= (a.y + sa.height as i32).min(b.y + sb.height as i32) as f64
        });
        if app
            .get_webview_window("settings")
            .is_some_and(|w| foreground(&w))
        {
            continue;
        }
        // Do not hold the hover mutex across calls dispatched to the UI thread.
        let (show, hide) = state
            .hover
            .lock()
            .unwrap()
            .update(in_ball || in_panel || bridge, Instant::now());
        if show {
            show_detail(&app);
        } else if hide {
            hide_detail(&app);
            collapse_docked(&app, &widget);
        }
    });
}

pub fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Com::{
            CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
        };
        let initialized = CoInitializeEx(
            std::ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as _,
        );
        if initialized < 0 {
            return Err("无法初始化 Windows 浏览器打开组件".into());
        }
        let verb: Vec<u16> = "open\0".encode_utf16().collect();
        let target: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
        let result = windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        );
        CoUninitialize();
        if result as isize > 32 {
            return Ok(());
        }
        return Err("无法打开默认浏览器，请检查 Windows 默认应用设置".into());
    }
    #[cfg(not(windows))]
    {
        let _ = url;
        Err("请在 Windows 桌面程序中打开控制台".into())
    }
}
