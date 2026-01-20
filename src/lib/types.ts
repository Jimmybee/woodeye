export interface Worktree {
  path: string;
  name: string;
  is_main: boolean;
  head: HeadInfo;
  /** Status is optional for lazy loading - initially null, fetched separately */
  status: WorktreeStatus | null;
  last_commit_timestamp: number;
}

export interface HeadInfo {
  branch: string | null;
  commit_sha: string;
  commit_message: string;
  upstream: UpstreamInfo | null;
}

export interface UpstreamInfo {
  remote_branch: string;
  ahead: number;
  behind: number;
}

export interface WorktreeStatus {
  is_clean: boolean;
  modified: number;
  staged: number;
  untracked: number;
  conflicted: number;
}

// Commit history types
export interface CommitInfo {
  hash: string;
  short_hash: string;
  author_name: string;
  author_email: string;
  timestamp: number;
  message: string;
  summary: string;
}

export interface CommitDiff {
  commit: CommitInfo;
  files: FileDiff[];
  stats: DiffStats;
}

export interface FileDiff {
  path: string;
  status: FileStatus;
  old_path: string | null;
  hunks: DiffHunk[];
  binary: boolean;
}

export type FileStatus = "Added" | "Modified" | "Deleted" | "Renamed";

export interface DiffHunk {
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  header: string;
  lines: DiffLine[];
}

export interface DiffLine {
  kind: string;
  content: string;
}

export interface DiffStats {
  files_changed: number;
  insertions: number;
  deletions: number;
}

// Working directory (uncommitted) changes
export interface WorkingDiff {
  staged_files: FileDiff[];
  unstaged_files: FileDiff[];
  stats: DiffStats;
}

// Worktree management types
export interface CreateWorktreeOptions {
  path: string;
  new_branch: string | null;
  commit_ish: string | null;
  detach: boolean;
}

export interface PruneResult {
  pruned_count: number;
  messages: string[];
}

export interface BranchInfo {
  name: string;
  is_remote: boolean;
  is_checked_out: boolean;
}

// Claude session types
export type ClaudeSessionState =
  | "working"
  | "waiting_for_approval"
  | "waiting_for_input"
  | "idle"
  | "unknown";

export interface ClaudeSession {
  session_id: string;
  project_path: string;
  state: ClaudeSessionState;
  waiting_reason: string | null;
  timestamp: number;
  /** Last tool that was invoked (for tool-aware timeouts) */
  last_tool: string | null;
}

export interface WorktreeClaudeStatus {
  active_sessions: ClaudeSession[];
  has_pending_input: boolean;
}

export interface ClaudeHooksConfig {
  configured: boolean;
  status_dir_exists: boolean;
}

// Debug window types
export interface StatusFileInfo {
  filename: string;
  project_path: string;
  state: string;
  timestamp: number;
  age_seconds: number;
  is_stale: boolean;
}

export interface DebugInfo {
  status_dir: string;
  status_files: StatusFileInfo[];
  hooks_configured: boolean;
  current_timestamp: number;
  stale_threshold_secs: number;
}
