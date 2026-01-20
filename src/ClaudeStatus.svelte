<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import type { ClaudeSession, HooksState } from "./lib/types";

  let sessions = $state<ClaudeSession[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let expandedSessions = $state<Set<string>>(new Set());
  let hooksState = $state<HooksState | null>(null);
  let hooksLoading = $state(false);
  let alwaysOnTop = $state(localStorage.getItem("claude-status-always-on-top") === "true");

  function getProjectName(path: string): string {
    if (!path) return "Unknown";
    const segments = path.replace(/\/$/, "").split("/");
    return segments[segments.length - 1] || path;
  }

  function getRelativeTime(timestamp: number): string {
    const now = Date.now();
    const diff = now - timestamp * 1000;

    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);

    if (days > 0) return `${days}d ago`;
    if (hours > 0) return `${hours}h ago`;
    if (minutes > 0) return `${minutes}m ago`;
    return "just now";
  }

  function getStateColor(state: string): string {
    switch (state) {
      case "working":
        return "var(--color-success)";
      case "waiting_for_approval":
        return "var(--color-warning)";
      case "idle":
      default:
        return "var(--color-text-muted)";
    }
  }

  function getStateLabel(state: string): string {
    switch (state) {
      case "working":
        return "Working";
      case "waiting_for_approval":
        return "Waiting";
      case "idle":
        return "Idle";
      default:
        return state;
    }
  }

  function truncateSessionId(id: string): string {
    if (id.length <= 12) return id;
    return `${id.slice(0, 8)}...`;
  }

  function formatJson(json: string): string {
    try {
      return JSON.stringify(JSON.parse(json), null, 2);
    } catch {
      return json;
    }
  }

  function toggleExpanded(sessionId: string) {
    const newSet = new Set(expandedSessions);
    if (newSet.has(sessionId)) {
      newSet.delete(sessionId);
    } else {
      newSet.add(sessionId);
    }
    expandedSessions = newSet;
  }

  function isExpanded(sessionId: string): boolean {
    return expandedSessions.has(sessionId);
  }

  async function deleteSession(sessionId: string) {
    try {
      await invoke("delete_claude_session", { sessionId });
      await loadSessions();
    } catch (e) {
      console.error("Failed to delete session:", e);
    }
  }

  let focusNotFound = $state<string | null>(null);

  async function focusTerminal(session: ClaudeSession) {
    try {
      const found = await invoke<boolean>("focus_terminal_for_path", { path: session.project_path });
      if (!found) {
        focusNotFound = session.session_id;
        setTimeout(() => {
          if (focusNotFound === session.session_id) {
            focusNotFound = null;
          }
        }, 2000);
      }
    } catch (e) {
      console.error("Failed to focus terminal:", e);
    }
  }

  async function loadSessions() {
    try {
      error = null;
      sessions = await invoke<ClaudeSession[]>("list_claude_sessions");
    } catch (e) {
      error = String(e);
      console.error("Failed to load sessions:", e);
    } finally {
      loading = false;
    }
  }

  async function loadHooksState() {
    try {
      hooksState = await invoke<HooksState>("get_claude_hooks_state");
    } catch (e) {
      console.error("Failed to load hooks state:", e);
    }
  }

  async function toggleHooks() {
    if (!hooksState) return;
    hooksLoading = true;
    try {
      if (hooksState.hooks_enabled) {
        await invoke("remove_claude_hooks");
      } else {
        await invoke("apply_claude_hooks");
      }
      await loadHooksState();
    } catch (e) {
      console.error("Failed to toggle hooks:", e);
    } finally {
      hooksLoading = false;
    }
  }

  async function toggleAlwaysOnTop() {
    const newValue = !alwaysOnTop;
    try {
      await invoke("set_claude_status_always_on_top", { alwaysOnTop: newValue });
      alwaysOnTop = newValue;
      localStorage.setItem("claude-status-always-on-top", String(newValue));
    } catch (e) {
      console.error("Failed to toggle always on top:", e);
    }
  }

  async function applyAlwaysOnTop() {
    if (alwaysOnTop) {
      try {
        await invoke("set_claude_status_always_on_top", { alwaysOnTop: true });
      } catch (e) {
        console.error("Failed to apply always on top:", e);
      }
    }
  }

  let unlisten: (() => void) | null = null;
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    // Start watching the status directory
    invoke("start_watching_claude_status").catch((e) => {
      console.error("Failed to start watching:", e);
    });

    // Load initial sessions and hooks state
    loadSessions();
    loadHooksState();
    applyAlwaysOnTop();

    // Poll every second for updates
    pollInterval = setInterval(() => {
      loadSessions();
    }, 1000);

    // Also listen for file system changes for immediate updates
    listen("claude-status-changed", () => {
      loadSessions();
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
      if (pollInterval) clearInterval(pollInterval);
    };
  });
