use crate::{
    core::phase::Phase,
    timer::{self, TimerState},
};
use std::sync::OnceLock;
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

pub fn build(app: &AppHandle, state: &TimerState) -> tauri::Result<()> {
    let header = MenuItem::with_id(app, "header", header_text(state.phase, state.is_running, state.remaining_seconds), false, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", toggle_label(state.is_running), true, None::<&str>)?;
    let skip = MenuItem::with_id(app, "skip", "Skip", true, None::<&str>)?;
    let reset = MenuItem::with_id(app, "reset", "Reset", true, None::<&str>)?;
    let bar = MenuItem::with_id(app, "bar", "Show floating bar", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let stats = MenuItem::with_id(app, "stats", "Stats…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Pomodoro", true, None::<&str>)?;
    let sep = || PredefinedMenuItem::separator(app);

    let menu = Menu::with_items(
        app,
        &[&header, &sep()?, &toggle, &skip, &reset, &sep()?, &bar, &stats, &settings, &sep()?, &quit],
    )?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip(tooltip_text(state.phase, state.remaining_seconds))
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => timer::apply(app, |c, s, now| {
                c.toggle(now, s);
                None
            }),
            "skip" => timer::apply(app, |c, s, _| Some(c.skip(s))),
            "reset" => timer::apply(app, |c, s, _| {
                c.reset(s);
                None
            }),
            "bar" => open_window(app, "bar", "Pomodoro"),
            "settings" => open_window(app, "settings", "Settings"),
            "stats" => open_window(app, "stats", "Stats"),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;

    let _ = ITEMS.set(Items { header, toggle });
    Ok(())
}

/// Called on every published change, so the tooltip and header follow the clock.
pub fn refresh(app: &AppHandle, state: &TimerState) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tooltip_text(state.phase, state.remaining_seconds)));
    }
    if let Some(items) = ITEMS.get() {
        let _ = items.header.set_text(header_text(state.phase, state.is_running, state.remaining_seconds));
        let _ = items.toggle.set_text(toggle_label(state.is_running));
    }
}

fn toggle_label(is_running: bool) -> &'static str {
    if is_running {
        "Pause"
    } else {
        "Start"
    }
}

/// Focus the window if it is open, otherwise create it. The page at `/{label}` is
/// built in a later task.
pub fn open_window(app: &AppHandle, label: &'static str, title: &'static str) {
    let app = app.clone();
    // Off the menu callback: building a window from inside it can stall on Windows.
    std::thread::spawn(move || {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.show();
            let _ = window.set_focus();
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
    fn tooltip_names_the_phase_and_time() {
        assert_eq!(tooltip_text(Phase::LongBreak, 900), "Long Break — 15:00 remaining");
    }
}
