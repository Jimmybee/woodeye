<script lang="ts">
  import type { CommitDiff, WorkingDiff, FileStatus, FileDiff } from "../types";
  import DiffHunk from "./DiffHunk.svelte";

  interface Props {
    diff: CommitDiff | null;
    workingDiff: WorkingDiff | null;
    loading?: boolean;
  }

  let { diff, workingDiff, loading = false }: Props = $props();

  let collapsedFiles: Set<string> = $state(new Set());

  function toggleFile(key: string) {
    if (collapsedFiles.has(key)) {
      collapsedFiles.delete(key);
    } else {
      collapsedFiles.add(key);
    }
    collapsedFiles = new Set(collapsedFiles);
  }

  function isCollapsed(key: string): boolean {
    return collapsedFiles.has(key);
  }

  function formatDate(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  function getStatusBadge(status: FileStatus): { label: string; class: string } {
    switch (status) {
      case "Added":
        return { label: "A", class: "added" };
      case "Modified":
        return { label: "M", class: "modified" };
      case "Deleted":
        return { label: "D", class: "deleted" };
      case "Renamed":
        return { label: "R", class: "renamed" };
      default:
        return { label: "?", class: "" };
    }
  }
</script>

<div class="commit-diff-view">
  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <span>Loading diff...</span>
    </div>
  {:else if workingDiff}
    <div class="diff-content">
      <div class="commit-header working-header">
        <div class="header-top">
          <h2 class="commit-message">Working Changes</h2>
        </div>
        <p class="commit-description">Uncommitted changes in your working directory</p>
        <div class="diff-stats">
          <div class="stat">
            <span class="stat-value">{workingDiff.stats.files_changed}</span>
            <span class="stat-label">files</span>
          </div>
          <div class="stat additions">
            <span class="stat-value">+{workingDiff.stats.insertions}</span>
            <span class="stat-label">additions</span>
          </div>
          <div class="stat deletions">
            <span class="stat-value">-{workingDiff.stats.deletions}</span>
            <span class="stat-label">deletions</span>
          </div>
        </div>
      </div>

      {#if workingDiff.staged_files.length > 0}
        <div class="section-header staged">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 11.08V12a10 10 0 11-5.93-9.14"/>
            <polyline points="22,4 12,14.01 9,11.01"/>
          </svg>
          <span>Staged Changes</span>
          <span class="section-count">{workingDiff.staged_files.length}</span>
        </div>
        <div class="file-diffs">
          {#each workingDiff.staged_files as file (file.path)}
            {@const badge = getStatusBadge(file.status)}
            {@const fileKey = `staged:${file.path}`}
            {@const collapsed = isCollapsed(fileKey)}
            <div class="file-diff">
              <button class="file-header" class:collapsed onclick={() => toggleFile(fileKey)}>
                <span class="collapse-icon">{collapsed ? '+' : '-'}</span>
                <span class="status-badge {badge.class}">{badge.label}</span>
                <span class="file-path">
                  {#if file.old_path}
                    <span class="old-path">{file.old_path}</span>
                    <span class="arrow">→</span>
                  {/if}
                  {file.path}
                </span>
              </button>

              {#if !collapsed}
                {#if file.binary}
                  <div class="binary-notice">Binary file</div>
                {:else if file.hunks.length === 0}
                  <div class="no-hunks">No changes to display</div>
                {:else}
                  <div class="hunks">
                    {#each file.hunks as hunk, i (i)}
                      <DiffHunk {hunk} />
                    {/each}
                  </div>
                {/if}
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      {#if workingDiff.unstaged_files.length > 0}
        <div class="section-header unstaged">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <path d="M12 6v6l4 2"/>
          </svg>
          <span>Unstaged Changes</span>
          <span class="section-count">{workingDiff.unstaged_files.length}</span>
        </div>
        <div class="file-diffs">
          {#each workingDiff.unstaged_files as file (file.path)}
            {@const badge = getStatusBadge(file.status)}
            {@const fileKey = `unstaged:${file.path}`}
            {@const collapsed = isCollapsed(fileKey)}
            <div class="file-diff">
              <button class="file-header" class:collapsed onclick={() => toggleFile(fileKey)}>
                <span class="collapse-icon">{collapsed ? '+' : '-'}</span>
                <span class="status-badge {badge.class}">{badge.label}</span>
                <span class="file-path">
                  {#if file.old_path}
                    <span class="old-path">{file.old_path}</span>
                    <span class="arrow">→</span>
                  {/if}
                  {file.path}
                </span>
              </button>

              {#if !collapsed}
                {#if file.binary}
                  <div class="binary-notice">Binary file</div>
                {:else if file.hunks.length === 0}
                  <div class="no-hunks">No changes to display</div>
                {:else}
                  <div class="hunks">
                    {#each file.hunks as hunk, i (i)}
                      <DiffHunk {hunk} />
                    {/each}
                  </div>
                {/if}
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else if diff}
    <div class="diff-content">
      <div class="commit-header">
        <div class="header-top">
          <h2 class="commit-message">{diff.commit.summary}</h2>
          <code class="hash">{diff.commit.short_hash}</code>
        </div>
        {#if diff.commit.message !== diff.commit.summary}
          <pre class="commit-body">{diff.commit.message.slice(diff.commit.summary.length).trim()}</pre>
        {/if}
        <div class="commit-meta">
          <div class="author-info">
            <div class="avatar">
              {diff.commit.author_name.charAt(0).toUpperCase()}
            </div>
            <div class="author-details">
              <span class="author-name">{diff.commit.author_name}</span>
              <span class="commit-date">{formatDate(diff.commit.timestamp)}</span>
            </div>
          </div>
        </div>
        <div class="diff-stats">
          <div class="stat">
            <span class="stat-value">{diff.stats.files_changed}</span>
            <span class="stat-label">files</span>
          </div>
          <div class="stat additions">
            <span class="stat-value">+{diff.stats.insertions}</span>
            <span class="stat-label">additions</span>
          </div>
          <div class="stat deletions">
            <span class="stat-value">-{diff.stats.deletions}</span>
            <span class="stat-label">deletions</span>
          </div>
        </div>
      </div>

      <div class="file-diffs">
        {#each diff.files as file (file.path)}
          {@const badge = getStatusBadge(file.status)}
          {@const fileKey = `commit:${file.path}`}
          {@const collapsed = isCollapsed(fileKey)}
          <div class="file-diff">
            <button class="file-header" class:collapsed onclick={() => toggleFile(fileKey)}>
              <span class="collapse-icon">{collapsed ? '+' : '-'}</span>
              <span class="status-badge {badge.class}">{badge.label}</span>
              <span class="file-path">
                {#if file.old_path}
                  <span class="old-path">{file.old_path}</span>
                  <span class="arrow">→</span>
                {/if}
                {file.path}
              </span>
            </button>

            {#if !collapsed}
              {#if file.binary}
                <div class="binary-notice">Binary file</div>
              {:else if file.hunks.length === 0}
                <div class="no-hunks">No changes to display</div>
              {:else}
                <div class="hunks">
                  {#each file.hunks as hunk, i (i)}
                    <DiffHunk {hunk} />
                  {/each}
                </div>
              {/if}
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <div class="empty">
      <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
        <polyline points="14,2 14,8 20,8"/>
        <line x1="16" y1="13" x2="8" y2="13"/>
        <line x1="16" y1="17" x2="8" y2="17"/>
        <polyline points="10,9 9,9 8,9"/>
      </svg>
      <span>Select a commit to view changes</span>
    </div>
  {/if}
</div>

<style>
  .commit-diff-view {
    height: 100%;
    overflow-y: auto;
  }

  .loading,
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-md);
    height: 100%;
    color: var(--color-text-muted);
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--color-border);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .diff-content {
    padding: var(--space-lg);
  }

  .commit-header {
    background: var(--color-bg);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
    margin-bottom: var(--space-lg);
  }

  .working-header {
    border-left: 3px solid var(--color-warning);
  }

  .header-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-md);
    margin-bottom: var(--space-sm);
  }

  .commit-message {
    font-size: 1.1rem;
    font-weight: 600;
    margin: 0;
    line-height: 1.4;
  }

  .commit-description {
    font-size: 0.85rem;
    color: var(--color-text-muted);
    margin: 0 0 var(--space-md) 0;
  }

  .commit-body {
    font-size: 0.85rem;
    color: var(--color-text-muted);
    margin: var(--space-sm) 0 var(--space-md) 0;
    white-space: pre-wrap;
    font-family: inherit;
    line-height: 1.5;
  }

  .hash {
    font-family: ui-monospace, monospace;
    font-size: 0.75rem;
    background: var(--color-bg-card);
    color: var(--color-primary);
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }

  .commit-meta {
    margin-bottom: var(--space-md);
  }

  .author-info {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--color-primary);
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
    font-size: 0.85rem;
  }

  .author-details {
    display: flex;
    flex-direction: column;
  }

  .author-name {
    font-weight: 500;
    font-size: 0.9rem;
  }

  .commit-date {
    font-size: 0.75rem;
    color: var(--color-text-muted);
  }

  .diff-stats {
    display: flex;
    gap: var(--space-lg);
    padding-top: var(--space-md);
    border-top: 1px solid var(--color-border);
  }

  .stat {
    display: flex;
    align-items: baseline;
    gap: var(--space-xs);
  }

  .stat-value {
    font-size: 1.1rem;
    font-weight: 600;
  }

  .stat-label {
    font-size: 0.75rem;
    color: var(--color-text-muted);
  }

  .stat.additions .stat-value {
    color: var(--color-success);
  }

  .stat.deletions .stat-value {
    color: var(--color-error);
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    margin: var(--space-lg) 0 var(--space-md) 0;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    font-weight: 600;
  }

  .section-header.staged {
    background: rgba(34, 197, 94, 0.1);
    color: var(--color-success);
  }

  .section-header.unstaged {
    background: rgba(245, 158, 11, 0.1);
    color: var(--color-warning);
  }

  .section-count {
    margin-left: auto;
    font-size: 0.75rem;
    padding: 2px 8px;
    background: rgba(255, 255, 255, 0.2);
    border-radius: var(--radius-sm);
  }

  .file-diffs {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .file-diff {
    border-radius: var(--radius-md);
    overflow: hidden;
    border: 1px solid var(--color-border);
  }

  .file-header {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    background: var(--color-bg);
    width: 100%;
    cursor: pointer;
    text-align: left;
    color: var(--color-text);
    font-size: inherit;
    font-family: inherit;
    border: none;
    transition: background-color 0.15s;
  }

  .file-header:hover {
    background: var(--color-border);
  }

  .collapse-icon {
    font-family: ui-monospace, monospace;
    font-size: 0.75rem;
    width: 16px;
    color: var(--color-text-muted);
  }

  .status-badge {
    font-size: 0.7rem;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 3px;
    font-family: ui-monospace, monospace;
  }

  .status-badge.added {
    background: var(--color-success);
    color: white;
  }

  .status-badge.modified {
    background: var(--color-warning);
    color: black;
  }

  .status-badge.deleted {
    background: var(--color-error);
    color: white;
  }

  .status-badge.renamed {
    background: var(--color-primary);
    color: white;
  }

  .file-path {
    font-family: ui-monospace, monospace;
    font-size: 0.85rem;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .old-path {
    color: var(--color-text-muted);
    text-decoration: line-through;
  }

  .arrow {
    color: var(--color-text-muted);
    margin: 0 var(--space-xs);
  }

  .binary-notice,
  .no-hunks {
    padding: var(--space-lg);
    text-align: center;
    color: var(--color-text-muted);
    font-size: 0.85rem;
    background: var(--color-bg-card);
  }

  .hunks {
    background: var(--color-bg-card);
  }

  .hunks :global(.diff-hunk) {
    border: none;
    border-radius: 0;
    margin-bottom: 0;
    border-top: 1px solid var(--color-border);
  }

  .hunks :global(.diff-hunk:first-child) {
    border-top: none;
  }
</style>
