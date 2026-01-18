mod commands;
mod git;
mod types;
mod watcher;

pub use commands::*;
pub use types::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_worktrees,
            commands::start_watching,
            commands::get_commit_history,
            commands::get_commit_diff,
            commands::get_working_diff,
            commands::get_worktree_status,
            commands::create_worktree,
            commands::delete_worktree,
            commands::prune_worktrees,
            commands::list_branches,
            commands::open_in_terminal
        ])
        .setup(|_app| {
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
