//! KernTeX desktop app. Domain logic lives in [`core`] (Tauri-free);
//! [`commands`] exposes it to the web UI and [`mcp`] to AI agents.

pub mod commands;
pub mod core;
pub mod mcp;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(commands::AppState::default())
        .setup(|app| {
            let hub = mcp::Hub::new(commands::TauriFrontend(app.handle().clone()));
            app.manage(hub.clone());
            tauri::async_runtime::spawn(hub.listen());
            Ok(())
        })
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
            commands::git_status,
            commands::git_diff,
            commands::git_suggest_message,
            commands::git_init,
            commands::git_commit,
            commands::git_pull,
            commands::git_push,
            commands::git_cancel,
            commands::git_set_remote,
            commands::git_set_identity,
            commands::git_abort_merge,
            commands::mcp_configure,
            commands::mcp_respond,
            commands::mcp_clients,
            commands::mcp_disconnect,
            commands::mcp_setup,
            commands::mcp_install_link,
            commands::mcp_add_to_claude_code,
        ])
        .run(tauri::generate_context!())
        .expect("error while running KernTeX");
}
