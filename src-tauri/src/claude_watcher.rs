use crate::types::{
    ClaudeHooksConfig, ClaudeSession, ClaudeSessionState, DebugInfo, StatusFileInfo,
    WorktreeClaudeStatus,
};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const STATUS_DIR_NAME: &str = ".woodeye-status";

/// Get the path to the woodeye status directory
pub fn get_status_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(STATUS_DIR_NAME)
}

/// Get the path to Claude's config directory
fn get_claude_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".claude")
}

/// Get the path to Claude's projects directory (for JSONL logs)
fn get_claude_projects_dir() -> PathBuf {
    get_claude_dir().join("projects")
}

/// Compute the status file path for a given project path
/// Uses the same md5 hash logic as the hooks
fn get_status_file_path(project_path: &str) -> PathBuf {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let status_dir = get_status_dir();

    // Compute md5 hash of project path (same as shell: echo "$path" | md5 | cut -c1-16)
    // Use the md5 command on macOS
    let hash = Command::new("md5")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(project_path.as_bytes());
                let _ = stdin.write_all(b"\n");
            }
            child.wait_with_output()
        })
        .ok()
        .and_then(|output| {
            String::from_utf8(output.stdout)
                .ok()
                .map(|s| s.trim().chars().take(16).collect::<String>())
        })
        .unwrap_or_else(|| {
            // Fallback: simple hash if md5 command fails
            format!("{:016x}", project_path.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64)))
        });

    status_dir.join(format!("{}.json", hash))
}

/// Remove the status file for a project if it exists
fn remove_status_file_for_project(project_path: &str) {
    let status_file = get_status_file_path(project_path);
    if status_file.exists() {
        let _ = fs::remove_file(&status_file);
    }
}

/// Status file format written by hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusFile {
    pub project_path: String,
    pub state: String,
    pub waiting_reason: Option<String>,
    #[serde(default)]
    pub timestamp: i64,
    /// Last tool that was invoked (for tool-aware timeouts)
    #[serde(default)]
    pub last_tool: Option<String>,
}

// =============================================================================
// Tool-specific timeout configuration
// =============================================================================

/// Stale threshold for waiting/idle states (10 minutes)
/// Users might be away for a bit, but sessions shouldn't persist forever
const WAITING_STATE_STALE_THRESHOLD: i64 = 600;

/// Get the stale threshold in seconds based on the last tool used
fn get_stale_threshold_for_tool(tool: Option<&str>) -> i64 {
    match tool {
        // Quick operations - 10 seconds
        Some("TodoWrite") | Some("ExitPlanMode") | Some("EnterPlanMode") => 10,

        // File I/O operations - 30 seconds
        Some("Read") | Some("Write") | Some("Edit") | Some("Glob") | Some("Grep")
        | Some("NotebookEdit") => 30,

        // System commands - 30 seconds
        Some("Bash") | Some("KillShell") => 30,

        // Network operations - 120 seconds (2 minutes)
        Some("WebFetch") | Some("WebSearch") => 120,

        // Sub-agents and complex operations - 180 seconds (3 minutes)
        Some("Task") | Some("TaskOutput") => 180,

        // Browser automation - 180 seconds (3 minutes)
        Some(t) if t.contains("Playwright") || t.contains("Browser") => 180,

        // MCP tools (variable, use longer timeout) - 120 seconds
        Some(t) if t.contains("mcp") || t.contains("MCP") => 120,

        // Default threshold - 60 seconds
        _ => 60,
    }
}

/// Get the stale threshold based on session state
/// Working states use tool-specific timeouts, waiting/idle states use longer timeout
fn get_stale_threshold_for_state(state: &str, tool: Option<&str>) -> i64 {
    match state {
        "working" => get_stale_threshold_for_tool(tool),
        // Waiting/idle states: user might be away, use longer threshold
        "waiting_for_approval" | "waiting_for_input" | "idle" => WAITING_STATE_STALE_THRESHOLD,
        // Unknown states: use default working threshold
        _ => get_stale_threshold_for_tool(tool),
    }
}

// =============================================================================
// JSONL Fallback Parser
// =============================================================================

