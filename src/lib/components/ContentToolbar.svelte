<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import WorktreeDropdown from "./WorktreeDropdown.svelte";
  import type { Worktree, WorktreeClaudeStatus, ClaudeHooksConfig } from "../types";

  let terminalMenuOpen = $state(false);
  let claudeMenuOpen = $state(false);

  function getFolderName(path: string): string {
    if (!path) return "";
    const segments = path.replace(/\/$/, "").split("/");
    return segments[segments.length - 1] || path;
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

  async function handleOpenAgent() {
    if (!selectedWorktree) return;
    try {
      await invoke("open_claude_in_terminal", { path: selectedWorktree.path });
    } catch (e) {
      console.error("Failed to open agent:", e);
    }
  }

  interface Props {
    repoPath: string;
    worktrees: Worktree[];
    selectedWorktree: Worktree | null;
    claudeStatuses: Map<string, WorktreeClaudeStatus>;
    claudeHooksConfig: ClaudeHooksConfig | null;
    loading: boolean;
    refreshing: boolean;
    hasExternalChanges: boolean;
    onLoadRepo: (path: string) => void;
    onSelectWorktree: (worktree: Worktree) => void;
    onCreateWorktree: () => void;
    onDeleteWorktree: (worktree: Worktree) => void;
    onPruneWorktrees: () => void;
    onRefresh: () => void;
    onConfigureHooks: () => void;
    onRemoveHooks: () => void;
  }

  let {
    repoPath = $bindable(),
    worktrees,
    selectedWorktree,
    claudeStatuses,
    claudeHooksConfig,
    loading,
    refreshing,
    hasExternalChanges,
    onLoadRepo,
    onSelectWorktree,
    onCreateWorktree,
    onDeleteWorktree,
    onPruneWorktrees,
    onRefresh,
    onConfigureHooks,
    onRemoveHooks,
  }: Props = $props();

  function toggleClaudeMenu() {
    claudeMenuOpen = !claudeMenuOpen;
    if (claudeMenuOpen) {
      terminalMenuOpen = false;
    }
  }

  function handleConfigureHooks() {
    claudeMenuOpen = false;
    onConfigureHooks();
  }

  function handleRemoveHooks() {
    claudeMenuOpen = false;
    onRemoveHooks();
  }

  // Get Claude status for the selected worktree
  let selectedClaudeStatus = $derived(
    selectedWorktree ? claudeStatuses.get(selectedWorktree.path) : undefined
  );

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
      {claudeStatuses}
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
      class="agent-btn"
      class:has-session={selectedClaudeStatus?.active_sessions.length}
      class:needs-input={selectedClaudeStatus?.has_pending_input}
      onclick={handleOpenAgent}
      disabled={!selectedWorktree}
      title={selectedClaudeStatus?.has_pending_input
        ? "Claude needs input - click to open"
        : selectedClaudeStatus?.active_sessions.length
          ? "Claude session active - click to open"
          : "Open Claude agent"}
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="8" r="4"/>
        <path d="M6 20c0-3.3 2.7-6 6-6s6 2.7 6 6"/>
        <path d="M12 4V2m-4 3L7 3m10 2l1-2"/>
      </svg>
      {#if selectedClaudeStatus?.has_pending_input}
        <span class="agent-badge waiting"></span>
      {:else if selectedClaudeStatus?.active_sessions.length}
        <span class="agent-badge active"></span>
      {/if}
    </button>
    <div class="claude-menu-wrapper">
      <button
        class="hooks-btn"
        class:configured={claudeHooksConfig?.configured}
        onclick={toggleClaudeMenu}
        title={claudeHooksConfig?.configured ? "Claude hooks settings" : "Enable Claude status tracking"}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="3"/>
          <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-2 2 2 2 0 01-2-2v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 01-2-2 2 2 0 012-2h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 010-2.83 2 2 0 012.83 0l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 012-2 2 2 0 012 2v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 0 2 2 0 010 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 012 2 2 2 0 01-2 2h-.09a1.65 1.65 0 00-1.51 1z"/>
        </svg>
      </button>
      {#if claudeMenuOpen}
        <div class="claude-menu">
          {#if claudeHooksConfig?.configured}
            <button class="claude-menu-option" onclick={handleConfigureHooks}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.66 0 3-4.03 3-9s-1.34-9-3-9m0 18c-1.66 0-3-4.03-3-9s1.34-9 3-9"/>
              </svg>
              Reconfigure Hooks
            </button>
            <button class="claude-menu-option danger" onclick={handleRemoveHooks}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6"/>
                <path d="M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
              </svg>
              Remove Hooks
            </button>
          {:else}
            <button class="claude-menu-option" onclick={handleConfigureHooks}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 5v14M5 12h14"/>
              </svg>
              Enable Status Tracking
            </button>
          {/if}
        </div>
      {/if}
    </div>
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

  .refresh-btn,
  .agent-btn {
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

  .agent-btn {
    position: relative;
  }

  .agent-btn:hover:not(:disabled) {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .agent-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .agent-btn.has-session {
    border-color: var(--color-success);
    color: var(--color-success);
  }

  .agent-btn.needs-input {
    border-color: var(--color-warning);
    color: var(--color-warning);
  }

  .agent-badge {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .agent-badge.active {
    background: var(--color-success);
  }

  .agent-badge.waiting {
    background: var(--color-warning);
    animation: pulse 1.5s ease-in-out infinite;
  }

  .claude-menu-wrapper {
    position: relative;
  }

  .hooks-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: 1px dashed var(--color-warning);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-warning);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s, border-color 0.15s;
  }

  .hooks-btn.configured {
    border: 1px solid var(--color-border);
    color: var(--color-text-muted);
  }

  .hooks-btn:hover {
    background: rgba(245, 158, 11, 0.1);
    border-style: solid;
    border-color: var(--color-warning);
    color: var(--color-warning);
  }

  .claude-menu {
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
    min-width: 180px;
  }

  .claude-menu-option {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
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

  .claude-menu-option:hover {
    background: var(--color-bg);
  }

  .claude-menu-option.danger {
    color: var(--color-error);
  }

  .claude-menu-option.danger:hover {
    background: rgba(248, 113, 113, 0.1);
  }

  @keyframes pulse {
    0%, 100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.6;
      transform: scale(1.2);
    }
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
