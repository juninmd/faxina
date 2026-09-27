mod category;
mod commands;
mod disks;
mod dupes;
mod guard;
mod junk;
mod junk_defs;
mod model;
mod scan;
mod view;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::start_scan,
            commands::cancel_scan,
            commands::get_view,
            commands::get_suggestions,
            commands::delete_paths,
            commands::junk_scan,
            commands::junk_clean,
            commands::find_duplicates,
            commands::cancel_duplicates,
            disks::disk_info,
            disks::list_disks,
            commands::home_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
