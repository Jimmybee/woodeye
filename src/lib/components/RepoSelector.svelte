<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";

  interface Props {
    value: string;
    onload: (path: string) => void;
    loading?: boolean;
  }

  let { value = $bindable(), onload, loading = false }: Props = $props();

  async function handleBrowse() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Git Repository",
    });

    if (selected && typeof selected === "string") {
      value = selected;
      onload(selected);
    }
  }

  function handleLoad() {
    if (value.trim()) {
      onload(value.trim());
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      handleLoad();
    }
  }
</script>

<div class="repo-selector">
  <input
    type="text"
    bind:value
    placeholder="/path/to/repository"
    onkeydown={handleKeydown}
    disabled={loading}
  />
  <button onclick={handleBrowse} disabled={loading} class="browse-btn">
    Browse
  </button>
  <button onclick={handleLoad} disabled={loading || !value.trim()} class="load-btn">
    {loading ? "Loading..." : "Load"}
  </button>
</div>

<style>
  .repo-selector {
    display: flex;
    gap: 0.5rem;
    padding: 1rem;
    background: var(--color-bg-card);
    border-bottom: 1px solid var(--color-border);
  }

  input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    font-size: 0.9rem;
    border: 1px solid var(--color-border);
    border-radius: 4px;
    background: var(--color-bg);
    color: var(--color-text);
    font-family: ui-monospace, monospace;
  }

  input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  input::placeholder {
    color: var(--color-text-muted);
  }

  button {
    padding: 0.5rem 1rem;
    font-size: 0.9rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 500;
    transition: opacity 0.15s;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .browse-btn {
    background: var(--color-bg);
    color: var(--color-text);
    border: 1px solid var(--color-border);
  }

  .browse-btn:hover:not(:disabled) {
    background: var(--color-border);
  }

  .load-btn {
    background: var(--color-primary);
    color: white;
  }

  .load-btn:hover:not(:disabled) {
    opacity: 0.9;
  }
</style>
