<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { ask, message } from "@tauri-apps/plugin-dialog";
  import ContentToolbar from "./lib/components/ContentToolbar.svelte";
  import CommitList from "./lib/components/CommitList.svelte";
  import CommitDiffView from "./lib/components/CommitDiffView.svelte";
  import CreateWorktreeDialog from "./lib/components/CreateWorktreeDialog.svelte";
  import type {
    Worktree,
    CommitInfo,
    CommitDiff,
    WorkingDiff,
    WorktreeStatus,
    BranchInfo,
    CreateWorktreeOptions,
    PruneResult,
    WorktreeClaudeStatus,
    ClaudeHooksConfig,
  } from "./lib/types";
  import { getLastRepoPath, saveLastRepoPath, getTheme, setTheme, type Theme } from "./lib/store";

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
  let refreshing = $state(false);
  let hasExternalChanges = $state(false);
  let unlisten: UnlistenFn | null = null;
  let unlistenTheme: UnlistenFn | null = null;
  let unlistenClaude: UnlistenFn | null = null;

  // Claude status state
  let claudeStatuses: Map<string, WorktreeClaudeStatus> = $state(new Map());
  let claudeHooksConfig: ClaudeHooksConfig | null = $state(null);

  // Working diff cache (keyed by worktree path)
  let workingDiffCache: Map<string, WorkingDiff> = $state(new Map());
  let refreshTimeout: ReturnType<typeof setTimeout> | null = null;

  // Dialog state
  let showCreateDialog = $state(false);
  let branches: BranchInfo[] = $state([]);

  // Get main worktree path for the dialog
  let mainWorktreePath = $derived(
    worktrees.find((w) => w.is_main)?.path ?? ""
  );

  async function loadWorktrees(path: string) {
    if (!path.trim()) return;

    loading = true;
    error = "";
    worktrees = [];
    commits = [];
    selectedWorktree = null;
    selectedCommit = null;
    commitDiff = null;
    workingDiffCache = new Map(); // Clear cache when loading new repo

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

      // Load Claude statuses for all worktrees
      loadClaudeStatuses();
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
    loadingDiff = true;  // Set loading immediately to prevent empty state flicker

    // Load commits and status in parallel
    const [, status] = await Promise.all([
      loadCommits(false),
      invoke<WorktreeStatus>("get_worktree_status", { worktreePath: worktree.path }),
    ]);

    // Update worktree with status
    if (selectedWorktree?.path === worktree.path) {
      selectedWorktree = { ...selectedWorktree, status };
      worktrees = worktrees.map((wt) =>
        wt.path === worktree.path ? { ...wt, status } : wt
      );

      // Auto-select working changes if dirty, otherwise first commit
      // All state updates happen synchronously to prevent flicker
      if (!status.is_clean) {
        workingSelected = true;
        fetchWorkingDiff(worktree.path, false);
      } else if (commits.length > 0) {
        await selectCommit(commits[0]);
      } else {
        loadingDiff = false;
      }
    }
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

    if (!selectedWorktree) {
      workingDiff = null;
      return;
    }

    // Set loading before clearing diff to show spinner instead of empty state
    loadingDiff = true;
    workingDiff = null;

    await fetchWorkingDiff(selectedWorktree.path, false);
  }

  /** Fetch working diff with optional caching */
  async function fetchWorkingDiff(worktreePath: string, skipCache = false) {
    // Check cache unless skipping
    if (!skipCache) {
      const cached = workingDiffCache.get(worktreePath);
      if (cached) {
        workingDiff = cached;
        loadingDiff = false;
        return;
      }
    }

    // Only show loading spinner if we don't already have content
    // This prevents flicker when refreshing due to file watcher
    if (!workingDiff) {
      loadingDiff = true;
    }

    try {
      const result = await invoke<WorkingDiff>("get_working_diff", { worktreePath });
      if (selectedWorktree?.path === worktreePath) {
        workingDiff = result;
        workingDiffCache.set(worktreePath, result);
        workingDiffCache = new Map(workingDiffCache);
      }
    } catch (e) {
      console.error("Failed to load working diff:", e);
    } finally {
      loadingDiff = false;
    }
  }

  /** Debounced refresh for working diff when file system changes */
  function refreshWorkingDiffDebounced() {
    if (refreshTimeout) clearTimeout(refreshTimeout);
    refreshTimeout = setTimeout(() => {
      if (workingSelected && selectedWorktree) {
        fetchWorkingDiff(selectedWorktree.path, true); // force refresh, skip cache
      }
    }, 300);
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

  async function refreshAll() {
    if (!repoPath.trim() || refreshing) return;

    refreshing = true;
    error = "";

    try {
      // Refresh worktrees
      const result = await invoke<Worktree[]>("list_worktrees", {
        repoPath: repoPath,
      });
      worktrees = result;

      // Load statuses in background
      loadAllWorktreeStatuses(result);

      // If a worktree was selected, refresh its commits
      if (selectedWorktree) {
        const updatedWorktree = result.find((w) => w.path === selectedWorktree!.path);
        if (updatedWorktree) {
          selectedWorktree = updatedWorktree;
          commits = [];
          hasMoreCommits = true;
          await loadCommits(false);

          // Refresh the currently viewed diff if any
          if (workingSelected) {
            await selectWorkingChanges();
          } else if (selectedCommit) {
            await selectCommit(selectedCommit);
          }
        }
      }

      hasExternalChanges = false;
    } catch (e) {
      error = String(e);
    } finally {
      refreshing = false;
    }
  }

  async function openCreateDialog() {
    if (!repoPath.trim()) return;

    try {
      branches = await invoke<BranchInfo[]>("list_branches", {
        repoPath: repoPath,
      });
      showCreateDialog = true;
    } catch (e) {
      error = String(e);
    }
  }

  async function handleCreateWorktree(options: CreateWorktreeOptions) {
    await invoke("create_worktree", {
      repoPath: repoPath,
      options,
    });
    await refreshWorktrees();
  }

  async function handleDeleteWorktree(worktree: Worktree) {
    const hasChanges = worktree.status && !worktree.status.is_clean;

    let confirmed = await ask(
      hasChanges
        ? `"${worktree.name}" has uncommitted changes. Are you sure you want to delete it?`
        : `Are you sure you want to delete the worktree "${worktree.name}"?`,
      {
        title: "Delete Worktree",
        kind: hasChanges ? "warning" : "info",
        okLabel: hasChanges ? "Force Delete" : "Delete",
        cancelLabel: "Cancel",
      }
    );

    if (!confirmed) return;

    try {
      await invoke("delete_worktree", {
        repoPath: repoPath,
        worktreePath: worktree.path,
        force: hasChanges,
      });

      // If the deleted worktree was selected, clear selection
      if (selectedWorktree?.path === worktree.path) {
        selectedWorktree = null;
        commits = [];
        selectedCommit = null;
        commitDiff = null;
        workingDiff = null;
      }

      await refreshWorktrees();
    } catch (e) {
      await message(String(e), { title: "Delete Failed", kind: "error" });
    }
  }

  async function handlePruneWorktrees() {
    const confirmed = await ask(
      "This will remove stale worktree references. Continue?",
      {
        title: "Prune Worktrees",
        kind: "info",
        okLabel: "Prune",
        cancelLabel: "Cancel",
      }
    );

    if (!confirmed) return;

    try {
      const result = await invoke<PruneResult>("prune_worktrees", {
        repoPath: repoPath,
      });

      if (result.pruned_count > 0) {
        await message(
          `Pruned ${result.pruned_count} stale worktree reference(s).\n\n${result.messages.join("\n")}`,
          { title: "Prune Complete", kind: "info" }
        );
        await refreshWorktrees();
      } else {
        await message("No stale worktree references found.", {
          title: "Prune Complete",
          kind: "info",
        });
      }
    } catch (e) {
      await message(String(e), { title: "Prune Failed", kind: "error" });
    }
  }

  function applyTheme(theme: Theme) {
    document.documentElement.setAttribute("data-theme", theme);
    setTheme(theme);
    invoke("set_theme_menu_state", { theme });
  }

  /** Load Claude status for all worktrees */
  async function loadClaudeStatuses() {
    if (worktrees.length === 0) return;

    try {
      const paths = worktrees.map((w) => w.path);
      const statuses = await invoke<Record<string, WorktreeClaudeStatus>>(
        "get_all_claude_statuses",
        { worktreePaths: paths }
      );
      claudeStatuses = new Map(Object.entries(statuses));
    } catch (e) {
      console.error("Failed to load Claude statuses:", e);
    }
  }

  /** Check and load Claude hooks configuration */
  async function checkClaudeHooks() {
    try {
      claudeHooksConfig = await invoke<ClaudeHooksConfig>("check_claude_hooks");
    } catch (e) {
      console.error("Failed to check Claude hooks:", e);
    }
  }

  /** Configure Claude hooks for Woodeye status tracking */
  async function handleConfigureHooks() {
    try {
      await invoke("configure_claude_hooks");
      await checkClaudeHooks();
      // Show success message
      await message("Claude hooks configured successfully! Status tracking is now enabled.", {
        title: "Hooks Configured",
        kind: "info",
      });
      // Start watching for Claude status changes
      invoke("start_claude_watching");
    } catch (e) {
      await message(String(e), { title: "Configuration Failed", kind: "error" });
    }
  }

  /** Remove Claude hooks from settings */
  async function handleRemoveHooks() {
    const confirmed = await ask(
      "This will remove Woodeye hooks from Claude settings. Status tracking will be disabled.",
      {
        title: "Remove Hooks",
        kind: "warning",
        okLabel: "Remove",
        cancelLabel: "Cancel",
      }
    );

    if (!confirmed) return;

    try {
      await invoke("remove_claude_hooks");
      await checkClaudeHooks();
      await message("Claude hooks removed. Status tracking is now disabled.", {
        title: "Hooks Removed",
        kind: "info",
      });
    } catch (e) {
      await message(String(e), { title: "Removal Failed", kind: "error" });
    }
  }

  onMount(() => {
    listen("worktree-changed", () => {
      // Clear the working diff cache since files have changed
      workingDiffCache = new Map();

      if (selectedWorktree) {
        // Refresh worktree status to update "Working Changes" entry visibility and file count
        loadWorktreeStatus(selectedWorktree.path);
      }

      if (workingSelected && selectedWorktree) {
        // If viewing working changes, auto-refresh with debounce
        refreshWorkingDiffDebounced();
      } else {
        // Otherwise, show indicator that external changes occurred
        hasExternalChanges = true;
      }
    }).then((fn) => {
      unlisten = fn;
    });

    listen<string>("menu-theme-changed", (event) => {
      applyTheme(event.payload as Theme);
    }).then((fn) => {
      unlistenTheme = fn;
    });

    // Listen for Claude status changes
    listen("claude-status-changed", () => {
      loadClaudeStatuses();
    }).then((fn) => {
      unlistenClaude = fn;
    });

    // Initialize theme from localStorage and sync with menu
    const savedTheme = getTheme();
    applyTheme(savedTheme);

    // Check Claude hooks configuration
    checkClaudeHooks();

    // Start watching for Claude status changes
    invoke("start_claude_watching").catch((e) => {
      console.error("Failed to start Claude watching:", e);
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
      if (unlistenTheme) {
        unlistenTheme();
      }
      if (unlistenClaude) {
        unlistenClaude();
      }
      if (refreshTimeout) {
        clearTimeout(refreshTimeout);
      }
    };
  });
</script>

<div class="app-layout">
  <ContentToolbar
    bind:repoPath
    {worktrees}
    {selectedWorktree}
    {claudeStatuses}
    {claudeHooksConfig}
    {loading}
    {refreshing}
    {hasExternalChanges}
    onLoadRepo={loadWorktrees}
    onSelectWorktree={selectWorktree}
    onCreateWorktree={openCreateDialog}
    onDeleteWorktree={handleDeleteWorktree}
    onPruneWorktrees={handlePruneWorktrees}
    onRefresh={refreshAll}
    onConfigureHooks={handleConfigureHooks}
    onRemoveHooks={handleRemoveHooks}
  />

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
      <div class="split-view">
        <section class="commits-panel">
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

{#if showCreateDialog}
  <CreateWorktreeDialog
    {branches}
    mainWorktreePath={mainWorktreePath}
    onClose={() => (showCreateDialog = false)}
    onCreate={handleCreateWorktree}
  />
{/if}

<style>
  .app-layout {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--color-bg);
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
  }

  .commits-panel {
    width: 340px;
    min-width: 280px;
    background: var(--color-bg-card);
    border-right: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .diff-panel {
    flex: 1;
    background: var(--color-bg-card);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
</style>
