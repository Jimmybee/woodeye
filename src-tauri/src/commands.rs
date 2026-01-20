use crate::claude_status::{self, ClaudeSession, HooksState};
use crate::git;
use crate::menu;
use crate::types::{
    BranchInfo, CommitDiff, CommitInfo, CreateWorktreeOptions, PruneResult, WorkingDiff, Worktree,
    WorktreeStatus,
};
use crate::watcher;
use tauri::{Emitter, Manager, WebviewWindowBuilder};
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

#[tauri::command]
pub async fn list_claude_sessions() -> Result<Vec<ClaudeSession>, String> {
    spawn_blocking(claude_status::list_sessions)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_claude_session(session_id: String) -> Result<(), String> {
    spawn_blocking(move || claude_status::delete_session(&session_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn start_watching_claude_status(app: tauri::AppHandle) -> Result<(), String> {
    use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
    use std::sync::mpsc;
    use std::time::Duration;

    let status_dir = claude_status::get_status_dir()
        .ok_or("Could not determine status directory")?;

    // Create the directory if it doesn't exist
    if !status_dir.exists() {
        std::fs::create_dir_all(&status_dir)
            .map_err(|e| format!("Failed to create status directory: {}", e))?;
    }

    let (tx, rx) = mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(200), tx)
        .map_err(|e| e.to_string())?;

    debouncer
        .watcher()
        .watch(&status_dir, notify::RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch status directory: {}", e))?;

    // Store the debouncer in app state to keep it alive
    app.manage(ClaudeStatusWatcherState { _debouncer: debouncer });

    // Spawn thread to handle events
    let app_handle = app.clone();
    std::thread::spawn(move || {
        while let Ok(result) = rx.recv() {
            match result {
                Ok(events) => {
                    let has_changes = events
                        .iter()
                        .any(|e| matches!(e.kind, DebouncedEventKind::Any));
                    if has_changes {
                        let _ = app_handle.emit("claude-status-changed", ());
                    }
                }
                Err(e) => eprintln!("Claude status watch error: {:?}", e),
            }
        }
    });

    Ok(())
}

struct ClaudeStatusWatcherState {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

#[tauri::command]
pub async fn open_claude_status_window(app: tauri::AppHandle) -> Result<(), String> {
    // Check if window already exists
    if let Some(window) = app.get_webview_window("claude-status") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Create new window
    let url = tauri::WebviewUrl::App("claude-status.html".into());

    WebviewWindowBuilder::new(&app, "claude-status", url)
        .title("Claude Sessions")
        .inner_size(400.0, 600.0)
        .resizable(true)
        .build()
        .map_err(|e| format!("Failed to create window: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_claude_hooks_state() -> Result<HooksState, String> {
    spawn_blocking(claude_status::get_hooks_state)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn remove_claude_hooks() -> Result<(), String> {
    spawn_blocking(claude_status::remove_hooks)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_claude_hooks() -> Result<(), String> {
    spawn_blocking(claude_status::apply_hooks)
        .await
        .map_err(|e| e.to_string())?
}
