<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import WorktreeDropdown from "./WorktreeDropdown.svelte";
  import { getTheme, setTheme, type Theme } from "../store";
  import type { Worktree } from "../types";

  let theme: Theme = $state("system");
  let terminalMenuOpen = $state(false);

  function getFolderName(path: string): string {
    if (!path) return "";
    const segments = path.replace(/\/$/, "").split("/");
    return segments[segments.length - 1] || path;
  }

  function applyTheme(t: Theme) {
    document.documentElement.setAttribute("data-theme", t);
  }

  function cycleTheme() {
    const next: Theme = theme === "system" ? "light" : theme === "light" ? "dark" : "system";
    theme = next;
    setTheme(next);
    applyTheme(next);
  }

  function toggleTerminalMenu() {
    terminalMenuOpen = !terminalMenuOpen;
  }

  async function handleOpenTerminal(terminal: string) {
    terminalMenuOpen = false;
    if (!selectedWorktree) return;
    try {
      await invoke("open_in_terminal", { path: selectedWorktree.path, terminal });
    } catch (e) {
      console.error("Failed to open terminal:", e);
    }
  }

  onMount(() => {
    theme = getTheme();
    applyTheme(theme);
  });

  interface Props {
    repoPath: string;
    worktrees: Worktree[];
    selectedWorktree: Worktree | null;
    loading: boolean;
    refreshing: boolean;
    hasExternalChanges: boolean;
    onLoadRepo: (path: string) => void;
    onSelectWorktree: (worktree: Worktree) => void;
    onCreateWorktree: () => void;
    onDeleteWorktree: (worktree: Worktree) => void;
    onPruneWorktrees: () => void;
    onRefresh: () => void;
  }

  let {
    repoPath = $bindable(),
    worktrees,
    selectedWorktree,
    loading,
    refreshing,
    hasExternalChanges,
    onLoadRepo,
    onSelectWorktree,
    onCreateWorktree,
    onDeleteWorktree,
    onPruneWorktrees,
    onRefresh,
  }: Props = $props();

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

<header class="content-toolbar">
  <div class="toolbar-logo">
    <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="12" cy="12" r="3"/>
      <path d="M12 2v4m0 12v4M2 12h4m12 0h4"/>
      <path d="M4.93 4.93l2.83 2.83m8.48 8.48l2.83 2.83M4.93 19.07l2.83-2.83m8.48-8.48l2.83-2.83"/>
    </svg>
    <span>Woodeye</span>
  </div>

  <div class="context-card repo-context" title={repoPath}>
      <svg class="context-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
      </svg>
      {#if repoPath}
        <span class="context-value">{getFolderName(repoPath)}</span>
      {:else}
        <span class="context-placeholder">No repository</span>
      {/if}
      <button class="context-action" onclick={handleBrowse} disabled={loading} title="Browse for repository">
        {#if loading}
          <span class="btn-spinner"></span>
        {:else}
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M5 12h14M12 5l7 7-7 7"/>
          </svg>
        {/if}
      </button>
  </div>

  {#if worktrees.length > 0}
    <WorktreeDropdown
      {worktrees}
      {selectedWorktree}
      {onSelectWorktree}
      {onCreateWorktree}
      {onDeleteWorktree}
      {onPruneWorktrees}
      {loading}
    />
  {/if}

  <div class="toolbar-actions">
    <div class="terminal-wrapper">
      <button
        class="terminal-btn"
        onclick={toggleTerminalMenu}
        disabled={!selectedWorktree}
        title="Open in terminal"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="4 17 10 11 4 5"/>
          <line x1="12" y1="19" x2="20" y2="19"/>
        </svg>
      </button>
      {#if terminalMenuOpen}
        <div class="terminal-menu">
          <button class="terminal-option" onclick={() => handleOpenTerminal("terminal")}>
            Terminal
          </button>
          <button class="terminal-option" onclick={() => handleOpenTerminal("warp")}>
            Warp
          </button>
          <button class="terminal-option" onclick={() => handleOpenTerminal("iterm")}>
            iTerm
          </button>
        </div>
      {/if}
    </div>
    <button
      class="theme-btn"
      onclick={cycleTheme}
      title={theme === "system" ? "Theme: System" : theme === "light" ? "Theme: Light" : "Theme: Dark"}
    >
      {#if theme === "system"}
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="4"/>
          <path d="M12 2v2m0 16v2M4.93 4.93l1.41 1.41m11.32 11.32l1.41 1.41M2 12h2m16 0h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/>
        </svg>
      {:else if theme === "light"}
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="5"/>
          <path d="M12 1v2m0 18v2M4.22 4.22l1.42 1.42m12.72 12.72l1.42 1.42M1 12h2m18 0h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
        </svg>
      {:else}
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
        </svg>
      {/if}
    </button>
    <button
      class="refresh-btn"
      class:has-changes={hasExternalChanges}
      onclick={onRefresh}
      disabled={refreshing || loading || worktrees.length === 0}
      title={hasExternalChanges ? "Changes detected - click to refresh" : "Refresh"}
    >
      <svg class:spinning={refreshing} width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M21 12a9 9 0 1 1-9-9"/>
        <path d="M21 3v9h-9"/>
      </svg>
    </button>
  </div>
</header>

<style>
  .content-toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-lg);
    padding: var(--space-md) var(--space-xl);
    background: var(--color-bg-card);
    border-bottom: 1px solid var(--color-border);
    min-height: 60px;
  }

  .toolbar-logo {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    color: var(--color-text);
    font-weight: 600;
    font-size: 1rem;
    flex-shrink: 0;
  }

  .toolbar-logo svg {
    color: var(--color-primary);
  }

  .context-card {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-md);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    min-width: 0;
  }

  .context-icon {
    color: var(--color-text-muted);
    flex-shrink: 0;
  }

  .context-value {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--color-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .context-placeholder {
    font-size: 0.85rem;
    color: var(--color-text-muted);
    font-style: italic;
  }

  .context-action {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    margin-left: auto;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
    flex-shrink: 0;
  }

  .context-action:hover:not(:disabled) {
    background: var(--color-bg-card);
    color: var(--color-primary);
  }

  .context-action:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .repo-context {
    flex-shrink: 0;
    max-width: 200px;
  }

  .btn-spinner {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(100, 100, 100, 0.3);
    border-top-color: var(--color-text-muted);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin-left: auto;
  }

  .theme-btn,
  .refresh-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s, border-color 0.15s;
  }

  .theme-btn:hover {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .refresh-btn:hover:not(:disabled) {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .refresh-btn.has-changes {
    color: var(--color-warning);
    border-color: var(--color-warning);
    position: relative;
  }

  .refresh-btn.has-changes::after {
    content: "";
    position: absolute;
    top: 4px;
    right: 4px;
    width: 8px;
    height: 8px;
    background: var(--color-warning);
    border-radius: 50%;
  }

  .refresh-btn .spinning {
    animation: spin 0.8s linear infinite;
  }

  .terminal-wrapper {
    position: relative;
  }

  .terminal-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s, border-color 0.15s;
  }

  .terminal-btn:hover:not(:disabled) {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .terminal-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .terminal-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    display: flex;
    flex-direction: column;
    background: var(--color-bg-card);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-md);
    overflow: hidden;
    z-index: 100;
  }

  .terminal-option {
    padding: var(--space-sm) var(--space-md);
    border: none;
    background: transparent;
    color: var(--color-text);
    font-size: 0.85rem;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    transition: background-color 0.15s;
  }

  .terminal-option:hover {
    background: var(--color-bg);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
