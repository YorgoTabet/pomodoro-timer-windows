mod commands;
pub mod core;
mod timer;
mod tray;

use core::settings::Settings;
use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        // Registered first, as the plugin requires. A second launch does nothing:
        // the tray icon of the running one is already there.
        .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::start,
            commands::pause,
            commands::toggle,
            commands::skip,
            commands::reset,
            commands::get_settings,
            commands::save_settings,
        ])
        .setup(|app| {
            // Tray only, like the Mac app's missing Dock icon.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let path = app.path().app_config_dir()?.join("settings.json");
            let shared = timer::Shared::new(Settings::load(&path), path);
            let state = shared.state();
            app.manage(shared);
            tray::build(app.handle(), &state)?;
            timer::spawn_ticker(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_, event| {
        // Closing the last window must not quit: the tray is the app.
        if let RunEvent::ExitRequested { api, code: None, .. } = event {
            api.prevent_exit();
        }
    });
}
