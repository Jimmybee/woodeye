<script lang="ts">
  import type { DiffHunk } from "../types";

  interface Props {
    hunk: DiffHunk;
  }

  let { hunk }: Props = $props();

  function getLineClass(kind: string): string {
    switch (kind) {
      case "+":
        return "addition";
      case "-":
        return "deletion";
      default:
        return "context";
    }
  }
</script>

<div class="diff-hunk">
  <div class="hunk-header">
    <span class="hunk-info">{hunk.header}</span>
  </div>
  <div class="hunk-lines">
    {#each hunk.lines as line, i (i)}
      <div class="diff-line {getLineClass(line.kind)}">
        <span class="line-indicator">{line.kind === " " ? " " : line.kind}</span>
        <pre class="line-content">{line.content}</pre>
      </div>
    {/each}
  </div>
</div>

<style>
  .diff-hunk {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    overflow: hidden;
    margin-bottom: var(--space-sm);
  }

  .hunk-header {
    display: flex;
    align-items: center;
    padding: var(--space-sm) var(--space-md);
    background: var(--color-bg);
    border-bottom: 1px solid var(--color-border);
  }

  .hunk-info {
    font-family: ui-monospace, monospace;
    font-size: 0.75rem;
    color: var(--color-text-muted);
  }

  .hunk-lines {
    font-family: ui-monospace, monospace;
    font-size: 0.8rem;
    line-height: 1.6;
  }

  .diff-line {
    display: flex;
    min-height: 1.6em;
  }

  .diff-line.addition {
    background: rgba(34, 197, 94, 0.12);
  }

  .diff-line.deletion {
    background: rgba(248, 113, 113, 0.12);
  }

  .diff-line.context {
    background: transparent;
  }

  .line-indicator {
    width: 24px;
    flex-shrink: 0;
    text-align: center;
    user-select: none;
    font-weight: 600;
    padding-left: var(--space-sm);
  }

  .addition .line-indicator {
    color: var(--color-success);
  }

  .deletion .line-indicator {
    color: var(--color-error);
  }

  .context .line-indicator {
    color: var(--color-text-muted);
  }

  .context .line-content {
    color: var(--color-text-muted);
  }

  .line-content {
    margin: 0;
    padding: 0 var(--space-sm);
    white-space: pre;
    overflow-x: auto;
    flex: 1;
    tab-size: 4;
  }

  .addition .line-content {
    color: var(--color-text);
  }

  .deletion .line-content {
    color: var(--color-text);
  }
</style>
