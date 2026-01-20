use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClaudeSession {
    pub project_path: String,
    pub session_id: String,
    pub state: String, // "working", "idle", "waiting_for_approval"
    pub timestamp: u64,
    pub name: Option<String>, // Extracted from first prompt
    #[serde(skip_deserializing)]
    pub raw_json: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HooksState {
    pub hooks_enabled: bool,
    pub hooks_json: Option<String>,
}

pub fn get_status_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".woodeye-status"))
}

fn get_names_file_path() -> Option<PathBuf> {
    get_status_dir().map(|d| d.join("names.json"))
}

/// Read session names from the separate names file
fn read_session_names() -> std::collections::HashMap<String, String> {
    let Some(path) = get_names_file_path() else {
        return std::collections::HashMap::new();
    };

    if !path.exists() {
        return std::collections::HashMap::new();
    }

    fs::read_to_string(&path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

/// Remove a session name from the names file
fn remove_session_name(session_id: &str) -> Result<(), String> {
    let path = get_names_file_path().ok_or("Could not determine names file path")?;

    if !path.exists() {
        return Ok(());
    }

    let contents = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read names file: {}", e))?;

    let mut names: std::collections::HashMap<String, String> = serde_json::from_str(&contents)
        .unwrap_or_default();

    names.remove(session_id);

    let updated = serde_json::to_string_pretty(&names)
        .map_err(|e| format!("Failed to serialize names: {}", e))?;

    fs::write(&path, updated)
        .map_err(|e| format!("Failed to write names file: {}", e))?;

    Ok(())
}

pub fn list_sessions() -> Result<Vec<ClaudeSession>, String> {
    let status_dir = get_status_dir().ok_or("Could not determine home directory")?;

    if !status_dir.exists() {
        return Ok(Vec::new());
    }

    // Read session names from separate file
    let names = read_session_names();

    let mut sessions: Vec<ClaudeSession> = Vec::new();

    let entries = fs::read_dir(&status_dir).map_err(|e| format!("Failed to read status directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        // Skip non-JSON files and special files (names.json, hooks_backup.json)
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !filename.ends_with(".json") || filename == "names.json" || filename == "hooks_backup.json" {
            continue;
        }

        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(mut session) = serde_json::from_str::<ClaudeSession>(&contents) {
                // Merge name from separate names file
                if session.name.is_none() {
                    session.name = names.get(&session.session_id).cloned();
                }
                session.raw_json = contents;
                sessions.push(session);
            }
        }
    }

    // Sort by timestamp (newest first)
    sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(sessions)
}

pub fn delete_session(session_id: &str) -> Result<(), String> {
    let status_dir = get_status_dir().ok_or("Could not determine home directory")?;
    let file_path = status_dir.join(format!("{}.json", session_id));

    if file_path.exists() {
        fs::remove_file(&file_path)
            .map_err(|e| format!("Failed to delete session file: {}", e))?;
    }

    // Also remove from names file
    let _ = remove_session_name(session_id);

    Ok(())
}

// --- Hooks Management ---

fn get_claude_settings_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("settings.json"))
}

fn get_hooks_backup_path() -> Option<PathBuf> {
    get_status_dir().map(|d| d.join("hooks_backup.json"))
}

