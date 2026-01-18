use crate::git;
use crate::types::{CommitDiff, CommitInfo, WorkingDiff, Worktree, WorktreeStatus};
use crate::watcher;
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
