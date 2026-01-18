<script lang="ts">
  import type { Worktree } from "../types";

  interface Props {
    worktrees: Worktree[];
    selectedWorktree: Worktree | null;
    onSelectWorktree: (worktree: Worktree) => void;
    onCreateWorktree: () => void;
    onDeleteWorktree: (worktree: Worktree) => void;
    onPruneWorktrees: () => void;
    loading?: boolean;
  }

  let {
    worktrees,
    selectedWorktree,
    onSelectWorktree,
    onCreateWorktree,
    onDeleteWorktree,
    onPruneWorktrees,
    loading = false,
  }: Props = $props();

  let isOpen = $state(false);
  let dropdownRef: HTMLDivElement | null = $state(null);

  let hasChanges = $derived(
    selectedWorktree?.status ? !selectedWorktree.status.is_clean : false
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

  function handleClickOutside(event: MouseEvent) {
    if (dropdownRef && !dropdownRef.contains(event.target as Node)) {
      isOpen = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && isOpen) {
      isOpen = false;
    }
  }

  function handleSelect(worktree: Worktree) {
    onSelectWorktree(worktree);
    isOpen = false;
  }

  function handleDelete(event: MouseEvent, worktree: Worktree) {
    event.stopPropagation();
    onDeleteWorktree(worktree);
  }

  function handleCreate(event: MouseEvent) {
    event.stopPropagation();
    onCreateWorktree();
    isOpen = false;
  }

  function handlePrune(event: MouseEvent) {
    event.stopPropagation();
    onPruneWorktrees();
  }
</script>

<svelte:window onclick={handleClickOutside} onkeydown={handleKeydown} />

<div class="worktree-dropdown" bind:this={dropdownRef}>
  <button
    class="dropdown-trigger"
    class:open={isOpen}
    onclick={() => (isOpen = !isOpen)}
    disabled={loading}
  >
    <div class="selected-worktree">
      {#if selectedWorktree}
        <span class="worktree-icon">
          {#if selectedWorktree.is_main}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10"/>
              <path d="M12 6v6l4 2"/>
            </svg>
          {:else}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M6 3v12"/>
              <circle cx="18" cy="6" r="3"/>
              <circle cx="6" cy="18" r="3"/>
              <path d="M18 9a9 9 0 01-9 9"/>
            </svg>
          {/if}
        </span>
        <span class="worktree-name">{selectedWorktree.name}</span>
        <span class="branch-badge">
          {selectedWorktree.head.branch ?? "detached"}
          {#if selectedWorktree.head.upstream}
            {#if selectedWorktree.head.upstream.ahead > 0 || selectedWorktree.head.upstream.behind > 0}
              <span class="sync-indicator">
                {#if selectedWorktree.head.upstream.ahead > 0}↑{selectedWorktree.head.upstream.ahead}{/if}
                {#if selectedWorktree.head.upstream.behind > 0}↓{selectedWorktree.head.upstream.behind}{/if}
              </span>
            {/if}
          {/if}
        </span>
        {#if hasChanges}
          <span class="change-indicator" title="Has uncommitted changes"></span>
        {/if}
      {:else}
        <span class="placeholder">Select worktree...</span>
      {/if}
    </div>
    <svg class="chevron" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M6 9l6 6 6-6"/>
    </svg>
  </button>

  {#if isOpen}
    <div class="dropdown-panel">
      <div class="dropdown-header">
        <span class="dropdown-title">Worktrees</span>
        <div class="dropdown-actions">
          <button class="action-btn" onclick={handleCreate} title="Create worktree">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 5v14M5 12h14"/>
            </svg>
          </button>
          <button class="action-btn" onclick={handlePrune} title="Prune stale worktrees">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6"/>
              <path d="M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
            </svg>
          </button>
        </div>
      </div>
      <div class="worktree-list">
        {#each worktrees as worktree (worktree.path)}
          {@const isSelected = selectedWorktree?.path === worktree.path}
          {@const wtHasChanges = worktree.status ? !worktree.status.is_clean : false}
          <div class="worktree-option-wrapper">
            <button
              class="worktree-option"
              class:selected={isSelected}
              onclick={() => handleSelect(worktree)}
            >
              <span class="worktree-icon">
                {#if worktree.is_main}
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="10"/>
                    <path d="M12 6v6l4 2"/>
                  </svg>
                {:else}
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M6 3v12"/>
                    <circle cx="18" cy="6" r="3"/>
                    <circle cx="6" cy="18" r="3"/>
                    <path d="M18 9a9 9 0 01-9 9"/>
                  </svg>
                {/if}
              </span>
              <div class="worktree-info">
                <span class="option-name">{worktree.name}</span>
                <div class="option-branch-line">
                  <span class="option-branch">{worktree.head.branch ?? "detached"}</span>
                  {#if worktree.head.upstream}
                    <span class="option-upstream">→ {worktree.head.upstream.remote_branch}</span>
                    {#if worktree.head.upstream.ahead > 0 || worktree.head.upstream.behind > 0}
                      <span class="option-sync">
                        {#if worktree.head.upstream.ahead > 0}↑{worktree.head.upstream.ahead}{/if}
                        {#if worktree.head.upstream.behind > 0}↓{worktree.head.upstream.behind}{/if}
                      </span>
                    {/if}
                  {/if}
                </div>
              </div>
              <div class="worktree-meta">
                {#if wtHasChanges}
                  <span class="change-indicator" title="Has uncommitted changes"></span>
                {/if}
                <span class="worktree-time">{formatRelativeTime(worktree.last_commit_timestamp)}</span>
              </div>
            </button>
            {#if !worktree.is_main}
              <button
                class="delete-btn"
                onclick={(e) => handleDelete(e, worktree)}
                title="Delete worktree"
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M18 6L6 18M6 6l12 12"/>
                </svg>
              </button>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .worktree-dropdown {
    position: relative;
    width: 100%;
  }

  .dropdown-trigger {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    width: 100%;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-text);
    cursor: pointer;
    transition: border-color 0.15s, background-color 0.15s;
  }

  .dropdown-trigger:hover:not(:disabled) {
    border-color: var(--color-primary);
  }

  .dropdown-trigger.open {
    border-color: var(--color-primary);
  }

  .dropdown-trigger:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .selected-worktree {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex: 1;
    min-width: 0;
  }

  .worktree-icon {
    flex-shrink: 0;
    color: var(--color-text-muted);
    display: flex;
    align-items: center;
  }

  .worktree-name {
    font-weight: 500;
    font-size: 0.85rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .branch-badge {
    font-size: 0.7rem;
    font-weight: 500;
    padding: 2px var(--space-xs);
    background: var(--color-primary-light);
    color: var(--color-primary);
    border-radius: 3px;
    flex-shrink: 0;
  }

  .sync-indicator {
    margin-left: 4px;
    font-size: 0.6rem;
    font-weight: 600;
    color: var(--color-info);
  }

  .change-indicator {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-warning);
    flex-shrink: 0;
  }

  .placeholder {
    color: var(--color-text-muted);
    font-size: 0.85rem;
  }

  .chevron {
    flex-shrink: 0;
    color: var(--color-text-muted);
    transition: transform 0.15s;
  }

  .dropdown-trigger.open .chevron {
    transform: rotate(180deg);
  }

  .dropdown-panel {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    max-height: 400px;
    background: var(--color-bg-card);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
    z-index: 100;
    display: flex;
    flex-direction: column;
  }

  .dropdown-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-sm) var(--space-md);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg);
  }

  .dropdown-title {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-muted);
  }

  .dropdown-actions {
    display: flex;
    gap: var(--space-xs);
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }

  .action-btn:hover {
    background: var(--color-bg-card);
    color: var(--color-primary);
  }

  .worktree-list {
    overflow-y: auto;
    flex: 1;
  }

  .worktree-option-wrapper {
    position: relative;
  }

  .worktree-option {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border: none;
    background: transparent;
    color: var(--color-text);
    cursor: pointer;
    text-align: left;
    transition: background-color 0.15s;
  }

  .worktree-option:hover {
    background: var(--color-bg);
  }

  .worktree-option.selected {
    background: var(--color-primary-light);
  }

  .worktree-option.selected .worktree-icon {
    color: var(--color-primary);
  }

  .worktree-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .option-name {
    font-size: 0.85rem;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .option-branch {
    font-size: 0.7rem;
    color: var(--color-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .option-branch-line {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
  }

  .option-upstream {
    font-size: 0.65rem;
    color: var(--color-text-muted);
    opacity: 0.8;
  }

  .option-sync {
    font-size: 0.65rem;
    font-weight: 600;
    color: var(--color-info);
  }

  .worktree-meta {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-shrink: 0;
  }

  .worktree-time {
    font-size: 0.7rem;
    color: var(--color-text-muted);
  }

  .delete-btn {
    position: absolute;
    right: var(--space-sm);
    top: 50%;
    transform: translateY(-50%);
    display: none;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }

  .worktree-option-wrapper:hover .delete-btn {
    display: flex;
  }

  .delete-btn:hover {
    background: rgba(248, 113, 113, 0.15);
    color: var(--color-error);
  }
</style>
