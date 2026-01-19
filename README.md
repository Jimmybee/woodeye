# Woodeye

A Git worktree status viewer built with Tauri.

## Features

- View all Git worktrees for any repository
- Browse commit history with pagination
- View diffs for commits and working changes
- Create, delete, and prune worktrees
- File system watching for automatic refresh
- Dark mode support

## Installation

### macOS

1. Download `Woodeye_x.x.x_aarch64.dmg` (Apple Silicon) or `Woodeye_x.x.x_x64.dmg` (Intel) from [Releases](../../releases)
2. Open the `.dmg` file and drag Woodeye to Applications
3. **First launch:** Right-click the app and select "Open" (required for unsigned apps)
4. **If you see "damaged" error:** Run `xattr -cr /Applications/Woodeye.app` in Terminal

### Windows

1. Download `Woodeye_x.x.x_x64-setup.exe` or `Woodeye_x.x.x_x64_en-US.msi` from [Releases](../../releases)
2. Run the installer
3. If Windows SmartScreen appears, click "More info" then "Run anyway"

### Linux

**AppImage:**
```bash
chmod +x Woodeye_x.x.x_amd64.AppImage
./Woodeye_x.x.x_amd64.AppImage
```

**Debian/Ubuntu:**
```bash
sudo dpkg -i woodeye_x.x.x_amd64.deb
```

## Development

### Prerequisites

- Node.js (LTS)
- Rust (stable)
- Platform dependencies:
  - **macOS:** Xcode Command Line Tools
  - **Linux:** `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`
  - **Windows:** WebView2 (usually pre-installed on Windows 10/11)

### Commands

```bash
# Development (hot reload)
npm run tauri dev

# Production build
npm run tauri build

# Type checking
npx svelte-check
cargo check --manifest-path src-tauri/Cargo.toml
```

## Architecture

- **Backend:** Rust + Tauri v2
- **Frontend:** Svelte 5 + TypeScript + Vite

## License

MIT
