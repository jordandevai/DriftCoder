# DriftCode Memory Bank

## Project Overview

**DriftCode** is a lightweight, cross-platform SSH-based code editor built with Tauri v2. It enables developers to remotely edit code on their machines via SSH without any server-side installation beyond standard SSH.

**Key Differentiators:**
- Zero server-side footprint (SSH only)
- Cross-platform: Desktop (Linux, Windows, macOS) and Mobile (Android, iOS)
- Lightweight bundle (~15MB vs 200MB+ Electron apps)
- Native performance via Tauri

---

## Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| Framework | Tauri v2 | Cross-platform native wrapper |
| Frontend | Svelte 5 + TypeScript | Reactive UI with minimal overhead |
| Build Tool | Vite | Fast dev server and optimized builds |
| Styling | Tailwind CSS | Utility-first CSS framework |
| Code Editor | CodeMirror 6 | Lightweight, extensible editor |
| Terminal | xterm.js | Industry-standard terminal emulator |
| SSH (Rust) | russh | Pure Rust async SSH client |
| State | Svelte stores | Reactive state management |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                       DRIFTCODE APP                             │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    FRONTEND (Svelte)                      │  │
│  │  UI Components • State Management • Panel System          │  │
│  │  CodeMirror (Editor) • xterm.js (Terminal)               │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                  │
│                         Tauri IPC                               │
│                              │                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    BACKEND (Rust)                         │  │
│  │  SSH Client • SFTP Handler • PTY Manager                  │  │
│  │  File Cache • Credential Storage                          │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                               │
                          SSH (Port 22)
                               │
                               ▼
                    ┌────────────────────┐
                    │   REMOTE MACHINE   │
                    └────────────────────┘
```

---

## Directory Structure

```
driftcode/
├── src/                          # Svelte frontend
│   ├── lib/
│   │   ├── components/
│   │   │   ├── layout/           # Layout system components
│   │   │   │   ├── MainLayout.svelte
│   │   │   │   ├── SplitPane.svelte
│   │   │   │   ├── PanelGroup.svelte
│   │   │   │   ├── TabBar.svelte
│   │   │   │   └── StatusBar.svelte
│   │   │   ├── panels/           # Panel content types
│   │   │   │   ├── EditorPanel.svelte
│   │   │   │   ├── TerminalPanel.svelte
│   │   │   │   └── FileTreePanel.svelte
│   │   │   ├── connection/       # Connection UI
│   │   │   │   ├── ConnectionScreen.svelte
│   │   │   │   ├── ConnectionForm.svelte
│   │   │   │   └── ConnectionList.svelte
│   │   │   └── shared/           # Reusable components
│   │   │       ├── Button.svelte
│   │   │       ├── Input.svelte
│   │   │       └── Modal.svelte
│   │   ├── stores/               # Svelte stores
│   │   │   ├── connection.ts     # Connection state
│   │   │   ├── files.ts          # File system state
│   │   │   ├── layout.ts         # Layout/panel state
│   │   │   └── settings.ts       # App settings
│   │   ├── utils/                # Utilities
│   │   │   ├── tauri.ts          # Tauri IPC wrappers
│   │   │   └── languages.ts      # Language detection
│   │   └── types/                # TypeScript types
│   │       └── index.ts
│   ├── App.svelte                # Root component
│   ├── main.ts                   # Entry point
│   └── app.css                   # Global styles
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── commands/             # Tauri IPC commands
│   │   │   ├── mod.rs
│   │   │   ├── connection.rs     # SSH connect/disconnect
│   │   │   ├── filesystem.rs     # SFTP operations
│   │   │   └── terminal.rs       # PTY management
│   │   ├── ssh/                  # SSH implementation
│   │   │   ├── mod.rs
│   │   │   ├── client.rs         # SSH connection wrapper
│   │   │   ├── sftp.rs           # SFTP session handling
│   │   │   ├── pty.rs            # PTY channel management
│   │   │   └── auth.rs           # Authentication
│   │   ├── credentials/          # Credential storage
│   │   │   ├── mod.rs
│   │   │   └── store.rs
│   │   ├── state.rs              # AppState management
│   │   ├── lib.rs                # Library exports
│   │   └── main.rs               # Entry point
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── capabilities/
├── package.json
├── svelte.config.js
├── vite.config.ts
├── tailwind.config.js
├── tsconfig.json
└── README.md
```

---

## Module Dependencies

### Frontend Modules

**App.svelte**
- Uses: `connectionStore`, `ConnectionScreen`, `MainLayout`
- Purpose: Root component, routes between connection and main views

**stores/connection.ts**
- Uses: `tauri.ts` (IPC calls)
- Purpose: Manage SSH connection state, profiles

**stores/files.ts**
- Uses: `tauri.ts` (SFTP operations)
- Purpose: File tree state, open files, dirty tracking

**stores/layout.ts**
- Uses: None
- Purpose: Panel layout, tab management, split pane state

**components/layout/MainLayout.svelte**
- Uses: `layoutStore`, `SplitPane`, `FileTreePanel`, `StatusBar`
- Purpose: Main IDE layout container

**components/panels/EditorPanel.svelte**
- Uses: `filesStore`, CodeMirror
- Purpose: Code editing with syntax highlighting

**components/panels/TerminalPanel.svelte**
- Uses: `tauri.ts`, xterm.js
- Purpose: Interactive terminal via PTY

### Backend Modules

**commands/connection.rs**
- Uses: `ssh/client.rs`, `ssh/auth.rs`, `credentials/store.rs`
- Purpose: SSH connect/disconnect/test commands

**commands/filesystem.rs**
- Uses: `ssh/sftp.rs`, `state.rs`
- Purpose: SFTP file operations (list, read, write, stat)

**commands/terminal.rs**
- Uses: `ssh/pty.rs`, `state.rs`
- Purpose: Terminal create/write/resize/close

**ssh/client.rs**
- Uses: `russh` crate, `ssh/auth.rs`
- Purpose: SSH connection management

**ssh/sftp.rs**
- Uses: `russh_sftp` crate
- Purpose: SFTP subsystem operations

**ssh/pty.rs**
- Uses: `russh` crate
- Purpose: PTY channel management

**state.rs**
- Uses: All SSH modules
- Purpose: AppState with connection pool, active sessions

---

## Key Data Models

### TypeScript (Frontend)

```typescript
interface ConnectionProfile {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  authMethod: 'key' | 'password';
  keyPath?: string;
  lastProject?: string;
  autoConnect?: boolean;
}