/// Generate the Woodeye status hooks configuration
fn generate_woodeye_hooks() -> Value {
    let status_dir = get_status_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "/tmp/.woodeye-status".to_string());

    let base_cmd = |state: &str| -> String {
        format!(
            r#"input=$(cat); sid=$(echo "$input" | jq -r '.session_id'); [ -n "$CLAUDE_PROJECT_DIR" ] && [ -n "$sid" ] && mkdir -p {} && echo "{{\"project_path\":\"$CLAUDE_PROJECT_DIR\",\"session_id\":\"$sid\",\"state\":\"{}\",\"timestamp\":$(date +%s)}}" > {}/{{}}.json"#,
            status_dir, state, status_dir
        ).replace("{}", "$sid")
    };

    let cleanup_cmd = format!(
        r#"input=$(cat); sid=$(echo "$input" | jq -r '.session_id'); if [ -n "$sid" ]; then rm -f {0}/"$sid".json; nf="{0}/names.json"; if [ -f "$nf" ]; then jq --arg s "$sid" 'del(.[$s])' "$nf" > "$nf.tmp" && mv "$nf.tmp" "$nf"; fi; fi"#,
        status_dir
    );

    // Command to extract session name from first user prompt and store in separate names.json
    let name_cmd = format!(
        r#"input=$(cat); sid=$(echo "$input" | jq -r '.session_id'); prompt=$(echo "$input" | jq -r '.prompt // empty'); nf="{0}/names.json"; if [ -n "$sid" ] && [ -n "$prompt" ]; then if [ -f "$nf" ]; then ex=$(jq -r --arg s "$sid" '.[$s] // empty' "$nf" 2>/dev/null); else ex=""; fi; if [ -z "$ex" ]; then name=$(printf '%s' "$prompt" | head -c 50 | sed 's/[[:space:]][^[:space:]]*$//'); if [ -f "$nf" ]; then jq --arg s "$sid" --arg n "$name" '. + {{($s): $n}}' "$nf" > "$nf.tmp" && mv "$nf.tmp" "$nf"; else echo "{{\"$sid\":\"$name\"}}" > "$nf"; fi; fi; fi"#,
        status_dir
    );

    json!({
        "PermissionRequest": [{
            "hooks": [{
                "command": base_cmd("waiting_for_approval"),
                "type": "command"
            }]
        }],
        "PostToolUse": [{
            "hooks": [{
                "command": base_cmd("working"),
                "type": "command"
            }],
            "matcher": "*"
        }],
        "PreToolUse": [{
            "hooks": [{
                "command": base_cmd("working"),
                "type": "command"
            }],
            "matcher": "*"
        }],
        "SessionEnd": [{
            "hooks": [{
                "command": cleanup_cmd,
                "type": "command"
            }]
        }],
        "SessionStart": [{
            "hooks": [{
                "command": base_cmd("idle"),
                "type": "command"
            }]
        }],
        "Stop": [{
            "hooks": [{
                "command": base_cmd("idle"),
                "type": "command"
            }]
        }],
        "UserPromptSubmit": [{
            "hooks": [{
                "command": name_cmd,
                "type": "command"
            }]
        }],
        "Notification": [{
            "hooks": [{
                "command": base_cmd("waiting_for_approval"),
                "type": "command"
            }],
            "matcher": "permission_prompt"
        }]
    })
}

/// Check if Woodeye hooks are currently enabled in Claude settings
pub fn get_hooks_state() -> Result<HooksState, String> {
    let settings_path = get_claude_settings_path()
        .ok_or("Could not determine Claude settings path")?;

    if !settings_path.exists() {
        return Ok(HooksState {
            hooks_enabled: false,
            hooks_json: None,
        });
    }

    let contents = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read Claude settings: {}", e))?;

    let settings: Value = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse Claude settings: {}", e))?;

    let hooks_enabled = settings.get("hooks")
        .and_then(|h| h.get("SessionStart"))
        .is_some();

    let hooks_json = settings.get("hooks")
        .map(|h| serde_json::to_string_pretty(h).unwrap_or_default());

    Ok(HooksState {
        hooks_enabled,
        hooks_json,
    })
}

/// Remove Woodeye hooks from Claude settings (backs up first)
pub fn remove_hooks() -> Result<(), String> {
    let settings_path = get_claude_settings_path()
        .ok_or("Could not determine Claude settings path")?;

    if !settings_path.exists() {
        return Ok(());
    }

    let contents = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read Claude settings: {}", e))?;

    let mut settings: Value = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse Claude settings: {}", e))?;

    // Backup current hooks if they exist
    if let Some(hooks) = settings.get("hooks") {
        let backup_path = get_hooks_backup_path()
            .ok_or("Could not determine hooks backup path")?;

        // Ensure status dir exists
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create backup directory: {}", e))?;
        }

        let backup_content = serde_json::to_string_pretty(hooks)
            .map_err(|e| format!("Failed to serialize hooks: {}", e))?;

        fs::write(&backup_path, backup_content)
            .map_err(|e| format!("Failed to write hooks backup: {}", e))?;
    }

    // Remove hooks from settings
    if let Some(obj) = settings.as_object_mut() {
        obj.remove("hooks");
    }

    // Write updated settings
    let updated = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(&settings_path, updated)
        .map_err(|e| format!("Failed to write settings: {}", e))?;

    Ok(())
}

/// Apply Woodeye hooks to Claude settings
pub fn apply_hooks() -> Result<(), String> {
    let settings_path = get_claude_settings_path()
        .ok_or("Could not determine Claude settings path")?;

    // Read existing settings or create new
    let mut settings: Value = if settings_path.exists() {
        let contents = fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read Claude settings: {}", e))?;
        serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse Claude settings: {}", e))?
    } else {
        // Ensure .claude directory exists
        if let Some(parent) = settings_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create .claude directory: {}", e))?;
        }
        json!({})
    };

    // Generate and apply hooks
    let hooks = generate_woodeye_hooks();

    if let Some(obj) = settings.as_object_mut() {
        obj.insert("hooks".to_string(), hooks);
    }

    // Write updated settings
    let updated = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(&settings_path, updated)
        .map_err(|e| format!("Failed to write settings: {}", e))?;

    Ok(())
}
