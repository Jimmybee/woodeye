use crate::claude_watcher;
use crate::git;
use crate::menu;
use crate::types::{
    BranchInfo, ClaudeHooksConfig, CommitDiff, CommitInfo, CreateWorktreeOptions, DebugInfo,
    PruneResult, WorkingDiff, Worktree, WorktreeClaudeStatus, WorktreeStatus,
};
use crate::watcher;
use std::collections::HashMap;
use tauri::async_runtime::spawn_blocking;

#[tauri::command]
pub async fn list_worktrees(repo_path: String) -> Result<Vec<Worktree>, String> {
    spawn_blocking(move || git::get_all_worktrees(&repo_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn start_watching(app: tauri::AppHandle, paths: Vec<String>) -> Result<(), String> {
    watcher::start_watching(app, paths)
}

#[tauri::command]
pub async fn get_commit_history(
    worktree_path: String,
    limit: usize,
    offset: usize,
) -> Result<Vec<CommitInfo>, String> {
    spawn_blocking(move || git::get_commit_history(&worktree_path, limit, offset))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_commit_diff(
    worktree_path: String,
    commit_sha: String,
) -> Result<CommitDiff, String> {
    spawn_blocking(move || git::get_commit_diff(&worktree_path, &commit_sha))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_working_diff(worktree_path: String) -> Result<WorkingDiff, String> {
    spawn_blocking(move || git::get_working_diff(&worktree_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_worktree_status(worktree_path: String) -> Result<WorktreeStatus, String> {
    spawn_blocking(move || git::get_worktree_status_by_path(&worktree_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_worktree(
    repo_path: String,
    options: CreateWorktreeOptions,
) -> Result<Worktree, String> {
    spawn_blocking(move || git::create_worktree(&repo_path, options))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_worktree(
    repo_path: String,
    worktree_path: String,
    force: bool,
) -> Result<(), String> {
    spawn_blocking(move || git::delete_worktree(&repo_path, &worktree_path, force))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn prune_worktrees(repo_path: String) -> Result<PruneResult, String> {
    spawn_blocking(move || git::prune_worktrees(&repo_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_branches(repo_path: String) -> Result<Vec<BranchInfo>, String> {
    spawn_blocking(move || git::list_branches(&repo_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn open_in_terminal(path: String, terminal: String) -> Result<(), String> {
    use std::process::Command;

    let result = match terminal.as_str() {
        "terminal" => Command::new("open").args(["-a", "Terminal", &path]).spawn(),
        "warp" => Command::new("open").arg(format!("warp://action/new_window?path={}", path)).spawn(),
        "iterm" => Command::new("open").args(["-a", "iTerm", &path]).spawn(),
        _ => return Err(format!("Unknown terminal: {}", terminal)),
    };

    result.map_err(|e| format!("Failed to open terminal: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn open_claude_in_terminal(path: String) -> Result<(), String> {
    use std::process::Command;

    // Use AppleScript to open Terminal and run claude
    let script = format!(
        r#"tell application "Terminal"
            do script "cd '{}' && claude"
            activate
        end tell"#,
        path.replace("'", "'\\''") // Escape single quotes
    );

    Command::new("osascript")
        .args(["-e", &script])
        .spawn()
        .map_err(|e| format!("Failed to open terminal: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn set_theme_menu_state(app_handle: tauri::AppHandle, theme: String) -> Result<(), String> {
    menu::update_theme_checkmarks(&app_handle, &theme)
}

// Claude status commands
#[tauri::command]
pub async fn get_claude_status(worktree_path: String) -> Result<WorktreeClaudeStatus, String> {
    spawn_blocking(move || Ok(claude_watcher::get_claude_status(&worktree_path)))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_all_claude_statuses(
    worktree_paths: Vec<String>,
) -> Result<HashMap<String, WorktreeClaudeStatus>, String> {
    spawn_blocking(move || Ok(claude_watcher::get_all_claude_statuses(&worktree_paths)))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn start_claude_watching(app: tauri::AppHandle) -> Result<(), String> {
    claude_watcher::start_claude_watching(app)
}

#[tauri::command]
pub fn check_claude_hooks() -> ClaudeHooksConfig {
    claude_watcher::check_hooks_configured()
}

#[tauri::command]
pub async fn configure_claude_hooks() -> Result<(), String> {
    spawn_blocking(claude_watcher::configure_claude_hooks)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn remove_claude_hooks() -> Result<(), String> {
    spawn_blocking(claude_watcher::remove_claude_hooks)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_debug_info() -> DebugInfo {
    claude_watcher::get_debug_info()
}
