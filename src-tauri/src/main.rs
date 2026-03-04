mod app_state;
mod commands;
mod db;
mod domain;
mod error;
mod resources;
mod services;
mod thunderstore;

use app_state::AppState;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let state = AppState::new(&app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::cache::queue_install_to_cache,
            commands::cache::get_cache_summary,
            commands::cache::open_cache_folder,
            commands::cache::clear_cache,
            commands::catalog::sync_catalog,
            commands::catalog::get_catalog_summary,
            commands::catalog::search_packages,
            commands::catalog::get_package_detail,
            commands::downloads::list_active_downloads,
            commands::downloads::get_task,
            commands::profiles::list_profiles,
            commands::profiles::get_active_profile,
            commands::profiles::set_active_profile,
            commands::profiles::create_profile,
            commands::profiles::update_profile,
            commands::profiles::delete_profile,
            commands::profiles::get_profile_detail,
            commands::profiles::reset_all_data,
            commands::reference::list_reference_rows,
            commands::reference::set_reference_state,
            commands::settings::get_warning_prefs,
            commands::settings::set_warning_preference,
            commands::system::open_external_url
        ])
        .run(tauri::generate_context!())
        .expect("failed to run 49modman");
}
