use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worktree {
    pub path: PathBuf,
    pub name: String,
    pub is_main: bool,
    pub head: HeadInfo,
    /// Status is optional for lazy loading - initially None, fetched separately
    pub status: Option<WorktreeStatus>,
    pub last_commit_timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadInfo {
    pub branch: Option<String>,
    pub commit_sha: String,
    pub commit_message: String,
    pub upstream: Option<UpstreamInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamInfo {
    pub remote_branch: String,
    pub ahead: u32,
    pub behind: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeStatus {
    pub is_clean: bool,
    pub modified: u32,
    pub staged: u32,
    pub untracked: u32,
    pub conflicted: u32,
}

// Commit history types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: i64,
    pub message: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDiff {
    pub commit: CommitInfo,
    pub files: Vec<FileDiff>,
    pub stats: DiffStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub status: FileStatus,
    pub old_path: Option<String>,
    pub hunks: Vec<DiffHunk>,
    pub binary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub kind: char,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

// Working directory (uncommitted) changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingDiff {
    pub staged_files: Vec<FileDiff>,
    pub unstaged_files: Vec<FileDiff>,
    pub stats: DiffStats,
}

// Worktree management types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorktreeOptions {
    pub path: String,
    pub new_branch: Option<String>,
    pub commit_ish: Option<String>,
    pub detach: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneResult {
    pub pruned_count: u32,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_remote: bool,
    pub is_checked_out: bool,
}
