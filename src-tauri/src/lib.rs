mod claude_watcher;
mod commands;
mod git;
mod menu;
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
            commands::open_in_terminal,
            commands::open_claude_in_terminal,
            commands::set_theme_menu_state,
            commands::get_claude_status,
            commands::get_all_claude_statuses,
            commands::start_claude_watching,
            commands::check_claude_hooks,
            commands::configure_claude_hooks,
            commands::remove_claude_hooks,
            commands::get_debug_info
        ])
        .setup(|app| {
            if let Err(e) = menu::build_menu(app) {
                eprintln!("Failed to build menu: {}", e);
            }
            menu::setup_menu_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
