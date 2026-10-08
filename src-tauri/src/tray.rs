use crate::{
    core::phase::Phase,
    core::settings::Settings,
    timer::{self, Shared, TimerState},
};
use std::sync::{Mutex, OnceLock};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, WebviewUrl, WebviewWindowBuilder,
};

const TRAY_ID: &str = "main";

/// The menu items that change after the menu is built.
struct Items {
    header: MenuItem<tauri::Wry>,
    toggle: MenuItem<tauri::Wry>,
    bar: MenuItem<tauri::Wry>,
}

static ITEMS: OnceLock<Items> = OnceLock::new();

/// mm:ss, never negative.
pub fn display_time(remaining_seconds: i64) -> String {
    let clamped = remaining_seconds.max(0);
    format!("{:02}:{:02}", clamped / 60, clamped % 60)
}

/// Windows tray icons cannot show text, so the countdown lives in the tooltip and
/// in the menu's first line.
pub fn header_text(phase: Phase, is_running: bool, remaining_seconds: i64) -> String {
    let time = display_time(remaining_seconds);
    if is_running {
        format!("{} · {}", phase.title(), time)
    } else {
        format!("{} · {} · paused", phase.title(), time)
    }
}

pub fn tooltip_text(phase: Phase, remaining_seconds: i64) -> String {
    format!("{} — {} remaining", phase.title(), display_time(remaining_seconds))
}

pub fn build(app: &AppHandle, state: &TimerState, show_bar: bool) -> tauri::Result<()> {
    let header = MenuItem::with_id(app, "header", header_text(state.phase, state.is_running, state.remaining_seconds), false, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", toggle_label(state.is_running), true, None::<&str>)?;
    let skip = MenuItem::with_id(app, "skip", "Skip", true, None::<&str>)?;
    let reset = MenuItem::with_id(app, "reset", "Reset", true, None::<&str>)?;
    let bar = MenuItem::with_id(app, "bar", bar_label(show_bar), true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let stats = MenuItem::with_id(app, "stats", "Statistics…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Pomodoro", true, None::<&str>)?;
    let sep = || PredefinedMenuItem::separator(app);

    let menu = Menu::with_items(
        app,
        &[&header, &sep()?, &toggle, &skip, &reset, &sep()?, &bar, &stats, &settings, &sep()?, &quit],
    )?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip(tooltip_text(state.phase, state.remaining_seconds))
        .on_menu_event(|app, event| {
            let app = app.clone();
            let id = event.id().as_ref().to_string();
            // Off the main thread: the handlers wait on the publish lock, and the
            // thread holding it may be waiting on the main thread to update the menu.
            std::thread::spawn(move || match id.as_str() {
                "toggle" => timer::apply(&app, |c, s, now| {
                    c.toggle(now, s);
                    None
                }),
                "skip" => timer::apply(&app, |c, s, _| Some(c.skip(s))),
                "reset" => timer::apply(&app, |c, s, _| {
                    c.reset(s);
                    None
                }),
                "bar" => {
                    let mut settings = app.state::<Shared>().settings();
                    settings.show_floating_bar = !settings.show_floating_bar;
                    let _ = timer::commit_settings(&app, settings);
                }
                "settings" => open_window(&app, "settings", "Pomodoro Settings"),
                "stats" => open_window(&app, "stats", "Statistics"),
                "quit" => app.exit(0),
                _ => {}
            });
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;

    let _ = ITEMS.set(Items { header, toggle, bar });
    Ok(())
}

/// The newest state the tray should show. Written under the publish lock, read on
/// the main thread, so whichever update runs last always draws the latest state.
static LATEST: Mutex<Option<TimerState>> = Mutex::new(None);

/// Called on every published change, so the tooltip and header follow the clock.
/// Never blocks: the tray is touched on the main thread, later, and the caller may
/// be holding the publish lock.
pub fn refresh(app: &AppHandle, state: &TimerState) {
    *LATEST.lock().unwrap_or_else(|e| e.into_inner()) = Some(state.clone());
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(state) = LATEST.lock().unwrap_or_else(|e| e.into_inner()).clone() else {
            return;
        };
        if let Some(tray) = handle.tray_by_id(TRAY_ID) {
            let _ = tray.set_tooltip(Some(tooltip_text(state.phase, state.remaining_seconds)));
        }
        if let Some(items) = ITEMS.get() {
            let _ = items.header.set_text(header_text(state.phase, state.is_running, state.remaining_seconds));
            let _ = items.toggle.set_text(toggle_label(state.is_running));
        }
    });
}

fn bar_label(show: bool) -> &'static str {
    if show {
        "Hide Floating Bar"
    } else {
        "Show Floating Bar"
    }
}

/// Make the bar window and the menu label match `showFloatingBar`.
pub fn sync_bar(app: &AppHandle, settings: &Settings) {
    if let Some(items) = ITEMS.get() {
        let _ = items.bar.set_text(bar_label(settings.show_floating_bar));
    }
    if settings.show_floating_bar {
        open_window(app, "bar", "Pomodoro");
    } else if let Some(window) = app.get_webview_window("bar") {
        // Hidden, not closed: closing is asynchronous and races with a quick re-show.
        let _ = window.hide();
    }
}

fn toggle_label(is_running: bool) -> &'static str {
    if is_running {
        "Pause"
    } else {
        "Start"
    }
}

/// Focus the window if it is open, otherwise create it. The page at `/{label}`
/// is built by the task that owns that window.
pub fn open_window(app: &AppHandle, label: &'static str, title: &'static str) {
    let app = app.clone();
    // Off the menu callback: building a window from inside it can stall on Windows.
    std::thread::spawn(move || {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.show();
            // The bar never takes focus: it sits over whatever you are working in.
            if label != "bar" {
                let _ = window.set_focus();
            }
            return;
        }
        if label == "bar" {
            crate::bar::create(&app, title);
            return;
        }
        let _ = WebviewWindowBuilder::new(&app, label, WebviewUrl::App(format!("/{label}").into()))
            .title(title)
            .build();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_is_zero_padded_and_never_negative() {
        assert_eq!(display_time(1453), "24:13");
        assert_eq!(display_time(65), "01:05");
        assert_eq!(display_time(0), "00:00");
        assert_eq!(display_time(-3), "00:00");
    }

    #[test]
    fn header_says_paused_only_when_stopped() {
        assert_eq!(header_text(Phase::Focus, true, 1453), "Focus · 24:13");
        assert_eq!(header_text(Phase::ShortBreak, false, 300), "Short Break · 05:00 · paused");
    }

    #[test]
    fn bar_label_follows_the_setting() {
        assert_eq!(bar_label(true), "Hide Floating Bar");
        assert_eq!(bar_label(false), "Show Floating Bar");
    }

    #[test]
    fn tooltip_names_the_phase_and_time() {
        assert_eq!(tooltip_text(Phase::LongBreak, 900), "Long Break — 15:00 remaining");
    }
}
