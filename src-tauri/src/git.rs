use crate::types::{
    CommitDiff, CommitInfo, DiffHunk, DiffLine, DiffStats, FileDiff, FileStatus, HeadInfo,
    Worktree, WorkingDiff, WorktreeStatus,
};
use rayon::prelude::*;
use std::path::PathBuf;
use std::process::Command;

/// Run a git command in the specified directory and return stdout as String
fn run_git(path: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run git {}: {}", args.join(" "), e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git {} failed: {}", args.join(" "), stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn get_all_worktrees(repo_path: &str) -> Result<Vec<Worktree>, String> {
    // Use git worktree list --porcelain to get all worktrees
    let output = run_git(repo_path, &["worktree", "list", "--porcelain"])?;

    // Parse porcelain output to extract paths
    // Format:
    // worktree /path/to/main
    // HEAD abc1234...
    // branch refs/heads/main
    //
    // worktree /path/to/linked
    // HEAD def5678...
    // branch refs/heads/feature

    let mut worktree_paths: Vec<PathBuf> = Vec::new();

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            worktree_paths.push(PathBuf::from(path));
        }
    }

    // Process all worktrees in parallel using rayon
    let mut worktrees: Vec<Worktree> = worktree_paths
        .par_iter()
        .enumerate()
        .filter_map(|(idx, path)| {
            let is_main = idx == 0; // First worktree is the main one
            build_worktree_info(path, is_main).ok()
        })
        .collect();

    // Sort by last commit timestamp (most recent first)
    worktrees.sort_by(|a, b| b.last_commit_timestamp.cmp(&a.last_commit_timestamp));

    Ok(worktrees)
}

/// Get status for a single worktree path (for lazy loading)
pub fn get_worktree_status_by_path(worktree_path: &str) -> Result<WorktreeStatus, String> {
    get_worktree_status(worktree_path)
}

