//! KernTeX desktop app. Domain logic lives in [`core`] (Tauri-free);
//! [`commands`] exposes it to the web UI.

pub mod commands;
pub mod core;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::detect_engines,
            commands::open_project,
            commands::read_tree,
            commands::project_exists,
            commands::read_file,
            commands::write_file,
            commands::create_file,
            commands::create_dir,
            commands::rename_path,
            commands::trash_path,
            commands::import_files,
            commands::read_binary,
            commands::scan_refs,
            commands::save_project_settings,
            commands::tex_root,
            commands::new_project,
            commands::welcome_project,
            commands::typst_welcome_project,
            commands::import_zip,
            commands::export_zip,
            commands::read_pdf,
            commands::export_pdf,
            commands::compile,
            commands::cancel_compile,
        ])
        .run(tauri::generate_context!())
        .expect("error while running KernTeX");
}