/// JSONL entry structure for Claude's session logs
#[derive(Debug, Deserialize)]
struct JsonlEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    #[serde(default)]
    message: Option<JsonlMessage>,
    #[serde(default)]
    timestamp: Option<String>,
    /// Working directory from the first entry
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JsonlMessage {
    role: Option<String>,
    #[serde(default)]
    content: Vec<JsonlContent>,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JsonlContent {
    #[serde(rename = "type")]
    content_type: Option<String>,
    name: Option<String>,
}

/// Result of parsing a JSONL session file
enum JsonlParseResult {
    /// Session is active with the given state, tool, and timestamp
    Active(ClaudeSessionState, Option<String>, i64),
    /// Session has ended (detected "summary" entry)
    SessionEnded,
    /// Could not parse or determine state
    Unknown,
}

/// Parse the last few entries of a JSONL file to determine session state
fn parse_jsonl_for_state(jsonl_path: &Path) -> JsonlParseResult {
    let content = match fs::read_to_string(jsonl_path) {
        Ok(c) => c,
        Err(_) => return JsonlParseResult::Unknown,
    };
    let lines: Vec<&str> = content.lines().collect();

    // Look at the last few entries (up to 10)
    let recent_lines: Vec<&str> = lines.iter().rev().take(10).copied().collect();

    let mut last_timestamp = 0i64;
    let mut last_tool: Option<String> = None;
    let mut last_state = ClaudeSessionState::Unknown;

    for line in recent_lines.iter().rev() {
        if let Ok(entry) = serde_json::from_str::<JsonlEntry>(line) {
            // Check entry type for session end FIRST
            if entry.entry_type.as_deref() == Some("summary") {
                return JsonlParseResult::SessionEnded;
            }

            // Try to parse timestamp
            if let Some(ts_str) = &entry.timestamp {
                if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(ts_str) {
                    last_timestamp = ts.timestamp();
                }
            }

            if let Some(msg) = &entry.message {
                match msg.role.as_deref() {
                    Some("user") => {
                        // User message means Claude is working on response
                        last_state = ClaudeSessionState::Working;
                    }
                    Some("assistant") => {
                        // Check stop_reason
                        match msg.stop_reason.as_deref() {
                            Some("tool_use") => {
                                // Claude is requesting tool use - waiting for approval
                                last_state = ClaudeSessionState::WaitingForApproval;
                                // Try to get the tool name
                                for content in &msg.content {
                                    if content.content_type.as_deref() == Some("tool_use") {
                                        last_tool = content.name.clone();
                                    }
                                }
                            }
                            Some("end_turn") => {
                                // Claude finished, waiting for user input
                                last_state = ClaudeSessionState::WaitingForInput;
                            }
                            _ => {
                                // Still working/streaming
                                last_state = ClaudeSessionState::Working;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    if last_timestamp > 0 && last_state != ClaudeSessionState::Unknown {
        JsonlParseResult::Active(last_state, last_tool, last_timestamp)
    } else {
        JsonlParseResult::Unknown
    }
}

/// Get the project path (cwd) from a JSONL file's first entry
fn get_project_path_from_jsonl(jsonl_path: &Path) -> Option<String> {
    let content = fs::read_to_string(jsonl_path).ok()?;
    let first_line = content.lines().next()?;

    if let Ok(entry) = serde_json::from_str::<JsonlEntry>(first_line) {
        return entry.cwd;
    }
    None
}

/// Find active sessions by scanning JSONL logs
fn find_sessions_from_jsonl(project_path: &str) -> Vec<ClaudeSession> {
    let projects_dir = get_claude_projects_dir();
    if !projects_dir.exists() {
        return Vec::new();
    }

    let mut sessions = Vec::new();
    let normalized_target = normalize_path(project_path);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // Claude stores projects in a hashed directory structure
    if let Ok(entries) = fs::read_dir(&projects_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Look for JSONL files directly or in subdirectories
                scan_directory_for_jsonl(
                    &path,
                    &normalized_target,
                    now,
                    &mut sessions,
                );
            }
        }
    }

    sessions
}

/// Recursively scan a directory for JSONL files
fn scan_directory_for_jsonl(
    dir: &Path,
    normalized_target: &str,
    now: i64,
    sessions: &mut Vec<ClaudeSession>,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Recurse into subdirectories (but limit depth)
                if path.components().count() < 10 {
                    scan_directory_for_jsonl(&path, normalized_target, now, sessions);
                }
            } else if path.extension().map_or(false, |ext| ext == "jsonl") {
                // Found a JSONL file
                if let Some(cwd) = get_project_path_from_jsonl(&path) {
                    let normalized_cwd = normalize_path(&cwd);
                    if paths_match(&normalized_cwd, normalized_target) {
                        // Parse this session's state
                        match parse_jsonl_for_state(&path) {
                            JsonlParseResult::SessionEnded => {
                                // Session has ended - clean up any orphaned status file
                                remove_status_file_for_project(&cwd);
                            }
                            JsonlParseResult::Active(state, last_tool, timestamp) => {
                                // Check if session is stale using state-aware threshold
                                let state_str = match state {
                                    ClaudeSessionState::Working => "working",
                                    ClaudeSessionState::WaitingForApproval => "waiting_for_approval",
                                    ClaudeSessionState::WaitingForInput => "waiting_for_input",
                                    ClaudeSessionState::Idle => "idle",
                                    ClaudeSessionState::Unknown => "unknown",
                                };
                                let stale_threshold =
                                    get_stale_threshold_for_state(state_str, last_tool.as_deref());

                                // Only include if not too old
                                if (now - timestamp) < stale_threshold {
                                    let session_id = path
                                        .file_stem()
                                        .map(|s| s.to_string_lossy().to_string())
                                        .unwrap_or_default();

                                    sessions.push(ClaudeSession {
                                        session_id,
                                        project_path: cwd,
                                        state,
                                        waiting_reason: None,
                                        timestamp,
                                        last_tool,
                                    });
                                }
                            }
                            JsonlParseResult::Unknown => {
                                // Could not determine state, skip this file
                            }
                        }
                    }
                }
            }
        }
    }
}

// =============================================================================
// Status File Reading (Primary - Hook-based)
// =============================================================================

/// Read all status files from the woodeye status directory
/// Filters out stale sessions using tool-aware timeouts
pub fn read_all_status_files() -> Vec<ClaudeSession> {
    let status_dir = get_status_dir();
    if !status_dir.exists() {
        return Vec::new();
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut sessions = Vec::new();

    if let Ok(entries) = fs::read_dir(&status_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(status) = serde_json::from_str::<StatusFile>(&content) {
                        // Skip sessions with empty project path (malformed)
                        if status.project_path.is_empty() {
                            continue;
                        }

                        // Use state-aware stale threshold (applies to ALL states, not just working)
                        let stale_threshold =
                            get_stale_threshold_for_state(&status.state, status.last_tool.as_deref());
                        let is_stale = status.timestamp > 0
                            && (now - status.timestamp) > stale_threshold;

                        // Skip stale working sessions - they're from interrupted/ended sessions
                        if is_stale {
                            // Optionally clean up the stale file
                            let _ = fs::remove_file(&path);
                            continue;
                        }

                        let state = match status.state.as_str() {
                            "working" => ClaudeSessionState::Working,
                            "waiting_for_approval" => ClaudeSessionState::WaitingForApproval,
                            "waiting_for_input" => ClaudeSessionState::WaitingForInput,
                            "idle" => ClaudeSessionState::Idle,
                            _ => ClaudeSessionState::Unknown,
                        };

                        // Use filename (hash) as session ID
                        let session_id = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();

                        sessions.push(ClaudeSession {
                            session_id,
                            project_path: status.project_path,
                            state,
                            waiting_reason: status.waiting_reason,
                            timestamp: status.timestamp,
                            last_tool: status.last_tool,
                        });
                    }
                }
            }
        }
    }

    sessions
}

/// Get Claude status for a specific worktree path
/// Uses hook-based status files as primary source, with JSONL fallback
pub fn get_claude_status(worktree_path: &str) -> WorktreeClaudeStatus {
    let all_sessions = read_all_status_files();

    // Normalize the worktree path
    let normalized_path = normalize_path(worktree_path);

    // Filter sessions that match this worktree
    let mut active_sessions: Vec<ClaudeSession> = all_sessions
        .into_iter()
        .filter(|session| {
            let session_path = normalize_path(&session.project_path);
            paths_match(&session_path, &normalized_path)
        })
        .collect();

    // If no sessions found via hooks, try JSONL fallback
    if active_sessions.is_empty() {
        active_sessions = find_sessions_from_jsonl(worktree_path);
    }

    // Session needs input if state is WaitingForApproval, WaitingForInput, or Idle
    let has_pending_input = active_sessions.iter().any(|s| {
        matches!(
            s.state,
            ClaudeSessionState::WaitingForApproval
                | ClaudeSessionState::WaitingForInput
                | ClaudeSessionState::Idle
        )
    });

    WorktreeClaudeStatus {
        active_sessions,
        has_pending_input,
    }
}

/// Get Claude status for all worktrees (returns map of path -> status)
pub fn get_all_claude_statuses(worktree_paths: &[String]) -> HashMap<String, WorktreeClaudeStatus> {
    let all_sessions = read_all_status_files();
    let mut result = HashMap::new();

    for worktree_path in worktree_paths {
        let normalized_path = normalize_path(worktree_path);

        let mut active_sessions: Vec<ClaudeSession> = all_sessions
            .iter()
            .filter(|session| {
                let session_path = normalize_path(&session.project_path);
                paths_match(&session_path, &normalized_path)
            })
            .cloned()
            .collect();

        // If no sessions found via hooks, try JSONL fallback
        if active_sessions.is_empty() {
            active_sessions = find_sessions_from_jsonl(worktree_path);
        }

        // Session needs input if state is WaitingForApproval, WaitingForInput, or Idle
        let has_pending_input = active_sessions.iter().any(|s| {
            matches!(
                s.state,
                ClaudeSessionState::WaitingForApproval
                    | ClaudeSessionState::WaitingForInput
                    | ClaudeSessionState::Idle
            )
        });

        result.insert(
            worktree_path.clone(),
            WorktreeClaudeStatus {
                active_sessions,
                has_pending_input,
            },
        );
    }

    result
}

/// Normalize a path for comparison
fn normalize_path(path: &str) -> String {
    // Remove trailing slashes and normalize
    let p = Path::new(path);
    p.canonicalize()
        .unwrap_or_else(|_| p.to_path_buf())
        .to_string_lossy()
        .trim_end_matches('/')
        .to_string()
}

/// Check if two paths match (handles symlinks and relative paths)
fn paths_match(path1: &str, path2: &str) -> bool {
    if path1.is_empty() || path2.is_empty() {
        return false;
    }
    path1 == path2
}

// =============================================================================
// File Watcher
// =============================================================================

/// Start watching the woodeye status directory for changes
pub fn start_claude_watching(app: AppHandle) -> Result<(), String> {
    let status_dir = get_status_dir();

    // Create the status directory if it doesn't exist
    if !status_dir.exists() {
        fs::create_dir_all(&status_dir).map_err(|e| e.to_string())?;
    }

    let (tx, rx) = mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(100), tx).map_err(|e| e.to_string())?;

    debouncer
        .watcher()
        .watch(&status_dir, notify::RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch {}: {}", status_dir.display(), e))?;

    // Store the debouncer in app state to keep it alive
    app.manage(ClaudeWatcherState {
        _debouncer: debouncer,
    });

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
                Err(e) => eprintln!("Claude watch error: {:?}", e),
            }
        }
    });

    Ok(())
}

