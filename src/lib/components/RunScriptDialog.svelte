<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { ScriptResult } from "../types";

  interface Props {
    worktreePath: string;
    onClose: () => void;
    onSuccess: () => void;
  }

  let { worktreePath, onClose, onSuccess }: Props = $props();

  let branchName = $state("");
  let running = $state(false);
  let error = $state("");
  let result: ScriptResult | null = $state(null);

  async function handleRun() {
    if (!branchName.trim()) {
      error = "Branch name is required";
      return;
    }

    running = true;
    error = "";
    result = null;

    try {
      const scriptResult = await invoke<ScriptResult>("run_custom_script", {
        branchName: branchName.trim(),
        worktreePath,
      });

      result = scriptResult;

      if (scriptResult.success) {
        onSuccess();
        onClose();
      } else {
        error = scriptResult.stderr || `Script exited with code ${scriptResult.exit_code}`;
      }
    } catch (e) {
      error = String(e);
    } finally {
      running = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onClose();
    } else if (e.key === "Enter" && !running) {
      handleRun();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="overlay" onclick={onClose} role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="dialog" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="dialog-title" tabindex="-1">
    <div class="dialog-header">
      <h2 id="dialog-title">Run Script</h2>
      <button class="close-btn" onclick={onClose} disabled={running} aria-label="Close dialog">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18"/>
          <line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>
    </div>

    <div class="dialog-body">
      {#if error}
        <div class="error-message">{error}</div>
      {/if}

      <div class="form-group">
        <label for="branchName">Branch Name</label>
        <input
          id="branchName"
          type="text"
          bind:value={branchName}
          placeholder="feature/my-branch"
          disabled={running}
        />
        <p class="hint">This will be passed as the first argument to the script</p>
      </div>

      {#if result && !result.success && result.stdout}
        <div class="output-section">
          <span class="output-label">Output</span>
          <pre class="output">{result.stdout}</pre>
        </div>
      {/if}
    </div>

    <div class="dialog-footer">
      <button class="cancel-btn" onclick={onClose} disabled={running}>
        Cancel
      </button>
      <button class="run-btn" onclick={handleRun} disabled={running}>
        {#if running}
          <span class="spinner"></span>
          Running...
        {:else}
          Run
        {/if}
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: var(--color-bg-card);
    border-radius: var(--radius-lg);
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);
    width: 100%;
    max-width: 480px;
    max-height: 90vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-lg);
    border-bottom: 1px solid var(--color-border);
  }

  .dialog-header h2 {
    font-size: 1.1rem;
    font-weight: 600;
    margin: 0;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }

  .close-btn:hover:not(:disabled) {
    background: var(--color-bg);
    color: var(--color-text);
  }

  .close-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .dialog-body {
    padding: var(--space-lg);
    overflow-y: auto;
    flex: 1;
  }

  .error-message {
    padding: var(--space-sm) var(--space-md);
    background: rgba(248, 113, 113, 0.1);
    color: var(--color-error);
    border-radius: var(--radius-sm);
    margin-bottom: var(--space-md);
    font-size: 0.9rem;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .form-group {
    margin-bottom: var(--space-lg);
  }

  .form-group label {
    display: block;
    font-size: 0.85rem;
    font-weight: 500;
    margin-bottom: var(--space-sm);
    color: var(--color-text);
  }

  input {
    width: 100%;
    padding: var(--space-sm) var(--space-md);
    font-size: 0.9rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-text);
    font-family: inherit;
    transition: border-color 0.15s;
  }

  input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .hint {
    margin-top: var(--space-xs);
    font-size: 0.8rem;
    color: var(--color-text-muted);
  }

  .output-section {
    margin-top: var(--space-md);
  }

  .output-label {
    display: block;
    font-size: 0.85rem;
    font-weight: 500;
    margin-bottom: var(--space-sm);
    color: var(--color-text);
  }

  .output {
    padding: var(--space-sm) var(--space-md);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    font-family: monospace;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 150px;
    overflow-y: auto;
    margin: 0;
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-sm);
    padding: var(--space-lg);
    border-top: 1px solid var(--color-border);
    background: var(--color-bg);
  }

  .cancel-btn,
  .run-btn {
    padding: var(--space-sm) var(--space-lg);
    font-size: 0.9rem;
    font-weight: 500;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: opacity 0.15s, background-color 0.15s;
  }

  .cancel-btn {
    border: 1px solid var(--color-border);
    background: var(--color-bg-card);
    color: var(--color-text);
  }

  .cancel-btn:hover:not(:disabled) {
    background: var(--color-bg);
  }

  .cancel-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .run-btn {
    border: none;
    background: var(--color-primary);
    color: white;
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    min-width: 100px;
    justify-content: center;
  }

  .run-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .run-btn:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