interface FileEntry {
  name: string;
  path: string;
  isDirectory: boolean;
  size: number;
  mtime: number;
}

interface OpenFile {
  path: string;
  content: string;
  language: string;
  dirty: boolean;
  remoteMtime: number;
}

interface Panel {
  id: string;
  type: 'editor' | 'terminal';
  title: string;
  filePath?: string;
  terminalId?: string;
}
```

### Rust (Backend)

```rust
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: AuthMethod,
    pub key_path: Option<String>,
}

pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub mtime: i64,
}
```

---

## Tauri IPC Commands

### Connection
- `ssh_connect(profile)` → `Result<ConnectionId, Error>`
- `ssh_disconnect(conn_id)` → `Result<(), Error>`
- `ssh_test_connection(profile)` → `Result<bool, Error>`

### File System
- `sftp_list_dir(conn_id, path)` → `Result<Vec<FileEntry>, Error>`
- `sftp_read_file(conn_id, path)` → `Result<String, Error>`
- `sftp_write_file(conn_id, path, content)` → `Result<FileMeta, Error>`
- `sftp_stat(conn_id, path)` → `Result<FileMeta, Error>`
- `sftp_create_file(conn_id, path)` → `Result<(), Error>`
- `sftp_create_dir(conn_id, path)` → `Result<(), Error>`
- `sftp_delete(conn_id, path)` → `Result<(), Error>`
- `sftp_rename(conn_id, old, new)` → `Result<(), Error>`

### Terminal
- `terminal_create(conn_id)` → `Result<TerminalId, Error>`
- `terminal_write(term_id, data)` → `Result<(), Error>`
- `terminal_resize(term_id, cols, rows)` → `Result<(), Error>`
- `terminal_close(term_id)` → `Result<(), Error>`

### Events (Rust → Frontend)
- `terminal_output(term_id, data)`
- `connection_status_changed(status)`

---

## Implementation Status

| Module | Status | Notes |
|--------|--------|-------|
| Project Setup | Complete | Tauri v2 + Svelte 5 + Vite |
| Rust SSH Client | Complete | Using russh + russh-sftp crates |
| SFTP Operations | Complete | Full CRUD file operations |
| PTY Management | Complete | Terminal via SSH channels |
| Credential Storage | Complete | Platform-specific keyring |
| Connection UI | Complete | Form + saved profiles + test |
| File Tree Panel | Complete | Lazy loading, context menu |
| CodeMirror Editor | Complete | Syntax highlighting, 10+ languages |
| xterm.js Terminal | Complete | PTY integration, resizing |
| Layout System | Complete | Tabs, resizable sidebar |
| Status Bar | Complete | Connection status, new terminal |

---

## Development Notes

### Conventions
- Use TypeScript strict mode
- Follow Rust clippy lints
- Use kebab-case for file names
- Use PascalCase for Svelte components
- Use snake_case for Rust functions

### Testing
- Frontend: Vitest for unit tests
- Backend: Rust built-in tests
- E2E: Tauri's webdriver testing (future)

### Build Commands
```bash
# Development
npm run tauri dev

# Build
npm run tauri build

# Type check
npm run check
```
