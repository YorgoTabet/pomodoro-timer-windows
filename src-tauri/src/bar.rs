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

/// A monitor's work area in physical pixels, with its scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

/// At least this much of the pill, in points, must be on a display to count as reachable.
const MIN_VISIBLE: f64 = 40.0;

/// Whether a pill whose top left is at (x, y), in physical pixels, still has a
/// usable part on some display. Sized with each display's own scale, so a bar saved
/// on a 150% monitor is judged at 150%. Pure so it can be tested without monitors.
pub fn reachable(x: f64, y: f64, areas: &[Area]) -> bool {
    areas.iter().any(|a| {
        let (w, h) = (WIDTH * a.scale, HEIGHT * a.scale);
        let (ax, ay) = (f64::from(a.x), f64::from(a.y));
        let overlap_w = (x + w).min(ax + f64::from(a.width)) - x.max(ax);
        let overlap_h = (y + h).min(ay + f64::from(a.height)) - y.max(ay);
        let need = MIN_VISIBLE * a.scale;
        overlap_w >= need.min(w) && overlap_h >= need.min(h)
    })
}

/// Upper right of the work area, in physical pixels.
fn default_position(a: &Area) -> (f64, f64) {
    let right = f64::from(a.x) + f64::from(a.width);
    (right - (WIDTH + EDGE_GAP) * a.scale, f64::from(a.y) + EDGE_GAP * a.scale)
}

fn area_of(m: &Monitor) -> Area {
    let w = m.work_area();
    Area { x: w.position.x, y: w.position.y, width: w.size.width, height: w.size.height, scale: m.scale_factor() }
}

/// The saved position is in physical pixels. Logical points depend on which monitor's
/// scale is used, and the bar can be saved on one monitor and restored on another.
fn start_position(app: &AppHandle) -> Option<(f64, f64)> {
    let settings = app.state::<Shared>().settings();
    let areas: Vec<Area> = app.available_monitors().unwrap_or_default().iter().map(area_of).collect();
    if let (Some(x), Some(y)) = (settings.floating_bar_x, settings.floating_bar_y) {
        if reachable(x, y, &areas) {
            return Some((x, y));
        }
    }
    app.primary_monitor().ok().flatten().map(|m| default_position(&area_of(&m)))
}

/// Make the always-on-top pill window and start saving where it is dragged to.
pub fn create(app: &AppHandle, title: &str) {
    let builder = WebviewWindowBuilder::new(app, "bar", WebviewUrl::App("/bar".into()))
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
    let builder = builder.transparent(true);
    // Placed in physical pixels after the build: the builder only takes logical ones.
    let start = start_position(app);
    let Ok(window) = builder.visible(start.is_none()).build() else { return };
    if let Some((x, y)) = start {
        let _ = window.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
        let _ = window.show();
    }

    let app = app.clone();
    let generation = Arc::new(AtomicU64::new(0));
    window.on_window_event(move |event| {
        let WindowEvent::Moved(PhysicalPosition { x, y }) = *event else { return };
        let (x, y) = (f64::from(x), f64::from(y));
        // Placing the window at launch is a move too, and is not a drag.
        if start.is_some_and(|(sx, sy)| (sx - x).abs() <= 1.0 && (sy - y).abs() <= 1.0) {
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

#[cfg(test)]
mod tests {
    use super::*;

    const A: Area = Area { x: 0, y: 0, width: 1920, height: 1080, scale: 1.0 };
    // A 150% monitor to the right.
    const B: Area = Area { x: 1920, y: 0, width: 2400, height: 1350, scale: 1.5 };

    #[test]
    fn fully_on_screen_is_reachable() {
        assert!(reachable(100.0, 100.0, &[A]));
    }

    #[test]
    fn mostly_off_screen_is_not() {
        // 30 of 236 points showing.
        assert!(!reachable(1890.0, 100.0, &[A]));
        assert!(!reachable(100.0, 1070.0, &[A]));
        assert!(!reachable(-300.0, 100.0, &[A]));
    }

    #[test]
    fn forty_points_showing_is_enough() {
        assert!(reachable(1920.0 - 40.0, 100.0, &[A]));
    }

    #[test]
    fn judged_at_the_monitors_own_scale() {
        // 50 physical pixels is 33 points on the 150% monitor: not enough there.
        assert!(!reachable(4320.0 - 50.0, 100.0, &[B]));
        assert!(reachable(4320.0 - 60.0, 100.0, &[B]));
    }

    #[test]
    fn a_disconnected_monitor_is_not_reachable() {
        assert!(!reachable(3000.0, 100.0, &[A]));
    }

    #[test]
    fn default_sits_in_the_top_right_at_the_monitors_scale() {
        assert_eq!(default_position(&A), (1920.0 - 260.0, 24.0));
        assert_eq!(default_position(&B), (4320.0 - 390.0, 36.0));
    }
}
