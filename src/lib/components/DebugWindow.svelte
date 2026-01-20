<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import type { DebugInfo } from "../types";

  let debugInfo: DebugInfo | null = $state(null);
  let error: string | null = $state(null);
  let autoRefresh = $state(true);
  let refreshInterval: number | null = null;

  async function loadDebugInfo() {
    try {
      debugInfo = await invoke<DebugInfo>("get_debug_info");
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  function formatAge(seconds: number): string {
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
    return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
  }

  function formatTimestamp(ts: number): string {
    if (ts === 0) return "N/A";
    return new Date(ts * 1000).toLocaleTimeString();
  }

  function toggleAutoRefresh() {
    autoRefresh = !autoRefresh;
    if (autoRefresh) {
      startAutoRefresh();
    } else {
      stopAutoRefresh();
    }
  }

  function startAutoRefresh() {
    if (refreshInterval) clearInterval(refreshInterval);
    refreshInterval = setInterval(loadDebugInfo, 2000);
  }

  function stopAutoRefresh() {
    if (refreshInterval) {
      clearInterval(refreshInterval);
      refreshInterval = null;
    }
  }

  onMount(() => {
    loadDebugInfo();
    if (autoRefresh) {
      startAutoRefresh();
    }
    return () => stopAutoRefresh();
  });
</script>

<div class="debug-window">
  <header>
    <h1>Woodeye Debug</h1>
    <div class="controls">
      <button onclick={loadDebugInfo}>Refresh</button>
      <label>
        <input type="checkbox" checked={autoRefresh} onchange={toggleAutoRefresh} />
        Auto-refresh
      </label>
    </div>
  </header>

  {#if error}
    <div class="error">{error}</div>
  {:else if debugInfo}
    <section class="info-section">
      <h2>Configuration</h2>
      <div class="info-grid">
        <span class="label">Status Directory:</span>
        <code class="value">{debugInfo.status_dir}</code>

        <span class="label">Hooks Configured:</span>
        <span class="value {debugInfo.hooks_configured ? 'yes' : 'no'}">
          {debugInfo.hooks_configured ? "Yes" : "No"}
        </span>

        <span class="label">Stale Threshold:</span>
        <span class="value">{debugInfo.stale_threshold_secs}s</span>

        <span class="label">Current Time:</span>
        <span class="value">{formatTimestamp(debugInfo.current_timestamp)}</span>
      </div>
    </section>

    <section class="info-section">
      <h2>Status Files ({debugInfo.status_files.length})</h2>
      {#if debugInfo.status_files.length === 0}
        <p class="empty">No status files found</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th>Project</th>
              <th>State</th>
              <th>Age</th>
              <th>Time</th>
              <th>Stale</th>
            </tr>
          </thead>
          <tbody>
            {#each debugInfo.status_files as file}
              <tr class:stale={file.is_stale}>
                <td class="project-path" title={file.project_path}>
                  {file.project_path.split("/").slice(-2).join("/")}
                </td>
                <td class="state state-{file.state}">{file.state}</td>
                <td class="age">{formatAge(file.age_seconds)}</td>
                <td class="time">{formatTimestamp(file.timestamp)}</td>
                <td class="stale-indicator">
                  {#if file.is_stale}
                    <span class="badge stale">STALE</span>
                  {:else if file.state.startsWith("working")}
                    <span class="badge working">WORKING</span>
                  {:else if file.state.startsWith("waiting") || file.state.startsWith("idle")}
                    <span class="badge waiting">WAITING</span>
                  {:else}
                    <span class="badge active">ACTIVE</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </section>
  {:else}
    <div class="loading">Loading...</div>
  {/if}
</div>

<style>
  .debug-window {
    padding: var(--space-lg);
    max-width: 100%;
    height: 100%;
    overflow: auto;
    background: var(--color-bg);
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-xl);
    padding-bottom: var(--space-md);
    border-bottom: 1px solid var(--color-border);
  }

  h1 {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0;
  }

  h2 {
    font-size: 1rem;
    font-weight: 600;
    margin: 0 0 var(--space-md) 0;
    color: var(--color-text-muted);
  }

  .controls {
    display: flex;
    align-items: center;
    gap: var(--space-md);
  }

  button {
    padding: var(--space-sm) var(--space-md);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg-card);
    color: var(--color-text);
    cursor: pointer;
    font-size: 0.875rem;
  }

  button:hover {
    background: var(--color-border);
  }

  label {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: 0.875rem;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .info-section {
    background: var(--color-bg-card);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
    margin-bottom: var(--space-lg);
    box-shadow: var(--shadow-sm);
  }

  .info-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--space-sm) var(--space-md);
    align-items: center;
  }

  .label {
    font-size: 0.875rem;
    color: var(--color-text-muted);
  }

  .value {
    font-size: 0.875rem;
  }

  .value.yes {
    color: var(--color-success);
  }

  .value.no {
    color: var(--color-error);
  }

  code {
    background: var(--color-bg);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }

  th, td {
    text-align: left;
    padding: var(--space-sm) var(--space-md);
    border-bottom: 1px solid var(--color-border);
  }

  th {
    font-weight: 600;
    color: var(--color-text-muted);
    font-size: 0.75rem;
    text-transform: uppercase;
  }

  tr.stale {
    opacity: 0.6;
  }

  .project-path {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .state {
    font-weight: 500;
  }

  .state-working {
    color: var(--color-success);
  }

  .state-idle {
    color: var(--color-warning);
  }

  .state-waiting_for_input {
    color: var(--color-warning);
  }

  .state-waiting_for_approval {
    color: var(--color-warning);
  }

  .age {
    font-family: ui-monospace, monospace;
    color: var(--color-text-muted);
  }

  .time {
    color: var(--color-text-muted);
  }

  .badge {
    font-size: 0.7rem;
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: 600;
    text-transform: uppercase;
  }

  .badge.stale {
    background: var(--color-error);
    color: white;
  }

  .badge.active {
    background: var(--color-success);
    color: white;
  }

  .badge.working {
    background: var(--color-success);
    color: white;
  }

  .badge.waiting {
    background: var(--color-warning);
    color: white;
  }

  .error {
    background: var(--color-error);
    color: white;
    padding: var(--space-md);
    border-radius: var(--radius-md);
  }

  .loading, .empty {
    color: var(--color-text-muted);
    font-style: italic;
  }
</style>