</script>

<div class="status-window">
  <header class="status-header">
    <h1>Claude Sessions</h1>
    <button class="refresh-btn" onclick={loadSessions} disabled={loading} title="Refresh sessions">
      <svg class:spinning={loading} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M21 12a9 9 0 1 1-9-9"/>
        <path d="M21 3v9h-9"/>
      </svg>
    </button>
  </header>

  <div class="hooks-section">
    <div class="hooks-row">
      <div class="hooks-info">
        <span class="hooks-label">Status Hooks</span>
        <span class="hooks-status" class:enabled={hooksState?.hooks_enabled}>
          {hooksState?.hooks_enabled ? "Enabled" : "Disabled"}
        </span>
      </div>
      <button
        class="hooks-toggle"
        class:enabled={hooksState?.hooks_enabled}
        onclick={toggleHooks}
        disabled={hooksLoading || !hooksState}
        title={hooksState?.hooks_enabled ? "Remove hooks from Claude settings" : "Add hooks to Claude settings"}
      >
        {#if hooksLoading}
          <span class="spinner small"></span>
        {:else}
          {hooksState?.hooks_enabled ? "Remove" : "Apply"}
        {/if}
      </button>
    </div>
    <div class="hooks-row">
      <div class="hooks-info">
        <span class="hooks-label">Always on Top</span>
      </div>
      <button
        class="always-on-top-toggle"
        class:enabled={alwaysOnTop}
        onclick={toggleAlwaysOnTop}
        title={alwaysOnTop ? "Disable always on top" : "Enable always on top"}
      >
        {alwaysOnTop ? "On" : "Off"}
      </button>
    </div>
  </div>

  <div class="session-list">
    {#if loading}
      <div class="loading-state">
        <span class="spinner"></span>
        <span>Loading sessions...</span>
      </div>
    {:else if error}
      <div class="error-state">
        <span>{error}</span>
      </div>
    {:else if sessions.length === 0}
      <div class="empty-state">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="12" cy="8" r="4"/>
          <path d="M6 20c0-3.3 2.7-6 6-6s6 2.7 6 6"/>
        </svg>
        <span>No active Claude sessions</span>
        <span class="empty-hint">Sessions will appear here when Claude Code is running</span>
      </div>
    {:else}
      {#each sessions as session}
        <div class="session-card" class:expanded={isExpanded(session.session_id)}>
          <div class="session-indicator" style="background: {getStateColor(session.state)}"></div>
          <div class="session-content">
            <div class="session-header">
              <span class="project-name" title={session.name || getProjectName(session.project_path)}>
                {session.name || getProjectName(session.project_path)}
              </span>
              <div class="session-actions">
                <span class="session-time">{getRelativeTime(session.timestamp)}</span>
                <button
                  class="action-btn focus-btn"
                  class:not-found={focusNotFound === session.session_id}
                  onclick={() => focusTerminal(session)}
                  title={focusNotFound === session.session_id ? "Terminal not found" : "Focus terminal"}
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <rect x="2" y="4" width="20" height="16" rx="2"/>
                    <path d="M6 9l3 3-3 3"/>
                    <path d="M12 15h6"/>
                  </svg>
                </button>
                <button
                  class="action-btn expand-btn"
                  onclick={() => toggleExpanded(session.session_id)}
                  title={isExpanded(session.session_id) ? "Collapse" : "Expand"}
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    {#if isExpanded(session.session_id)}
                      <path d="M18 15l-6-6-6 6"/>
                    {:else}
                      <path d="M6 9l6 6 6-6"/>
                    {/if}
                  </svg>
                </button>
                <button
                  class="action-btn delete-btn"
                  onclick={() => deleteSession(session.session_id)}
                  title="Delete session"
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
                  </svg>
                </button>
              </div>
            </div>
            <div class="session-details">
              {#if session.name}
                <span class="session-project">{getProjectName(session.project_path)}</span>
              {/if}
              <span class="session-id" title={session.session_id}>{truncateSessionId(session.session_id)}</span>
              <span class="session-state" style="color: {getStateColor(session.state)}">{getStateLabel(session.state)}</span>
            </div>
            <div class="session-path" title={session.project_path}>{session.project_path}</div>
            {#if isExpanded(session.session_id)}
              <div class="session-json">
                <pre>{formatJson(session.raw_json)}</pre>
              </div>
            {/if}
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .status-window {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--color-bg);
  }

  .status-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-md) var(--space-lg);
    background: var(--color-bg-card);
    border-bottom: 1px solid var(--color-border);
  }

  .status-header h1 {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-text);
    margin: 0;
  }

  .refresh-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s, border-color 0.15s;
  }

  .refresh-btn:hover:not(:disabled) {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .refresh-btn .spinning {
    animation: spin 0.8s linear infinite;
  }

  .hooks-section {
    padding: var(--space-sm) var(--space-md);
    background: var(--color-bg-card);
    border-bottom: 1px solid var(--color-border);
  }

  .hooks-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
  }

  .hooks-info {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .hooks-label {
    font-size: 0.85rem;
    color: var(--color-text-muted);
  }

  .hooks-status {
    font-size: 0.75rem;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: 10px;
    background: var(--color-bg);
    color: var(--color-text-muted);
  }

  .hooks-status.enabled {
    background: rgba(34, 197, 94, 0.15);
    color: var(--color-success);
  }

  .hooks-toggle {
    font-size: 0.75rem;
    font-weight: 500;
    padding: 4px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-text);
    cursor: pointer;
    transition: background-color 0.15s, border-color 0.15s, color 0.15s;
    min-width: 60px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .hooks-toggle:hover:not(:disabled) {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .hooks-toggle.enabled:hover:not(:disabled) {
    border-color: var(--color-error);
    color: var(--color-error);
  }

  .hooks-toggle:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .always-on-top-toggle {
    font-size: 0.75rem;
    font-weight: 500;
    padding: 4px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-text);
    cursor: pointer;
    transition: background-color 0.15s, border-color 0.15s, color 0.15s;
    min-width: 50px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .always-on-top-toggle:hover {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .always-on-top-toggle.enabled {
    background: rgba(124, 92, 252, 0.15);
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .spinner.small {
    width: 12px;
    height: 12px;
    border-width: 1.5px;
  }

  .session-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-md);
  }

  .loading-state,
  .error-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-sm);
    padding: var(--space-xl);
    color: var(--color-text-muted);
    text-align: center;
  }

  .empty-state svg {
    opacity: 0.4;
    margin-bottom: var(--space-sm);
  }

  .empty-hint {
    font-size: 0.8rem;
    opacity: 0.7;
  }

  .error-state {
    color: var(--color-error);
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid rgba(100, 100, 100, 0.3);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  .session-card {
    display: flex;
    gap: var(--space-sm);
    padding: var(--space-md);
    margin-bottom: var(--space-sm);
    background: var(--color-bg-card);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-sm);
  }

  .session-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    margin-top: 6px;
    flex-shrink: 0;
  }

  .session-content {
    flex: 1;
    min-width: 0;
  }

  .session-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-sm);
    margin-bottom: 4px;
  }

  .project-name {
    font-weight: 600;
    color: var(--color-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .session-actions {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-shrink: 0;
  }

  .session-time {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    flex-shrink: 0;
  }

  .action-btn {
    display: inline-flex;
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
    background: var(--color-bg);
  }

  .focus-btn:hover {
    color: var(--color-info);
  }

  .focus-btn.not-found {
    color: var(--color-warning);
    animation: shake 0.3s ease-in-out;
  }

  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    25% { transform: translateX(-2px); }
    75% { transform: translateX(2px); }
  }

  .expand-btn:hover {
    color: var(--color-primary);
  }

  .delete-btn:hover {
    color: var(--color-error);
  }

  .session-details {
    display: flex;
    gap: var(--space-md);
    margin-bottom: 4px;
    font-size: 0.8rem;
  }

  .session-project {
    font-weight: 500;
    color: var(--color-text);
  }

  .session-id {
    font-family: monospace;
    color: var(--color-text-muted);
  }

  .session-state {
    font-weight: 500;
  }

  .session-path {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .session-json {
    margin-top: var(--space-sm);
    padding: var(--space-sm);
    background: var(--color-bg);
    border-radius: var(--radius-sm);
    overflow-x: auto;
  }

  .session-json pre {
    margin: 0;
    font-family: monospace;
    font-size: 0.75rem;
    color: var(--color-text);
    white-space: pre-wrap;
    word-break: break-all;
  }

  .session-card.expanded {
    border: 1px solid var(--color-border);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