fn build_worktree_info(path: &PathBuf, is_main: bool) -> Result<Worktree, String> {
    let path_str = path.to_string_lossy();

    // Get short SHA
    let short_sha = run_git(&path_str, &["rev-parse", "--short", "HEAD"])?
        .trim()
        .to_string();

    // Get branch name (returns "HEAD" if detached)
    let branch_output = run_git(&path_str, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let branch_name = branch_output.trim();
    let branch = if branch_name == "HEAD" {
        None // Detached HEAD
    } else {
        Some(branch_name.to_string())
    };

    // Get commit message summary
    let commit_message = run_git(&path_str, &["log", "-1", "--format=%s"])?
        .trim()
        .to_string();

    // Get commit timestamp
    let timestamp_str = run_git(&path_str, &["log", "-1", "--format=%ct"])?;
    let timestamp = timestamp_str.trim().parse::<i64>().unwrap_or(0);

    // Defer status scanning - return None initially for faster load
    // Frontend will fetch status lazily
    let status = None;

    Ok(Worktree {
        path: path.clone(),
        name: path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string(),
        is_main,
        head: HeadInfo {
            branch,
            commit_sha: short_sha,
            commit_message,
        },
        status,
        last_commit_timestamp: timestamp,
    })
}

fn get_worktree_status(worktree_path: &str) -> Result<WorktreeStatus, String> {
    let output = run_git(worktree_path, &["status", "--porcelain"])?;

    let mut modified = 0u32;
    let mut staged = 0u32;
    let mut untracked = 0u32;
    let mut conflicted = 0u32;

    for line in output.lines() {
        if line.len() < 2 {
            continue;
        }

        let index_status = line.chars().next().unwrap_or(' ');
        let worktree_status = line.chars().nth(1).unwrap_or(' ');

        // Check for conflicts (UU, AA, DD, AU, UA, DU, UD)
        if matches!(
            (index_status, worktree_status),
            ('U', 'U') | ('A', 'A') | ('D', 'D') | ('A', 'U') | ('U', 'A') | ('D', 'U') | ('U', 'D')
        ) {
            conflicted += 1;
        } else if index_status == '?' && worktree_status == '?' {
            // Untracked
            untracked += 1;
        } else {
            // Check staged changes (index column)
            if matches!(index_status, 'M' | 'A' | 'D' | 'R' | 'C') {
                staged += 1;
            }
            // Check unstaged changes (worktree column) - only if not already counted as staged
            else if matches!(worktree_status, 'M' | 'D') {
                modified += 1;
            }
        }
    }

    Ok(WorktreeStatus {
        is_clean: modified == 0 && staged == 0 && untracked == 0 && conflicted == 0,
        modified,
        staged,
        untracked,
        conflicted,
    })
}

// Get commit history for a worktree
pub fn get_commit_history(
    worktree_path: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<CommitInfo>, String> {
    // Use record separator (%x1e) between commits and unit separator (%x1f) between fields
    // Format: hash, short_hash, author_name, author_email, timestamp, summary, body
    let format = "%H%x1f%h%x1f%an%x1f%ae%x1f%ct%x1f%s%x1f%B%x1e";

    let output = run_git(
        worktree_path,
        &[
            "log",
            &format!("--format={}", format),
            &format!("--skip={}", offset),
            &format!("-n{}", limit),
        ],
    )?;

    let mut commits = Vec::new();

    for record in output.split('\x1e') {
        let record = record.trim();
        if record.is_empty() {
            continue;
        }

        let fields: Vec<&str> = record.split('\x1f').collect();
        if fields.len() < 6 {
            continue;
        }

        let hash = fields[0].to_string();
        let short_hash = fields[1].to_string();
        let author_name = fields[2].to_string();
        let author_email = fields[3].to_string();
        let timestamp = fields[4].parse::<i64>().unwrap_or(0);
        let summary = fields[5].to_string();
        let message = fields.get(6).unwrap_or(&"").trim().to_string();

        commits.push(CommitInfo {
            hash,
            short_hash,
            author_name,
            author_email,
            timestamp,
            message,
            summary,
        });
    }

    Ok(commits)
}

// Get diff for a specific commit
pub fn get_commit_diff(worktree_path: &str, commit_sha: &str) -> Result<CommitDiff, String> {
    // Get commit info using git log
    let format = "%H%x1f%h%x1f%an%x1f%ae%x1f%ct%x1f%s%x1f%B";
    let commit_output = run_git(
        worktree_path,
        &["log", "-1", &format!("--format={}", format), commit_sha],
    )?;

    let fields: Vec<&str> = commit_output.trim().split('\x1f').collect();
    if fields.len() < 6 {
        return Err(format!("Failed to parse commit info for {}", commit_sha));
    }

    let commit_info = CommitInfo {
        hash: fields[0].to_string(),
        short_hash: fields[1].to_string(),
        author_name: fields[2].to_string(),
        author_email: fields[3].to_string(),
        timestamp: fields[4].parse::<i64>().unwrap_or(0),
        summary: fields[5].to_string(),
        message: fields.get(6).unwrap_or(&"").trim().to_string(),
    };

    // Get diff using git show
    let diff_output = run_git(
        worktree_path,
        &["show", commit_sha, "--format=", "-U3", "-M"],
    )?;

    let files = parse_git_diff_output(&diff_output);

    // Calculate stats
    let mut total_insertions = 0u32;
    let mut total_deletions = 0u32;

    for file in &files {
        for hunk in &file.hunks {
            for line in &hunk.lines {
                match line.kind {
                    '+' => total_insertions += 1,
                    '-' => total_deletions += 1,
                    _ => {}
                }
            }
        }
    }

    let files_changed = files.len() as u32;

    Ok(CommitDiff {
        commit: commit_info,
        files,
        stats: DiffStats {
            files_changed,
            insertions: total_insertions,
            deletions: total_deletions,
        },
    })
}

// Get uncommitted working directory changes using git CLI
pub fn get_working_diff(worktree_path: &str) -> Result<WorkingDiff, String> {
    // Get staged changes: git diff --cached
    let staged_diff_text = run_git(worktree_path, &["diff", "--cached", "-U3"])?;
    let staged_files = parse_git_diff_output(&staged_diff_text);

    // Get unstaged changes: git diff
    let unstaged_diff_text = run_git(worktree_path, &["diff", "-U3"])?;
    let mut unstaged_files = parse_git_diff_output(&unstaged_diff_text);

    // Get untracked files: git ls-files --others --exclude-standard
    let untracked_text = run_git(worktree_path, &["ls-files", "--others", "--exclude-standard"])?;
    for line in untracked_text.lines() {
        if !line.is_empty() {
            unstaged_files.push(FileDiff {
                path: line.to_string(),
                status: FileStatus::Added,
                old_path: None,
                hunks: Vec::new(), // Untracked files don't have hunks
                binary: false,
            });
        }
    }

    // Calculate total stats
    let mut total_insertions = 0u32;
    let mut total_deletions = 0u32;

    for file in staged_files.iter().chain(unstaged_files.iter()) {
        for hunk in &file.hunks {
            for line in &hunk.lines {
                match line.kind {
                    '+' => total_insertions += 1,
                    '-' => total_deletions += 1,
                    _ => {}
                }
            }
        }
    }

    let files_changed = (staged_files.len() + unstaged_files.len()) as u32;

    Ok(WorkingDiff {
        staged_files,
        unstaged_files,
        stats: DiffStats {
            files_changed,
            insertions: total_insertions,
            deletions: total_deletions,
        },
    })
}

/// Parse git diff output into Vec<FileDiff>
fn parse_git_diff_output(diff_text: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut current_file: Option<FileDiff> = None;
    let mut current_hunk: Option<DiffHunk> = None;

    for line in diff_text.lines() {
        // New file header: diff --git a/path b/path
        if line.starts_with("diff --git ") {
            // Save previous file
            if let Some(mut file) = current_file.take() {
                if let Some(hunk) = current_hunk.take() {
                    file.hunks.push(hunk);
                }
                files.push(file);
            }

            // Extract path from "diff --git a/path b/path"
            let parts: Vec<&str> = line.split(" b/").collect();
            let path = if parts.len() >= 2 {
                parts[1].to_string()
            } else {
                // Fallback: extract from a/ part
                line.split(" a/")
                    .nth(1)
                    .and_then(|s| s.split(' ').next())
                    .unwrap_or("")
                    .to_string()
            };

            current_file = Some(FileDiff {
                path,
                status: FileStatus::Modified, // Will be updated below
                old_path: None,
                hunks: Vec::new(),
                binary: false,
            });
            continue;
        }

        // Check for binary file
        if line.starts_with("Binary files") {
            if let Some(ref mut file) = current_file {
                file.binary = true;
            }
            continue;
        }

        // New file indicator
        if line.starts_with("new file mode") {
            if let Some(ref mut file) = current_file {
                file.status = FileStatus::Added;
            }
            continue;
        }

        // Deleted file indicator
        if line.starts_with("deleted file mode") {
            if let Some(ref mut file) = current_file {
                file.status = FileStatus::Deleted;
            }
            continue;
        }

        // Rename detection: rename from / rename to
        if line.starts_with("rename from ") {
            if let Some(ref mut file) = current_file {
                file.old_path = Some(line.trim_start_matches("rename from ").to_string());
                file.status = FileStatus::Renamed;
            }
            continue;
        }

        // Hunk header: @@ -old_start,old_lines +new_start,new_lines @@
        if line.starts_with("@@ ") {
            // Save previous hunk
            if let Some(ref mut file) = current_file {
                if let Some(hunk) = current_hunk.take() {
                    file.hunks.push(hunk);
                }
            }

            // Parse hunk header
            if let Some((old_start, old_lines, new_start, new_lines)) = parse_hunk_header(line) {
                current_hunk = Some(DiffHunk {
                    old_start,
                    old_lines,
                    new_start,
                    new_lines,
                    header: line.to_string(),
                    lines: Vec::new(),
                });
            }
            continue;
        }

        // Diff lines: +, -, or space (context)
        if let Some(ref mut hunk) = current_hunk {
            let first_char = line.chars().next();
            match first_char {
                Some('+') => {
                    hunk.lines.push(DiffLine {
                        kind: '+',
                        content: line[1..].to_string(),
                    });
                }
                Some('-') => {
                    hunk.lines.push(DiffLine {
                        kind: '-',
                        content: line[1..].to_string(),
                    });
                }
                Some(' ') => {
                    hunk.lines.push(DiffLine {
                        kind: ' ',
                        content: line[1..].to_string(),
                    });
                }
                _ => {}
            }
        }
    }

    // Save last file
    if let Some(mut file) = current_file {
        if let Some(hunk) = current_hunk {
            file.hunks.push(hunk);
        }
        files.push(file);
    }

    files
}

/// Parse hunk header like "@@ -1,5 +1,7 @@" into (old_start, old_lines, new_start, new_lines)
fn parse_hunk_header(line: &str) -> Option<(u32, u32, u32, u32)> {
    // Format: @@ -old_start,old_lines +new_start,new_lines @@
    let line = line.trim_start_matches("@@ ");
    let parts: Vec<&str> = line.split(" @@").next()?.split(' ').collect();

    if parts.len() < 2 {
        return None;
    }

    let old_part = parts[0].trim_start_matches('-');
    let new_part = parts[1].trim_start_matches('+');

    let (old_start, old_lines) = parse_range(old_part)?;
    let (new_start, new_lines) = parse_range(new_part)?;

    Some((old_start, old_lines, new_start, new_lines))
}

/// Parse "start,lines" or just "start" (implies lines=1)
fn parse_range(s: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = s.split(',').collect();
    let start = parts[0].parse().ok()?;
    let lines = if parts.len() > 1 {
        parts[1].parse().ok()?
    } else {
        1
    };
    Some((start, lines))
}