// State to keep the debouncer alive
struct ClaudeWatcherState {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

// =============================================================================
// Hook Configuration
// =============================================================================

/// Check if Claude hooks are configured for Woodeye
pub fn check_hooks_configured() -> ClaudeHooksConfig {
    let claude_dir = get_claude_dir();
    let settings_path = claude_dir.join("settings.json");
    let status_dir = get_status_dir();

    let status_dir_exists = status_dir.exists();

    // Check if settings.json exists and contains woodeye hooks
    let configured = if settings_path.exists() {
        if let Ok(content) = fs::read_to_string(&settings_path) {
            content.contains(".woodeye-status")
        } else {
            false
        }
    } else {
        false
    };

    ClaudeHooksConfig {
        configured,
        status_dir_exists,
    }
}

/// Configure Claude hooks for Woodeye status tracking
/// Includes: PreToolUse, PostToolUse, Notification, Stop, PermissionRequest, SessionStart, SessionEnd
pub fn configure_claude_hooks() -> Result<(), String> {
    let claude_dir = get_claude_dir();
    let settings_path = claude_dir.join("settings.json");
    let status_dir = get_status_dir();

    // Create the status directory if it doesn't exist
    if !status_dir.exists() {
        fs::create_dir_all(&status_dir).map_err(|e| e.to_string())?;
    }

    // Create Claude directory if it doesn't exist
    if !claude_dir.exists() {
        fs::create_dir_all(&claude_dir).map_err(|e| e.to_string())?;
    }

    // Read existing settings or create new
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Define the hooks we want to add
    // Uses CLAUDE_PROJECT_DIR as the identifier (hashed to create filename)
    // For tool events (PreToolUse, PostToolUse): matcher is a string pattern, "*" matches all
    // For other events (Notification, Stop, etc.): no matcher needed
    //
    // Hook events:
    // - PreToolUse/PostToolUse: Tracks tool execution, includes tool name for tool-aware timeouts
    // - PermissionRequest: Fires when Claude needs user approval (accurate waiting_for_approval state)
    // - Notification: Fires when Claude is idle/waiting for input
    // - Stop: Session ended, clean up status file
    // - SessionStart: Session beginning
    // - SessionEnd: Session completed
    let woodeye_hooks = serde_json::json!({
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": format!(
                                "mkdir -p {} && echo '{{\"project_path\":\"'\"$CLAUDE_PROJECT_DIR\"'\",\"state\":\"working\",\"last_tool\":\"'\"$CLAUDE_TOOL_NAME\"'\",\"timestamp\":'$(date +%s)'}}' > {}/$(echo \"$CLAUDE_PROJECT_DIR\" | md5 | cut -c1-16).json",
                                status_dir.display(),
                                status_dir.display()
                            )
                        }
                    ]
                }
            ],
            "PostToolUse": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": format!(
                                "mkdir -p {} && echo '{{\"project_path\":\"'\"$CLAUDE_PROJECT_DIR\"'\",\"state\":\"working\",\"last_tool\":\"'\"$CLAUDE_TOOL_NAME\"'\",\"timestamp\":'$(date +%s)'}}' > {}/$(echo \"$CLAUDE_PROJECT_DIR\" | md5 | cut -c1-16).json",
                                status_dir.display(),
                                status_dir.display()
                            )
                        }
                    ]
                }
            ],
            "PermissionRequest": [
                {
                    "hooks": [
                        {
                            "type": "command",
                            "command": format!(
                                "mkdir -p {} && echo '{{\"project_path\":\"'\"$CLAUDE_PROJECT_DIR\"'\",\"state\":\"waiting_for_approval\",\"waiting_reason\":\"'\"$CLAUDE_TOOL_NAME\"'\",\"timestamp\":'$(date +%s)'}}' > {}/$(echo \"$CLAUDE_PROJECT_DIR\" | md5 | cut -c1-16).json",
                                status_dir.display(),
                                status_dir.display()
                            )
                        }
                    ]
                }
            ],
            "Notification": [
                {
                    "hooks": [
                        {
                            "type": "command",
                            "command": format!(
                                "mkdir -p {} && echo '{{\"project_path\":\"'\"$CLAUDE_PROJECT_DIR\"'\",\"state\":\"waiting_for_input\",\"timestamp\":'$(date +%s)'}}' > {}/$(echo \"$CLAUDE_PROJECT_DIR\" | md5 | cut -c1-16).json",
                                status_dir.display(),
                                status_dir.display()
                            )
                        }
                    ]
                }
            ],
            "Stop": [
                {
                    "hooks": [
                        {
                            "type": "command",
                            "command": format!(
                                "rm -f {}/$(echo \"$CLAUDE_PROJECT_DIR\" | md5 | cut -c1-16).json",
                                status_dir.display()
                            )
                        }
                    ]
                }
            ],
            "SessionStart": [
                {
                    "hooks": [
                        {
                            "type": "command",
                            "command": format!(
                                "mkdir -p {} && echo '{{\"project_path\":\"'\"$CLAUDE_PROJECT_DIR\"'\",\"state\":\"working\",\"timestamp\":'$(date +%s)'}}' > {}/$(echo \"$CLAUDE_PROJECT_DIR\" | md5 | cut -c1-16).json",
                                status_dir.display(),
                                status_dir.display()
                            )
                        }
                    ]
                }
            ],
            "SessionEnd": [
                {
                    "hooks": [
                        {
                            "type": "command",
                            "command": format!(
                                "rm -f {}/$(echo \"$CLAUDE_PROJECT_DIR\" | md5 | cut -c1-16).json",
                                status_dir.display()
                            )
                        }
                    ]
                }
            ]
        }
    });

    // Merge hooks with existing settings
    if let Some(existing_hooks) = settings.get_mut("hooks") {
        if let Some(new_hooks) = woodeye_hooks.get("hooks") {
            // Merge each hook type
            for (hook_type, hook_list) in new_hooks.as_object().unwrap() {
                if let Some(existing_list) = existing_hooks.get_mut(hook_type) {
                    // Append new hooks to existing list
                    if let (Some(existing_arr), Some(new_arr)) =
                        (existing_list.as_array_mut(), hook_list.as_array())
                    {
                        for hook in new_arr {
                            // Only add if not already present (check by matcher containing woodeye)
                            let hook_str = hook.to_string();
                            if hook_str.contains(".woodeye-status")
                                && !existing_arr
                                    .iter()
                                    .any(|h| h.to_string().contains(".woodeye-status"))
                            {
                                existing_arr.push(hook.clone());
                            }
                        }
                    }
                } else {
                    // Add new hook type
                    existing_hooks[hook_type] = hook_list.clone();
                }
            }
        }
    } else {
        // No existing hooks, add all
        settings["hooks"] = woodeye_hooks["hooks"].clone();
    }

    // Write back to settings file
    let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&settings_path, content).map_err(|e| e.to_string())?;

    Ok(())
}

