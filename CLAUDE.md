# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Woodeye is a Git worktree status viewer built as a Tauri v2 desktop application. It displays worktree information, commit history, and diffs for any Git repository with multiple worktrees.

## Build and Development Commands

```bash
# Development - runs both Vite dev server and Tauri app
npm run tauri dev

# Build production app
npm run tauri build

# Frontend only (no Tauri shell)
npm run dev
npm run build

# Type checking (Svelte)
npx svelte-check

# Rust checks
cargo check --manifest-path src-tauri/Cargo.toml
```

## Architecture

### Tauri Backend (Rust) - `src-tauri/src/`

- **lib.rs** - Tauri app setup, plugin registration, and command handler registration
- **commands.rs** - Tauri command definitions that bridge frontend calls to git module
- **git.rs** - Core Git operations using libgit2 (git2 crate):
  - `get_all_worktrees()` - Lists main + linked worktrees with status
  - `get_commit_history()` - Paginated commit log
  - `get_commit_diff()` / `get_working_diff()` - Diff generation with hunks/lines
- **watcher.rs** - File system watcher using notify-debouncer-mini, emits `worktree-changed` events
- **types.rs** - Shared data structures (Worktree, CommitInfo, FileDiff, DiffHunk, etc.)

### Svelte Frontend - `src/`

- **App.svelte** - Main app component, manages worktree/commit selection state
- **lib/types.ts** - TypeScript types mirroring Rust types (must stay in sync)
- **lib/store.ts** - localStorage persistence for last opened repo path
- **lib/components/** - UI components:
  - WorktreeSelector - Repo path input and worktree list
  - CommitList - Scrollable commit history with working changes entry
  - CommitDiffView - Displays commit or working diff
  - DiffHunk - Renders individual diff hunks with syntax highlighting

### Frontend-Backend Communication

Commands are invoked via `@tauri-apps/api/core`:
- `list_worktrees(repoPath)` - Get all worktrees for a repo
- `start_watching(paths)` - Begin file watching
- `get_commit_history(worktreePath, limit, offset)` - Paginated commits
- `get_commit_diff(worktreePath, commitSha)` - Diff for a specific commit
- `get_working_diff(worktreePath)` - Staged + unstaged changes

Events are received via `@tauri-apps/api/event`:
- `worktree-changed` - Triggers UI refresh when files change

## Key Dependencies

**Rust:**
- `git2` - libgit2 bindings for Git operations
- `notify` / `notify-debouncer-mini` - File system watching
- `tauri` v2 with `tauri-plugin-dialog`

**Frontend:**
- Svelte 5 with runes (`$state`, `$derived`)
- Vite 6
- TypeScript
