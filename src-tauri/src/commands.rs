use crate::{
    core::settings::Settings,
    timer::{self, Shared, TimerState},
};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_state(shared: State<Shared>) -> TimerState {
    shared.state()
}

#[tauri::command]
pub fn start(app: AppHandle) {
    timer::apply(&app, |c, s, now| {
        c.start(now, s);
        None
    });
}

#[tauri::command]
pub fn pause(app: AppHandle) {
    timer::apply(&app, |c, _, now| {
        c.pause(now);
        None
    });
}

#[tauri::command]
pub fn toggle(app: AppHandle) {
    timer::apply(&app, |c, s, now| {
        c.toggle(now, s);
        None
    });
}

#[tauri::command]
pub fn skip(app: AppHandle) {
    timer::apply(&app, |c, s, _| Some(c.skip(s)));
}

#[tauri::command]
pub fn reset(app: AppHandle) {
    timer::apply(&app, |c, s, _| {
        c.reset(s);
        None
    });
}

#[tauri::command]
pub fn get_settings(shared: State<Shared>) -> Settings {
    shared.settings()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    timer::commit_settings(&app, settings)
}
