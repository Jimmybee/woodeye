use crate::types::{
    BranchInfo, CommitDiff, CommitInfo, CreateWorktreeOptions, DiffHunk, DiffLine, DiffStats,
    FileDiff, FileStatus, HeadInfo, PruneResult, UpstreamInfo, Worktree, WorkingDiff,
    WorktreeStatus,
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

/// Get upstream tracking info for the current branch
/// Returns None if branch has no upstream or git command fails
fn get_upstream_info(path_str: &str) -> Option<UpstreamInfo> {
    // Get upstream branch name: git rev-parse --abbrev-ref @{upstream}
    let remote_branch = run_git(path_str, &["rev-parse", "--abbrev-ref", "@{upstream}"]).ok()?;
    let remote_branch = remote_branch.trim().to_string();

    // Get ahead/behind counts: git rev-list --count --left-right @{upstream}...HEAD
    let counts = run_git(
        path_str,
        &["rev-list", "--count", "--left-right", "@{upstream}...HEAD"],
    )
    .ok()
    .map(|s| parse_ahead_behind(&s))
    .unwrap_or((0, 0));

    Some(UpstreamInfo {
        remote_branch,
        behind: counts.0,
        ahead: counts.1,
    })
}

/// Parse the output of "git rev-list --count --left-right"
/// Output format: "behind\tahead" (tab-separated)
fn parse_ahead_behind(output: &str) -> (u32, u32) {
    let parts: Vec<&str> = output.trim().split('\t').collect();
    if parts.len() == 2 {
        (
            parts[0].parse().unwrap_or(0),
            parts[1].parse().unwrap_or(0),
        )
    } else {
        (0, 0)
    }
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

    // Get upstream tracking info if we have a branch (not detached)
    let upstream = if branch.is_some() {
        get_upstream_info(&path_str)
    } else {
        None
    };

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
            upstream,
        },
        status,
        last_commit_timestamp: timestamp,
    })
}

