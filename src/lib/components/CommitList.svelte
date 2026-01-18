<script lang="ts">
  import type { CommitInfo, WorktreeStatus } from "../types";

  interface Props {
    commits: CommitInfo[];
    selectedCommit: CommitInfo | null;
    workingSelected: boolean;
    worktreeStatus: WorktreeStatus | null;
    onSelectCommit: (commit: CommitInfo) => void;
    onSelectWorking: () => void;
    onLoadMore: () => void;
    hasMore?: boolean;
    loading?: boolean;
  }

  let {
    commits,
    selectedCommit,
    workingSelected,
    worktreeStatus,
    onSelectCommit,
    onSelectWorking,
    onLoadMore,
    hasMore = true,
    loading = false,
  }: Props = $props();

  let hasWorkingChanges = $derived(
    worktreeStatus && !worktreeStatus.is_clean
  );

  let workingChangeCount = $derived(
    worktreeStatus
      ? worktreeStatus.modified + worktreeStatus.staged + worktreeStatus.untracked
      : 0
  );

  function formatRelativeTime(timestamp: number): string {
    const now = Math.floor(Date.now() / 1000);
    const diff = now - timestamp;

    if (diff < 60) return "just now";
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    if (diff < 604800) return `${Math.floor(diff / 86400)}d ago`;
    if (diff < 2592000) return `${Math.floor(diff / 604800)}w ago`;
    return `${Math.floor(diff / 2592000)}mo ago`;
  }
</script>

<div class="commit-list">
  {#if commits.length === 0 && !hasWorkingChanges && !loading}
    <div class="empty">
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <path d="M12 6v6l4 2"/>
      </svg>
      <span>No commits</span>
    </div>
  {:else}
    <div class="commits">
      {#if hasWorkingChanges}
        <button
          class="commit-item working-changes"
          class:selected={workingSelected}
          onclick={onSelectWorking}
        >
          <div class="commit-indicator working"></div>
          <div class="commit-content">
            <div class="commit-header">
              <span class="commit-summary">Working Changes</span>
              <span class="commit-badge warning">
                {workingChangeCount} file{workingChangeCount !== 1 ? 's' : ''}
              </span>
            </div>
            <div class="commit-meta">
              <span class="commit-author">Uncommitted</span>
            </div>
          </div>
        </button>
      {/if}

      {#each commits as commit (commit.hash)}
        <button
          class="commit-item"
          class:selected={!workingSelected && selectedCommit?.hash === commit.hash}
          onclick={() => onSelectCommit(commit)}
        >
          <div class="commit-indicator"></div>
          <div class="commit-content">
            <div class="commit-header">
              <span class="commit-summary">{commit.summary}</span>
            </div>
            <div class="commit-meta">
              <code class="sha">{commit.short_hash}</code>
              <span class="separator">·</span>
              <span class="commit-author">{commit.author_name}</span>
              <span class="separator">·</span>
              <span class="time">{formatRelativeTime(commit.timestamp)}</span>
            </div>
          </div>
        </button>
      {/each}
    </div>

    {#if hasMore}
      <button class="load-more" onclick={onLoadMore} disabled={loading}>
        {#if loading}
          <span class="btn-spinner"></span>
          <span>Loading...</span>
        {:else}
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 5v14M5 12l7 7 7-7"/>
          </svg>
          <span>Load More</span>
        {/if}
      </button>
    {/if}
  {/if}
</div>

<style>
  .commit-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .commits {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-sm);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-sm);
    padding: var(--space-2xl);
    color: var(--color-text-muted);
  }

  .commit-item {
    width: 100%;
    display: flex;
    align-items: flex-start;
    gap: var(--space-md);
    padding: var(--space-md);
    margin-bottom: var(--space-sm);
    border: none;
    border-radius: var(--radius-md);
    background: var(--color-bg);
    text-align: left;
    cursor: pointer;
    color: var(--color-text);
    transition: background-color 0.15s, box-shadow 0.15s;
  }

  .commit-item:hover {
    background: var(--color-border);
  }

  .commit-item.selected {
    background: var(--color-primary-light);
    box-shadow: inset 0 0 0 1px var(--color-primary);
  }

  .commit-item.selected .commit-indicator {
    background: var(--color-primary);
  }

  .commit-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-border);
    margin-top: 6px;
    flex-shrink: 0;
  }

  .commit-indicator.working {
    background: var(--color-warning);
  }

  .commit-content {
    flex: 1;
    min-width: 0;
  }

  .commit-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-sm);
    margin-bottom: var(--space-xs);
  }

  .commit-summary {
    font-size: 0.875rem;
    font-weight: 500;
    line-height: 1.4;
    color: var(--color-text);
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .commit-item.selected .commit-summary {
    color: var(--color-primary);
  }

  .commit-badge {
    flex-shrink: 0;
    font-size: 0.7rem;
    font-weight: 500;
    padding: 2px var(--space-sm);
    border-radius: var(--radius-sm);
    text-transform: uppercase;
  }

  .commit-badge.warning {
    background: rgba(245, 158, 11, 0.15);
    color: var(--color-warning);
  }

  .commit-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-xs);
    font-size: 0.75rem;
    color: var(--color-text-muted);
  }

  .sha {
    font-family: ui-monospace, monospace;
    font-size: 0.7rem;
    padding: 1px 4px;
    background: var(--color-bg-card);
    border-radius: 3px;
    color: var(--color-primary);
  }

  .commit-item.selected .sha {
    background: rgba(124, 92, 252, 0.15);
  }

  .separator {
    color: var(--color-border);
  }

  .commit-author {
    font-weight: 500;
  }

  .time {
    opacity: 0.8;
  }

  .load-more {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-sm);
    padding: var(--space-md);
    border: none;
    border-top: 1px solid var(--color-border);
    background: transparent;
    color: var(--color-primary);
    cursor: pointer;
    font-weight: 500;
    font-size: 0.85rem;
    transition: background-color 0.15s;
  }

  .load-more:hover:not(:disabled) {
    background: var(--color-bg);
  }

  .load-more:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--color-border);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .working-changes {
    border-left: 3px solid var(--color-warning);
  }

  .working-changes.selected {
    border-left-color: var(--color-warning);
  }
</style>
