<script lang="ts">
  import type { Worktree } from "../types";

  interface Props {
    worktree: Worktree;
  }

  let { worktree }: Props = $props();

  let statusClass = $derived(
    worktree.status ? (worktree.status.is_clean ? "clean" : "dirty") : "loading"
  );
</script>

<article class="card {statusClass}">
  <header>
    <span class="status-dot"></span>
    <h3>{worktree.name}</h3>
    {#if worktree.is_main}
      <span class="badge main">main</span>
    {/if}
  </header>

  <div class="branch">
    {#if worktree.head.branch}
      <span class="branch-name">{worktree.head.branch}</span>
    {:else}
      <span class="detached">detached</span>
    {/if}
    <code class="sha">{worktree.head.commit_sha}</code>
  </div>

  <p class="commit-msg">{worktree.head.commit_message}</p>

  {#if worktree.status && !worktree.status.is_clean}
    <div class="status-counts">
      {#if worktree.status.staged > 0}
        <span class="staged" title="Staged">{worktree.status.staged}</span>
      {/if}
      {#if worktree.status.modified > 0}
        <span class="modified" title="Modified">M{worktree.status.modified}</span>
      {/if}
      {#if worktree.status.untracked > 0}
        <span class="untracked" title="Untracked">?{worktree.status.untracked}</span>
      {/if}
      {#if worktree.status.conflicted > 0}
        <span class="conflict" title="Conflicts">!{worktree.status.conflicted}</span>
      {/if}
    </div>
  {:else if !worktree.status}
    <div class="status-loading">Loading status...</div>
  {/if}

  <footer>
    <small class="path">{worktree.path}</small>
  </footer>
</article>

<style>
  .card {
    background: var(--color-bg-card);
    border-radius: 8px;
    padding: 1rem;
    border-left: 4px solid;
  }

  .card.clean {
    border-color: var(--color-success);
  }

  .card.dirty {
    border-color: var(--color-warning);
  }

  .card.loading {
    border-color: var(--color-text-muted);
  }

  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  h3 {
    font-size: 1.1rem;
    font-weight: 600;
    margin: 0;
    flex: 1;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
    flex-shrink: 0;
  }

  .clean .status-dot {
    background: var(--color-success);
  }

  .dirty .status-dot {
    background: var(--color-warning);
  }

  .loading .status-dot {
    background: var(--color-text-muted);
  }

  .badge {
    font-size: 0.7rem;
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    text-transform: uppercase;
    font-weight: 600;
  }

  .badge.main {
    background: var(--color-primary);
    color: white;
  }

  .branch {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .branch-name {
    font-weight: 500;
    color: var(--color-primary);
  }

  .detached {
    font-style: italic;
    color: var(--color-text-muted);
  }

  .sha {
    font-size: 0.8rem;
    color: var(--color-text-muted);
    background: var(--color-bg);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
  }

  .commit-msg {
    font-size: 0.9rem;
    color: var(--color-text-muted);
    margin: 0 0 0.75rem 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-counts {
    display: flex;
    gap: 0.75rem;
    font-family: ui-monospace, monospace;
    font-size: 0.875rem;
    margin-bottom: 0.75rem;
  }

  .staged {
    color: var(--color-success);
  }

  .staged::before {
    content: "+";
  }

  .modified {
    color: var(--color-warning);
  }

  .untracked {
    color: var(--color-text-muted);
  }

  .conflict {
    color: var(--color-error);
  }

  footer {
    border-top: 1px solid var(--color-border);
    padding-top: 0.5rem;
    margin-top: 0.5rem;
  }

  .path {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    word-break: break-all;
  }

  .status-loading {
    font-size: 0.8rem;
    color: var(--color-text-muted);
    font-style: italic;
    margin-bottom: 0.75rem;
  }
</style>
