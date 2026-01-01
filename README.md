# DriftCode

**Your code, wherever you drift. No server install. Just SSH.**

DriftCode is a lightweight, cross-platform code editor that enables developers to remotely edit code on their home or work machines via SSH. Unlike heavyweight solutions, DriftCode requires zero server-side installation beyond standard SSH.

Created by Jordan Gonzales (Jtech Minds LLC) — https://jtechminds.com • https://driftcoder.com

## Features

- **Zero Server Footprint**: Only requires standard SSH access
- **Cross-Platform**: Desktop (Linux, Windows, macOS) and Mobile (Android, iOS)
- **Lightweight**: ~15MB bundle vs 200MB+ for Electron apps
- **Full Code Editor**: Syntax highlighting, line numbers, search & replace
- **Integrated Terminal**: Full PTY terminal access
- **Flexible Layout**: Draggable panels and tabs

## Tech Stack

- **Framework**: Tauri v2
- **Frontend**: Svelte 5 + TypeScript
- **Build Tool**: Vite
- **Styling**: Tailwind CSS
- **Code Editor**: CodeMirror 6
- **Terminal**: xterm.js
- **SSH**: russh (Rust)

## Development

### Prerequisites

- Node.js 18+
- Rust 1.77+
- Platform-specific Tauri dependencies (see [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/))

### Setup

```bash
# Install dependencies
npm install

# Run development server
npm run tauri:dev

# Build for production
npm run tauri:build
```

### Project Structure

```
driftcode/
├── src/                    # Svelte frontend
│   ├── lib/
│   │   ├── components/     # UI components
│   │   ├── stores/         # State management
│   │   ├── utils/          # Utilities
│   │   └── types/          # TypeScript types
│   └── routes/             # SvelteKit routes
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── commands/       # Tauri IPC commands
│   │   ├── ssh/            # SSH implementation
│   │   └── credentials/    # Credential storage
│   └── Cargo.toml
└── package.json
```

## License

DriftCoder is **source-available**:

- Free for personal and other non-commercial use: see `LICENSE.md`
- Commercial use (subscription, $15/seat/month): see `COMMERCIAL_LICENSE.md`