fn get_worktree_status(worktree_path: &str) -> Result<WorktreeStatus, String> {
    let output = run_git(worktree_path, &["status", "--porcelain"])?;
    Ok(parse_status_porcelain(&output))
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

    Ok(parse_commit_log(&output))
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

/// Create a new worktree
pub fn create_worktree(repo_path: &str, options: CreateWorktreeOptions) -> Result<Worktree, String> {
    let mut args = vec!["worktree", "add"];

    // Build temporary strings to hold the branch flag
    let branch_flag;
    if let Some(ref branch) = options.new_branch {
        branch_flag = format!("-b");
        args.push(&branch_flag);
        args.push(branch);
    }

    if options.detach {
        args.push("--detach");
    }

    args.push(&options.path);

    if let Some(ref commit_ish) = options.commit_ish {
        args.push(commit_ish);
    }

    run_git(repo_path, &args)?;

    // Build and return the new worktree info
    let path = PathBuf::from(&options.path);
    build_worktree_info(&path, false)
}

/// Delete a worktree
pub fn delete_worktree(repo_path: &str, worktree_path: &str, force: bool) -> Result<(), String> {
    let mut args = vec!["worktree", "remove"];

    if force {
        args.push("--force");
    }

    args.push(worktree_path);

    run_git(repo_path, &args)?;
    Ok(())
}

/// Prune stale worktree references
pub fn prune_worktrees(repo_path: &str) -> Result<PruneResult, String> {
    // First, do a dry run to see what would be pruned
    let dry_run_output = run_git(repo_path, &["worktree", "prune", "--dry-run"])?;

    let messages: Vec<String> = dry_run_output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect();

    let pruned_count = messages.len() as u32;

    // Actually prune
    run_git(repo_path, &["worktree", "prune"])?;

    Ok(PruneResult {
        pruned_count,
        messages,
    })
}

/// List all branches (local and remote)
pub fn list_branches(repo_path: &str) -> Result<Vec<BranchInfo>, String> {
    // Get list of checked out branches from worktrees
    let worktree_output = run_git(repo_path, &["worktree", "list", "--porcelain"])?;
    let mut checked_out_branches: Vec<String> = Vec::new();

    for line in worktree_output.lines() {
        if let Some(branch) = line.strip_prefix("branch refs/heads/") {
            checked_out_branches.push(branch.to_string());
        }
    }

    // Get all branches with format: refname, is_remote indicator
    // Using for-each-ref for better control over output
    let output = run_git(
        repo_path,
        &[
            "for-each-ref",
            "--format=%(refname:short)%09%(if)%(upstream)%(then)local%(else)%(if:equals=refs/remotes)%(refname:rstrip=-2)%(then)remote%(else)local%(end)%(end)",
            "refs/heads",
            "refs/remotes",
        ],
    )?;

    let mut branches: Vec<BranchInfo> = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        let name = parts[0].to_string();

        // Skip HEAD references from remotes
        if name.ends_with("/HEAD") {
            continue;
        }

        let is_remote = name.contains('/');
        let is_checked_out = if is_remote {
            false
        } else {
            checked_out_branches.contains(&name)
        };

        branches.push(BranchInfo {
            name,
            is_remote,
            is_checked_out,
        });
    }

    // Sort: local branches first, then remote, alphabetically within each group
    branches.sort_by(|a, b| {
        match (a.is_remote, b.is_remote) {
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    Ok(branches)
}

/// Parse git status --porcelain output into WorktreeStatus
/// Extracted for testability
fn parse_status_porcelain(output: &str) -> WorktreeStatus {
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

    WorktreeStatus {
        is_clean: modified == 0 && staged == 0 && untracked == 0 && conflicted == 0,
        modified,
        staged,
        untracked,
        conflicted,
    }
}

/// Parse git log output with record/unit separators into Vec<CommitInfo>
/// Extracted for testability
fn parse_commit_log(output: &str) -> Vec<CommitInfo> {
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

    commits
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== parse_range tests ====================

    #[test]
    fn test_parse_range_with_comma() {
        assert_eq!(parse_range("1,5"), Some((1, 5)));
        assert_eq!(parse_range("10,20"), Some((10, 20)));
        assert_eq!(parse_range("0,0"), Some((0, 0)));
    }

    #[test]
    fn test_parse_range_single_number() {
        // Single number implies 1 line
        assert_eq!(parse_range("1"), Some((1, 1)));
        assert_eq!(parse_range("10"), Some((10, 1)));
        assert_eq!(parse_range("0"), Some((0, 1)));
    }

    #[test]
    fn test_parse_range_invalid() {
        assert_eq!(parse_range(""), None);
        assert_eq!(parse_range("abc"), None);
        assert_eq!(parse_range("1,abc"), None);
        assert_eq!(parse_range("abc,1"), None);
    }

    // ==================== parse_hunk_header tests ====================

    #[test]
    fn test_parse_hunk_header_basic() {
        assert_eq!(
            parse_hunk_header("@@ -1,5 +1,7 @@"),
            Some((1, 5, 1, 7))
        );
        assert_eq!(
            parse_hunk_header("@@ -10,3 +12,5 @@"),
            Some((10, 3, 12, 5))
        );
    }

    #[test]
    fn test_parse_hunk_header_single_line() {
        // Single line changes (no comma means 1 line)
        assert_eq!(
            parse_hunk_header("@@ -1 +1 @@"),
            Some((1, 1, 1, 1))
        );
    }

    #[test]
    fn test_parse_hunk_header_with_context() {
        // Hunk headers can have function context after @@
        assert_eq!(
            parse_hunk_header("@@ -10,3 +12,5 @@ fn some_function()"),
            Some((10, 3, 12, 5))
        );
    }

    #[test]
    fn test_parse_hunk_header_new_file() {
        // New file: old side is 0,0
        assert_eq!(
            parse_hunk_header("@@ -0,0 +1,10 @@"),
            Some((0, 0, 1, 10))
        );
    }

    #[test]
    fn test_parse_hunk_header_deleted_file() {
        // Deleted file: new side is 0,0
        assert_eq!(
            parse_hunk_header("@@ -1,10 +0,0 @@"),
            Some((1, 10, 0, 0))
        );
    }

    #[test]
    fn test_parse_hunk_header_invalid() {
        assert_eq!(parse_hunk_header("not a hunk header"), None);
        assert_eq!(parse_hunk_header("@@ @@"), None);
        assert_eq!(parse_hunk_header("@@ -1 @@"), None);
    }

    // ==================== parse_git_diff_output tests ====================

    #[test]
    fn test_parse_diff_empty() {
        let files = parse_git_diff_output("");
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_diff_single_modified_file() {
        let diff = r#"diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!("Hello");
     println!("World");
 }
"#;
        let files = parse_git_diff_output(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/main.rs");
        assert!(matches!(files[0].status, FileStatus::Modified));
        assert!(!files[0].binary);
        assert_eq!(files[0].hunks.len(), 1);

        let hunk = &files[0].hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_lines, 3);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_lines, 4);
        assert_eq!(hunk.lines.len(), 4);

        // Check line types
        assert_eq!(hunk.lines[0].kind, ' '); // context
        assert_eq!(hunk.lines[1].kind, '+'); // addition
        assert_eq!(hunk.lines[2].kind, ' '); // context
        assert_eq!(hunk.lines[3].kind, ' '); // context
    }

    #[test]
    fn test_parse_diff_new_file() {
        let diff = r#"diff --git a/new_file.txt b/new_file.txt
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/new_file.txt
@@ -0,0 +1,2 @@
+line 1
+line 2
"#;
        let files = parse_git_diff_output(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "new_file.txt");
        assert!(matches!(files[0].status, FileStatus::Added));
    }

    #[test]
    fn test_parse_diff_deleted_file() {
        let diff = r#"diff --git a/old_file.txt b/old_file.txt
deleted file mode 100644
index abc1234..0000000
--- a/old_file.txt
+++ /dev/null
@@ -1,2 +0,0 @@
-line 1
-line 2
"#;
        let files = parse_git_diff_output(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "old_file.txt");
        assert!(matches!(files[0].status, FileStatus::Deleted));
    }

    #[test]
    fn test_parse_diff_renamed_file() {
        let diff = r#"diff --git a/old_name.rs b/new_name.rs
similarity index 95%
rename from old_name.rs
rename to new_name.rs
index abc1234..def5678 100644
--- a/old_name.rs
+++ b/new_name.rs
@@ -1,3 +1,3 @@
 fn main() {
-    old();
+    new();
 }
"#;
        let files = parse_git_diff_output(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "new_name.rs");
        assert!(matches!(files[0].status, FileStatus::Renamed));
        assert_eq!(files[0].old_path, Some("old_name.rs".to_string()));
    }

    #[test]
    fn test_parse_diff_binary_file() {
        let diff = r#"diff --git a/image.png b/image.png
new file mode 100644
index 0000000..abc1234
Binary files /dev/null and b/image.png differ
"#;
        let files = parse_git_diff_output(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "image.png");
        assert!(files[0].binary);
        assert!(files[0].hunks.is_empty());
    }

    #[test]
    fn test_parse_diff_multiple_files() {
        let diff = r#"diff --git a/file1.rs b/file1.rs
index abc..def 100644
--- a/file1.rs
+++ b/file1.rs
@@ -1 +1 @@
-old
+new
diff --git a/file2.rs b/file2.rs
index 123..456 100644
--- a/file2.rs
+++ b/file2.rs
@@ -1 +1 @@
-foo
+bar
"#;
        let files = parse_git_diff_output(diff);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "file1.rs");
        assert_eq!(files[1].path, "file2.rs");
    }

    #[test]
    fn test_parse_diff_multiple_hunks() {
        let diff = r#"diff --git a/file.rs b/file.rs
index abc..def 100644
--- a/file.rs
+++ b/file.rs
@@ -1,3 +1,3 @@
 fn foo() {
-    old1();
+    new1();
 }
@@ -10,3 +10,3 @@
 fn bar() {
-    old2();
+    new2();
 }
"#;
        let files = parse_git_diff_output(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].hunks.len(), 2);
        assert_eq!(files[0].hunks[0].old_start, 1);
        assert_eq!(files[0].hunks[1].old_start, 10);
    }

    // ==================== parse_status_porcelain tests ====================

    #[test]
    fn test_status_clean() {
        let status = parse_status_porcelain("");
        assert!(status.is_clean);
        assert_eq!(status.modified, 0);
        assert_eq!(status.staged, 0);
        assert_eq!(status.untracked, 0);
        assert_eq!(status.conflicted, 0);
    }

    #[test]
    fn test_status_modified_unstaged() {
        let status = parse_status_porcelain(" M src/main.rs\n");
        assert!(!status.is_clean);
        assert_eq!(status.modified, 1);
        assert_eq!(status.staged, 0);
    }

    #[test]
    fn test_status_staged() {
        let status = parse_status_porcelain("M  src/main.rs\n");
        assert!(!status.is_clean);
        assert_eq!(status.staged, 1);
        assert_eq!(status.modified, 0);
    }

    #[test]
    fn test_status_added_staged() {
        let status = parse_status_porcelain("A  new_file.rs\n");
        assert!(!status.is_clean);
        assert_eq!(status.staged, 1);
    }

    #[test]
    fn test_status_deleted_staged() {
        let status = parse_status_porcelain("D  old_file.rs\n");
        assert!(!status.is_clean);
        assert_eq!(status.staged, 1);
    }

    #[test]
    fn test_status_untracked() {
        let status = parse_status_porcelain("?? untracked.txt\n");
        assert!(!status.is_clean);
        assert_eq!(status.untracked, 1);
    }

    #[test]
    fn test_status_conflicts() {
        // UU - both modified (merge conflict)
        let status = parse_status_porcelain("UU conflicted.rs\n");
        assert!(!status.is_clean);
        assert_eq!(status.conflicted, 1);

        // AA - both added
        let status = parse_status_porcelain("AA both_added.rs\n");
        assert_eq!(status.conflicted, 1);

        // DD - both deleted
        let status = parse_status_porcelain("DD both_deleted.rs\n");
        assert_eq!(status.conflicted, 1);
    }

    #[test]
    fn test_status_mixed() {
        let output = "M  staged.rs\n M modified.rs\n?? untracked.txt\nUU conflict.rs\n";
        let status = parse_status_porcelain(output);
        assert!(!status.is_clean);
        assert_eq!(status.staged, 1);
        assert_eq!(status.modified, 1);
        assert_eq!(status.untracked, 1);
        assert_eq!(status.conflicted, 1);
    }

    // ==================== parse_commit_log tests ====================

    #[test]
    fn test_commit_log_single() {
        // Format: hash, short_hash, author_name, author_email, timestamp, summary, body
        let output = "abc123def456\x1fabc123\x1fJohn Doe\x1fjohn@example.com\x1f1700000000\x1fFix bug\x1fDetailed description\x1e";
        let commits = parse_commit_log(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].hash, "abc123def456");
        assert_eq!(commits[0].short_hash, "abc123");
        assert_eq!(commits[0].author_name, "John Doe");
        assert_eq!(commits[0].author_email, "john@example.com");
        assert_eq!(commits[0].timestamp, 1700000000);
        assert_eq!(commits[0].summary, "Fix bug");
        assert_eq!(commits[0].message, "Detailed description");
    }

    #[test]
    fn test_commit_log_multiple() {
        let output = "hash1\x1fh1\x1fAlice\x1falice@test.com\x1f1700000000\x1fFirst\x1fBody1\x1e\
                      hash2\x1fh2\x1fBob\x1fbob@test.com\x1f1700001000\x1fSecond\x1fBody2\x1e";
        let commits = parse_commit_log(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].summary, "First");
        assert_eq!(commits[1].summary, "Second");
    }

    #[test]
    fn test_commit_log_empty_body() {
        let output = "hash\x1fh\x1fName\x1femail\x1f1700000000\x1fSummary\x1f\x1e";
        let commits = parse_commit_log(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "");
    }

    #[test]
    fn test_commit_log_multiline_body() {
        let output = "hash\x1fh\x1fName\x1femail\x1f1700000000\x1fSummary\x1fLine1\nLine2\nLine3\x1e";
        let commits = parse_commit_log(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "Line1\nLine2\nLine3");
    }

    #[test]
    fn test_commit_log_empty() {
        let commits = parse_commit_log("");
        assert!(commits.is_empty());
    }

    #[test]
    fn test_commit_log_invalid_record() {
        // Too few fields - should be skipped
        let output = "hash\x1fh\x1fName\x1e";
        let commits = parse_commit_log(output);
        assert!(commits.is_empty());
    }
}
