use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub fn start_watching(app: AppHandle, paths: Vec<String>) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(200), tx).map_err(|e| e.to_string())?;

    for path_str in &paths {
        let path = Path::new(path_str);
        // Watch the .git directory if it exists, otherwise the path itself
        let watch_path = path.join(".git");
        let target = if watch_path.exists() {
            watch_path
        } else {
            path.to_path_buf()
        };

        debouncer
            .watcher()
            .watch(&target, notify::RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch {}: {}", target.display(), e))?;
    }

    // Store the debouncer in app state to keep it alive
    app.manage(WatcherState { _debouncer: debouncer });

    // Spawn thread to handle events
    let app_handle = app.clone();
    std::thread::spawn(move || {
        while let Ok(result) = rx.recv() {
            match result {
                Ok(events) => {
                    // Only emit if there are actual changes
                    let has_changes = events
                        .iter()
                        .any(|e| matches!(e.kind, DebouncedEventKind::Any));
                    if has_changes {
                        let _ = app_handle.emit("worktree-changed", ());
                    }
                }
                Err(e) => eprintln!("Watch error: {:?}", e),
            }
        }
    });

    Ok(())
}

// State to keep the debouncer alive
struct WatcherState {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}
