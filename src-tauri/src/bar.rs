use crate::timer::Shared;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tauri::{
    AppHandle, Emitter, Manager, Monitor, PhysicalPosition, WebviewUrl,
    WebviewWindowBuilder, WindowEvent,
};

/// The pill's size in points, from the Swift `FloatingBar.Metrics`: the row adds up
/// to 236 wide and 46 high. The window is exactly the pill for now; the characters
/// task adds their transparent margin around it.
pub const WIDTH: f64 = 236.0;
pub const HEIGHT: f64 = 46.0;

/// Gap kept to the screen's top right corner by the default placement.
const EDGE_GAP: f64 = 24.0;

/// How long the bar must sit still before its position is saved. A drag fires a
/// move event per pixel, and a write for each would hammer the disk.
const SAVE_AFTER: Duration = Duration::from_millis(400);

/// Where the pill goes on first launch: upper right of the primary work area,
/// clear of the taskbar and the top edge.
fn default_position(monitor: &Monitor) -> (f64, f64) {
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let right = (f64::from(area.position.x) + f64::from(area.size.width)) / scale;
    let top = f64::from(area.position.y) / scale;
    (right - WIDTH - EDGE_GAP, top + EDGE_GAP)
}

/// A saved position only counts if it still lands on a connected display.
fn on_a_display(app: &AppHandle, x: f64, y: f64) -> bool {
    app.available_monitors().unwrap_or_default().iter().any(|m| {
        let scale = m.scale_factor();
        let (left, top) = (f64::from(m.position().x) / scale, f64::from(m.position().y) / scale);
        let (w, h) = (f64::from(m.size().width) / scale, f64::from(m.size().height) / scale);
        x >= left - 1.0 && x <= left + w + 1.0 && y >= top - 1.0 && y <= top + h + 1.0
    })
}

fn start_position(app: &AppHandle) -> Option<(f64, f64)> {
    let settings = app.state::<Shared>().settings();
    if let (Some(x), Some(y)) = (settings.floating_bar_x, settings.floating_bar_y) {
        if on_a_display(app, x, y) {
            return Some((x, y));
        }
    }
    app.primary_monitor().ok().flatten().map(|m| default_position(&m))
}

/// Make the always-on-top pill window and start saving where it is dragged to.
pub fn create(app: &AppHandle, title: &str) {
    let mut builder = WebviewWindowBuilder::new(app, "bar", WebviewUrl::App("/bar".into()))
        .title(title)
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .shadow(false)
        .focused(false);
    // macOS only allows a see-through window behind a private API switch. The app
    // ships on Windows, so the Mac build just gets a solid backdrop for development.
    #[cfg(not(target_os = "macos"))]
    {
        builder = builder.transparent(true);
    }
    let start = start_position(app);
    if let Some((x, y)) = start {
        builder = builder.position(x, y);
    }
    let Ok(window) = builder.build() else { return };

    let app = app.clone();
    let generation = Arc::new(AtomicU64::new(0));
    window.on_window_event(move |event| {
        let WindowEvent::Moved(PhysicalPosition { x, y }) = *event else { return };
        let Some(window) = app.get_webview_window("bar") else { return };
        let scale = window.scale_factor().unwrap_or(1.0);
        let (x, y) = (f64::from(x) / scale, f64::from(y) / scale);
        // Placing the window at launch is a move too, and is not a drag.
        if start.is_some_and(|(sx, sy)| (sx - x).abs() < 1.0 && (sy - y).abs() < 1.0) {
            return;
        }
        let mine = generation.fetch_add(1, Ordering::SeqCst) + 1;
        let (app, generation) = (app.clone(), generation.clone());
        thread::spawn(move || {
            thread::sleep(SAVE_AFTER);
            // A newer move arrived, so the drag is still going.
            if generation.load(Ordering::SeqCst) != mine {
                return;
            }
            if let Ok(saved) = app.state::<Shared>().set_bar_position(x, y) {
                // Other windows hold a copy and would write the old position back.
                let _ = app.emit("settings-changed", &saved);
            }
        });
    });
}