/// Remove Woodeye hooks from Claude settings
pub fn remove_claude_hooks() -> Result<(), String> {
    let claude_dir = get_claude_dir();
    let settings_path = claude_dir.join("settings.json");

    if !settings_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
    let mut settings: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| e.to_string())?;

    if let Some(hooks) = settings.get_mut("hooks") {
        if let Some(hooks_obj) = hooks.as_object_mut() {
            for (_hook_type, hook_list) in hooks_obj.iter_mut() {
                if let Some(arr) = hook_list.as_array_mut() {
                    arr.retain(|h| !h.to_string().contains(".woodeye-status"));
                }
            }
        }
    }

    let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&settings_path, content).map_err(|e| e.to_string())?;

    Ok(())
}

// =============================================================================
// Debug Info
// =============================================================================

/// Get debug information about Claude watcher state
pub fn get_debug_info() -> DebugInfo {
    let status_dir = get_status_dir();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut status_files = Vec::new();

    if status_dir.exists() {
        if let Ok(entries) = fs::read_dir(&status_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(status) = serde_json::from_str::<StatusFile>(&content) {
                            let age_seconds = if status.timestamp > 0 {
                                now - status.timestamp
                            } else {
                                0
                            };

                            let stale_threshold =
                                get_stale_threshold_for_state(&status.state, status.last_tool.as_deref());
                            let is_stale = age_seconds > stale_threshold;

                            status_files.push(StatusFileInfo {
                                filename: path
                                    .file_name()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                                project_path: status.project_path,
                                state: format!(
                                    "{} (tool: {}, threshold: {}s)",
                                    status.state,
                                    status.last_tool.as_deref().unwrap_or("none"),
                                    stale_threshold
                                ),
                                timestamp: status.timestamp,
                                age_seconds,
                                is_stale,
                            });
                        }
                    }
                }
            }
        }
    }

    // Sort by timestamp descending (most recent first)
    status_files.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    let hooks_config = check_hooks_configured();

    DebugInfo {
        status_dir: status_dir.to_string_lossy().to_string(),
        status_files,
        hooks_configured: hooks_config.configured,
        current_timestamp: now,
        stale_threshold_secs: 60, // Default, actual varies by tool
    }
}
