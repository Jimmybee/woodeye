<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import WorktreeSelector from "./lib/components/WorktreeSelector.svelte";
  import CommitList from "./lib/components/CommitList.svelte";
  import CommitDiffView from "./lib/components/CommitDiffView.svelte";
  import type { Worktree, CommitInfo, CommitDiff, WorkingDiff, WorktreeStatus } from "./lib/types";
  import { getLastRepoPath, saveLastRepoPath } from "./lib/store";

  const COMMITS_PER_PAGE = 10;

  let worktrees: Worktree[] = $state([]);
  let selectedWorktree: Worktree | null = $state(null);
  let commits: CommitInfo[] = $state([]);
  let selectedCommit: CommitInfo | null = $state(null);
  let commitDiff: CommitDiff | null = $state(null);
  let workingDiff: WorkingDiff | null = $state(null);
  let workingSelected = $state(false);

  let repoPath = $state("");
  let loading = $state(false);
  let loadingCommits = $state(false);
  let loadingDiff = $state(false);
  let hasMoreCommits = $state(true);
  let error = $state("");
  let unlisten: UnlistenFn | null = null;

  async function loadWorktrees(path: string) {
    if (!path.trim()) return;

    loading = true;
    error = "";
    worktrees = [];
    commits = [];
    selectedWorktree = null;
    selectedCommit = null;
    commitDiff = null;

    try {
      const result = await invoke<Worktree[]>("list_worktrees", {
        repoPath: path,
      });
      worktrees = result;
      saveLastRepoPath(path);

      // Auto-select first worktree
      if (result.length > 0) {
        await selectWorktree(result[0]);
      }

      // Start watching for changes (don't await)
      const watchPaths = result.map((w) => w.path);
      invoke("start_watching", { paths: watchPaths });

      // Load status for all worktrees in background
      loadAllWorktreeStatuses(result);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  /** Load status for all worktrees in background (non-blocking) */
  async function loadAllWorktreeStatuses(trees: Worktree[]) {
    for (const wt of trees) {
      loadWorktreeStatus(wt.path);
    }
  }

  /** Load status for a single worktree and update state */
  async function loadWorktreeStatus(worktreePath: string) {
    try {
      const status = await invoke<WorktreeStatus>("get_worktree_status", {
        worktreePath,
      });
      // Update the worktree in the array with the new status
      worktrees = worktrees.map((wt) =>
        wt.path === worktreePath ? { ...wt, status } : wt
      );
      // Also update selectedWorktree if it matches
      if (selectedWorktree?.path === worktreePath) {
        selectedWorktree = { ...selectedWorktree, status };
      }
    } catch (e) {
      console.error(`Failed to load status for ${worktreePath}:`, e);
    }
  }

  async function selectWorktree(worktree: Worktree) {
    selectedWorktree = worktree;
    commits = [];
    selectedCommit = null;
    commitDiff = null;
    workingDiff = null;
    workingSelected = false;
    hasMoreCommits = true;

    await loadCommits(false);
  }

  async function loadCommits(append: boolean) {
    if (!selectedWorktree || loadingCommits) return;

    loadingCommits = true;
    const offset = append ? commits.length : 0;

    try {
      const result = await invoke<CommitInfo[]>("get_commit_history", {
        worktreePath: selectedWorktree.path,
        limit: COMMITS_PER_PAGE,
        offset,
      });

      if (append) {
        commits = [...commits, ...result];
      } else {
        commits = result;
      }

      hasMoreCommits = result.length === COMMITS_PER_PAGE;

      // Don't auto-select commit on startup - let user choose to avoid loading diffs
    } catch (e) {
      console.error("Failed to load commits:", e);
    } finally {
      loadingCommits = false;
    }
  }

  async function selectCommit(commit: CommitInfo) {
    selectedCommit = commit;
    commitDiff = null;
    workingDiff = null;
    workingSelected = false;

    if (!selectedWorktree) return;

    loadingDiff = true;
    try {
      commitDiff = await invoke<CommitDiff>("get_commit_diff", {
        worktreePath: selectedWorktree.path,
        commitSha: commit.hash,
      });
    } catch (e) {
      console.error("Failed to load diff:", e);
    } finally {
      loadingDiff = false;
    }
  }

  async function selectWorkingChanges() {
    workingSelected = true;
    selectedCommit = null;
    commitDiff = null;
    workingDiff = null;

    if (!selectedWorktree) return;

    loadingDiff = true;
    try {
      workingDiff = await invoke<WorkingDiff>("get_working_diff", {
        worktreePath: selectedWorktree.path,
      });
    } catch (e) {
      console.error("Failed to load working diff:", e);
    } finally {
      loadingDiff = false;
    }
  }

  async function refreshWorktrees() {
    if (!repoPath.trim() || loading) return;

    try {
      const result = await invoke<Worktree[]>("list_worktrees", {
        repoPath: repoPath,
      });
      worktrees = result;
      // Load status in background
      loadAllWorktreeStatuses(result);
    } catch (e) {
      console.error("Refresh error:", e);
    }
  }

  onMount(() => {
    listen("worktree-changed", () => {
      refreshWorktrees();
    }).then((fn) => {
      unlisten = fn;
    });

    const lastRepo = getLastRepoPath();
    if (lastRepo) {
      repoPath = lastRepo;
      loadWorktrees(lastRepo);
    }

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  });
</script>

<div class="app-layout">
  <aside class="sidebar">
    <div class="sidebar-header">
      <div class="logo">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="3"/>
          <path d="M12 2v4m0 12v4M2 12h4m12 0h4"/>
          <path d="M4.93 4.93l2.83 2.83m8.48 8.48l2.83 2.83M4.93 19.07l2.83-2.83m8.48-8.48l2.83-2.83"/>
        </svg>
        <span>Woodeye</span>
      </div>
    </div>

    <WorktreeSelector
      {worktrees}
      {selectedWorktree}
      bind:repoPath
      onSelectWorktree={selectWorktree}
      onLoadRepo={loadWorktrees}
      {loading}
    />
  </aside>

  <main class="main-content">
    {#if error}
      <div class="error-banner">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/>
          <line x1="12" y1="8" x2="12" y2="12"/>
          <line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
        <span>{error}</span>
      </div>
    {:else if worktrees.length > 0}
      <div class="content-header">
        <h1>
          {selectedWorktree?.name ?? "Repository"}
          {#if selectedWorktree?.head.branch}
            <span class="branch-badge">{selectedWorktree.head.branch}</span>
          {/if}
        </h1>
        {#if selectedWorktree}
          <div class="stats-row">
            <div class="stat-card">
              <span class="stat-value">{commits.length}{hasMoreCommits ? '+' : ''}</span>
              <span class="stat-label">Commits loaded</span>
            </div>
            {#if selectedWorktree.status}
              <div class="stat-card" class:has-changes={!selectedWorktree.status.is_clean}>
                <span class="stat-value">{selectedWorktree.status.modified}</span>
                <span class="stat-label">Modified</span>
              </div>
              <div class="stat-card" class:has-additions={selectedWorktree.status.staged > 0}>
                <span class="stat-value">{selectedWorktree.status.staged}</span>
                <span class="stat-label">Staged</span>
              </div>
              <div class="stat-card">
                <span class="stat-value">{selectedWorktree.status.untracked}</span>
                <span class="stat-label">Untracked</span>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <div class="split-view">
        <section class="commits-panel">
          <div class="panel-header">
            <h2>Commits</h2>
          </div>
          <CommitList
            {commits}
            {selectedCommit}
            {workingSelected}
            worktreeStatus={selectedWorktree?.status ?? null}
            onSelectCommit={selectCommit}
            onSelectWorking={selectWorkingChanges}
            onLoadMore={() => loadCommits(true)}
            hasMore={hasMoreCommits}
            loading={loadingCommits}
          />
        </section>
        <section class="diff-panel">
          <div class="panel-header">
            <h2>Changes</h2>
          </div>
          <CommitDiffView
            diff={commitDiff}
            {workingDiff}
            loading={loadingDiff}
          />
        </section>
      </div>
    {:else if !loading}
      <div class="empty-state">
        <div class="empty-icon">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
          </svg>
        </div>
        <h2>No repository loaded</h2>
        <p>Enter a repository path or browse to get started</p>
      </div>
    {:else}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>Loading repository...</p>
      </div>
    {/if}
  </main>
</div>

<style>
  .app-layout {
    height: 100%;
    display: flex;
    overflow: hidden;
  }

  .sidebar {
    width: 280px;
    min-width: 240px;
    background: var(--color-bg-sidebar);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    padding: var(--space-lg);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  .logo {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    color: var(--color-text-sidebar-active);
    font-weight: 600;
    font-size: 1.1rem;
  }

  .logo svg {
    color: var(--color-primary);
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--color-bg);
  }

  .content-header {
    padding: var(--space-xl);
    background: var(--color-bg-card);
    border-bottom: 1px solid var(--color-border);
  }

  .content-header h1 {
    font-size: 1.5rem;
    font-weight: 600;
    margin-bottom: var(--space-md);
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .branch-badge {
    font-size: 0.75rem;
    font-weight: 500;
    padding: var(--space-xs) var(--space-sm);
    background: var(--color-primary-light);
    color: var(--color-primary);
    border-radius: var(--radius-sm);
  }

  .stats-row {
    display: flex;
    gap: var(--space-lg);
  }

  .stat-card {
    display: flex;
    flex-direction: column;
    padding: var(--space-md) var(--space-lg);
    background: var(--color-bg);
    border-radius: var(--radius-md);
    min-width: 100px;
  }

  .stat-card.has-changes {
    background: rgba(245, 158, 11, 0.1);
  }

  .stat-card.has-changes .stat-value {
    color: var(--color-warning);
  }

  .stat-card.has-additions {
    background: rgba(34, 197, 94, 0.1);
  }

  .stat-card.has-additions .stat-value {
    color: var(--color-success);
  }

  .stat-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-text);
  }

  .stat-label {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-md) var(--space-lg);
    background: rgba(248, 113, 113, 0.1);
    color: var(--color-error);
    font-size: 0.9rem;
    border-bottom: 1px solid var(--color-error);
  }

  .empty-state,
  .loading-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--color-text-muted);
    gap: var(--space-md);
  }

  .empty-icon {
    color: var(--color-border);
    margin-bottom: var(--space-md);
  }

  .empty-state h2 {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--color-text);
  }

  .empty-state p {
    font-size: 0.9rem;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--color-border);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .split-view {
    flex: 1;
    display: flex;
    overflow: hidden;
    padding: var(--space-xl);
    gap: var(--space-xl);
  }

  .commits-panel {
    width: 340px;
    min-width: 280px;
    background: var(--color-bg-card);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-md);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .diff-panel {
    flex: 1;
    background: var(--color-bg-card);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-md);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel-header {
    padding: var(--space-lg);
    border-bottom: 1px solid var(--color-border);
  }

  .panel-header h2 {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--color-text);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
</style>
