<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import type { Worktree } from "../types";

  interface Props {
    worktrees: Worktree[];
    selectedWorktree: Worktree | null;
    repoPath: string;
    onSelectWorktree: (worktree: Worktree) => void;
    onLoadRepo: (path: string) => void;
    loading?: boolean;
  }

  let {
    worktrees,
    selectedWorktree,
    repoPath = $bindable(),
    onSelectWorktree,
    onLoadRepo,
    loading = false,
  }: Props = $props();

  function formatRelativeTime(timestamp: number): string {
    const now = Math.floor(Date.now() / 1000);
    const diff = now - timestamp;

    if (diff < 60) return "just now";
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    if (diff < 604800) return `${Math.floor(diff / 86400)}d ago`;
    return `${Math.floor(diff / 604800)}w ago`;
  }

  async function handleBrowse() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Git Repository",
    });

    if (selected && typeof selected === "string") {
      repoPath = selected;
      onLoadRepo(selected);
    }
  }
</script>

<div class="worktree-selector">
  <div class="repo-input">
    <div class="input-wrapper">
      <svg class="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8"/>
        <path d="m21 21-4.3-4.3"/>
      </svg>
      <input
        type="text"
        bind:value={repoPath}
        placeholder="Repository path..."
        disabled={loading}
        onkeydown={(e) => e.key === "Enter" && onLoadRepo(repoPath)}
      />
    </div>
    <button onclick={handleBrowse} disabled={loading} class="icon-btn" title="Browse">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
      </svg>
    </button>
    <button
      onclick={() => onLoadRepo(repoPath)}
      disabled={loading || !repoPath.trim()}
      class="load-btn"
    >
      {#if loading}
        <span class="btn-spinner"></span>
      {:else}
        Load
      {/if}
    </button>
  </div>

  {#if worktrees.length > 0}
    <div class="section-label">Worktrees</div>
    <nav class="worktree-list">
      {#each worktrees as worktree (worktree.path)}
        {@const isSelected = selectedWorktree?.path === worktree.path}
        {@const hasChanges = worktree.status ? !worktree.status.is_clean : false}
        <button
          class="worktree-item"
          class:selected={isSelected}
          class:has-changes={hasChanges}
          onclick={() => onSelectWorktree(worktree)}
          disabled={loading}
        >
          <div class="worktree-icon">
            {#if worktree.is_main}
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <path d="M12 6v6l4 2"/>
              </svg>
            {:else}
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M6 3v12"/>
                <circle cx="18" cy="6" r="3"/>
                <circle cx="6" cy="18" r="3"/>
                <path d="M18 9a9 9 0 01-9 9"/>
              </svg>
            {/if}
          </div>
          <div class="worktree-info">
            <span class="worktree-name">{worktree.name}</span>
            <span class="worktree-branch">
              {worktree.head.branch ?? "detached"}
            </span>
          </div>
          <div class="worktree-meta">
            {#if hasChanges}
              <span class="change-indicator" title="Has uncommitted changes"></span>
            {/if}
            <span class="worktree-time">{formatRelativeTime(worktree.last_commit_timestamp)}</span>
          </div>
        </button>
      {/each}
    </nav>
  {:else if !loading && repoPath}
    <div class="empty-hint">
      <p>Press Enter or click Load to open the repository</p>
    </div>
  {/if}
</div>

<style>
  .worktree-selector {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .repo-input {
    display: flex;
    gap: var(--space-sm);
    padding: var(--space-md);
  }

  .input-wrapper {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: var(--space-sm);
    color: var(--color-text-sidebar);
    pointer-events: none;
  }

  input {
    width: 100%;
    padding: var(--space-sm) var(--space-sm) var(--space-sm) 32px;
    font-size: 0.85rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.05);
    color: var(--color-text-sidebar-active);
    font-family: ui-monospace, monospace;
    transition: border-color 0.15s, background-color 0.15s;
  }

  input:focus {
    outline: none;
    border-color: var(--color-primary);
    background: rgba(255, 255, 255, 0.08);
  }

  input::placeholder {
    color: var(--color-text-sidebar);
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: none;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.05);
    color: var(--color-text-sidebar);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }

  .icon-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: var(--color-text-sidebar-active);
  }

  .icon-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .load-btn {
    padding: var(--space-sm) var(--space-md);
    font-size: 0.85rem;
    font-weight: 500;
    border: none;
    border-radius: var(--radius-sm);
    background: var(--color-primary);
    color: white;
    cursor: pointer;
    transition: opacity 0.15s;
    min-width: 60px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .load-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .load-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .section-label {
    padding: var(--space-sm) var(--space-md);
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--color-text-sidebar);
  }

  .worktree-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 var(--space-sm);
  }

  .worktree-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    margin-bottom: 2px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-sidebar);
    cursor: pointer;
    text-align: left;
    transition: background-color 0.15s, color 0.15s;
  }

  .worktree-item:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.05);
    color: var(--color-text-sidebar-active);
  }

  .worktree-item.selected {
    background: rgba(124, 92, 252, 0.2);
    color: var(--color-text-sidebar-active);
  }

  .worktree-item.selected .worktree-icon {
    color: var(--color-primary);
  }

  .worktree-item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .worktree-icon {
    flex-shrink: 0;
    opacity: 0.7;
  }

  .worktree-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .worktree-name {
    font-size: 0.85rem;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .worktree-branch {
    font-size: 0.75rem;
    opacity: 0.7;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .worktree-meta {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-shrink: 0;
  }

  .change-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-warning);
  }

  .worktree-time {
    font-size: 0.7rem;
    opacity: 0.6;
  }

  .empty-hint {
    padding: var(--space-lg);
    text-align: center;
  }

  .empty-hint p {
    font-size: 0.8rem;
    color: var(--color-text-sidebar);
    opacity: 0.7;
  }
</style>
