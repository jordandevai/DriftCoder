# Product Requirements Document
## DriftCode: Lightweight SSH-Based Code Editor
### driftcoder.com

**Version:** 1.0  
**Date:** December 31, 2024  
**Status:** Draft

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Problem Statement](#2-problem-statement)
3. [Solution Overview](#3-solution-overview)
4. [Target Users](#4-target-users)
5. [Technical Architecture](#5-technical-architecture)
6. [Core Features](#6-core-features)
7. [User Interface Design](#7-user-interface-design)
8. [User Flows](#8-user-flows)
9. [Technical Specifications](#9-technical-specifications)
10. [Security Considerations](#10-security-considerations)
11. [Performance Requirements](#11-performance-requirements)
12. [Platform Support](#12-platform-support)
13. [Future Considerations](#13-future-considerations)
14. [Success Metrics](#14-success-metrics)
15. [Appendix](#15-appendix)

---

## 1. Executive Summary

DriftCode is a lightweight, cross-platform code editor that enables developers to remotely edit code on their home or work machines via SSH. Unlike heavyweight solutions (VS Code Remote, JetBrains Gateway), DriftCode requires zero server-side installation beyond standard SSH—the protocol every developer already has enabled.

**Value Proposition:** Your code, wherever you drift. No server install. Just SSH.

**Key Differentiators:**
- Zero server-side footprint (SSH only)
- Single app for desktop and mobile (Android/iOS)
- Lightweight (~10-15MB bundle vs 200MB+ for Electron apps)
- Native performance via Tauri

---

## 2. Problem Statement

### The Scenario

Developer Joe works on code all day at his home workstation. He wants to continue working while:
- Traveling (laptop, tablet)
- Relaxing in bed (tablet)
- Waiting at appointments (phone/tablet)
- At a coffee shop (lightweight laptop)

### Current Solutions & Their Problems

| Solution | Problem |
|----------|---------|
| VS Code Remote | Installs ~200MB server component on remote machine |
| JetBrains Gateway | Heavy, requires substantial bandwidth |
| code-server | Full VS Code installation on server |
| Pure SSH + vim/nano | Not everyone is proficient; poor mobile experience |
| GitHub Codespaces | Requires cloud hosting, not local machine |
| Sync tools (Syncthing) | Two-way sync conflicts, not real-time |

### Unmet Need

A simple, lightweight editor that:
- Works over standard SSH (no server installation)
- Runs on all platforms including mobile
- Provides a proper code editing experience (not just a terminal)
- Allows running commands and interacting with local AI models

---

## 3. Solution Overview

DriftCode is a native application built with Tauri v2 that provides:

1. **SSH-Native Architecture:** All operations use SSH protocol—file operations via SFTP subsystem, terminals via PTY channels
2. **Cross-Platform:** Single codebase for Linux, Windows, macOS, Android, and iOS
3. **Flexible Panel System:** Dock code editors, terminals, and other panels anywhere
4. **Local File Caching:** Fast performance with intelligent sync
5. **Full Code Editor:** Syntax highlighting, line numbers, auto-indent—not a text area

### What It Is NOT

- Not a full IDE (no LSP, no debugger, no extensions)
- Not a replacement for VS Code/JetBrains for primary development
- Not responsible for networking (user handles SSH accessibility)

---

## 4. Target Users

### Primary Persona: "Mobile Joe"

- Professional developer with a primary workstation
- Wants to continue work during downtime/travel
- Comfortable with SSH concepts
- Values simplicity over features
- Uses local AI models (Ollama, llama.cpp) for assistance

### User Skill Assumptions

- Can enable SSH on their machine (or follow a guide)
- Understands basic networking (IP addresses, ports)
- May use Tailscale/ngrok/port-forwarding (not our problem)
- Ranges from casual to power user

### Use Cases

| Use Case | Context | Needs |
|----------|---------|-------|
| Quick bug fix | On phone in taxi | View file, make small edit, save |
| Code review | On tablet on couch | Navigate project, read multiple files |
| Running scripts | At coffee shop | Edit file, run in terminal, check output |
| AI-assisted coding | Anywhere | Chat with local LLM, apply suggestions |
| Jupyter notebooks | Light data work | View/edit/run notebook cells |

---

## 5. Technical Architecture

### High-Level Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                       REMOTECODE APP                            │
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
│  │  File Cache • Credential Storage • Port Forwarding        │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                               │
                          SSH (Port 22)
                               │
                               ▼
                    ┌────────────────────┐
                    │   REMOTE MACHINE   │
                    │   (sshd running)   │
                    └────────────────────┘
```

### Technology Stack

| Layer | Technology | Rationale |
|-------|------------|-----------|
| Framework | Tauri v2 | Cross-platform (including mobile), small bundle, Rust backend |
| Frontend | Svelte + TypeScript | Minimal overhead, excellent for imperative library integration |
| Build Tool | Vite | Fast HMR, optimized builds |
| Styling | Tailwind CSS | Utility-first, rapid development |
| Code Editor | CodeMirror 6 | Lightweight, extensible, MIT licensed |
| Terminal | xterm.js | Industry standard, battle-tested |
| SSH (Rust) | russh | Pure Rust, async, actively maintained |
| State | Svelte stores + Zustand patterns | Simple, reactive |

### SSH Channel Architecture

```
Single SSH Connection
│
├── SFTP Subsystem (shared)
│   └── All file operations (read/write/list/stat)
│
├── PTY Channel 1 → Terminal Tab 1
├── PTY Channel 2 → Terminal Tab 2
├── PTY Channel N → Terminal Tab N
│
└── Port Forward Channel → Jupyter (localhost:8888)
```

**Key Design Decision:** SFTP for all file operations (not shell commands). One SFTP channel handles unlimited file operations across any directory. PTY channels are reserved for interactive terminals.

---

## 6. Core Features

### 6.1 Connection Management

**Connect to Remote Machine**
- Host, port, username input
- Authentication: SSH key (preferred) or password
- Test connection before saving
- Save multiple connection profiles

**Credential Storage**
- Option: "Remember this connection"
- Encrypted local storage (platform-appropriate encryption)
- SSH key file selection or paste

**Connection Handling**
- Silent auto-reconnect on disconnection
- Visual connection status indicator
- Reconnection attempts with exponential backoff
- Queue pending operations during reconnect

### 6.2 File System Navigation

**File Tree Panel**
- Hierarchical folder/file display
- Lazy loading (fetch children on expand)
- File type icons
- Current file highlighting
- Refresh capability

**Project Folder Selection**
- Browse and select working directory on connect
- Remember last opened project per connection
- Quick-switch between recent projects

**File Operations**
- Create file/folder
- Rename
- Delete (with confirmation)
- Duplicate

### 6.3 Code Editor

**Core Editing (CodeMirror 6)**
- Syntax highlighting (50+ languages via @codemirror/lang-*)
- Line numbers
- Auto-indent on new lines
- Bracket matching and auto-close
- Code folding
- Search and replace (Ctrl+F, Ctrl+H)
- Multiple cursors
- Undo/redo
- Soft wrap toggle

**File Handling**
- Open multiple files in tabs
- Unsaved change indicator (dot on tab)
- Save (Ctrl+S) → writes to remote immediately
- Auto-detect file type from extension
- Encoding: UTF-8 (default), detect others

**Large File Handling**
- Files < 1MB: Open normally
- Files ≥ 1MB: Warning dialog with file size
  - "Large file (X.X MB), may be slow"
  - [Open Anyway] [Cancel]

### 6.4 Conflict Detection & Resolution

**Detection**
- On save: Check remote file modification time
- If remote changed since last fetch → conflict

**Resolution Flow**
1. Block save attempt
2. Show warning: "File changed on server"
3. Display diff view (side-by-side using @codemirror/merge)
   - Left: Local version (yours)
   - Right: Remote version (server)
4. Options:
   - [Keep Local] → Overwrite remote with local
   - [Keep Remote] → Discard local, reload from remote
   - [Cancel] → Return to editing, no action

### 6.5 Terminal

**Interactive Shell**
- Full PTY session via SSH
- xterm.js rendering
- Multiple terminal tabs
- Resize handling (SIGWINCH)
- Copy/paste support
- Scrollback buffer

**Terminal Management**
- New terminal: Ctrl+`
- Close terminal tab
- Rename terminal tab
- Clear terminal buffer

### 6.6 Local File Caching

**Cache Strategy**
- Cache files locally for fast re-open
- Cache location: App data directory (platform-specific)
- Store file content + remote modification timestamp

**Sync Behavior**
- On file open: Compare remote mtime with cached mtime
  - Remote newer → fetch and update cache
  - Cache current → use cache (instant open)
- On save: Write to remote, update cache
- Background: Optional periodic sync check (configurable)

**Cache Management**
- Clear cache per connection
- Clear all cache
- Maximum cache size setting (default: 500MB)

### 6.7 Jupyter Notebook Support (Phase 2)

**Prerequisites**
- User has Jupyter running on remote machine
- DriftCode creates SSH tunnel (local:8888 → remote:8888)

**Features**
- Open .ipynb files in notebook viewer
- Render cells (code, markdown, output)
- Execute cells via Jupyter API
- Cell output display (text, images, HTML)
- Save notebook

### 6.8 Local AI Chat (Phase 2)

**Prerequisites**
- User has Ollama or compatible API running on remote

**Features**
- Chat panel
- Send context (current file, selection)
- Stream responses
- Apply code suggestions to editor

---

## 7. User Interface Design

### 7.1 Layout System

**Panel-Based Architecture**

All content types are "panels" that can be arranged flexibly:
- Editor panels (one per open file)
- Terminal panels
- Notebook panels (Phase 2)
- AI Chat panel (Phase 2)

**Layout Structure**

```
┌─────────────────────────────────────────────────────────────────┐
│ Menu Bar                                                        │
├───────────┬─────────────────────────────────────────────────────┤
│           │              Panel Area                             │
│   File    │   (splits horizontally and vertically)              │
│   Tree    │                                                     │
│           │   Contains one or more Panel Groups                 │
│  (fixed   │   Each group has tabs                               │
│   left)   │                                                     │
│           │                                                     │
├───────────┴─────────────────────────────────────────────────────┤
│ Status Bar                                                      │
└─────────────────────────────────────────────────────────────────┘
```

**Panel Groups**
- Each split region is a "panel group"
- Panel group has its own tab bar
- Tabs can be: editor, terminal, notebook, chat
- Active tab displayed in group's content area

### 7.2 Tab Behavior

**Tab Actions**
- Click: Switch to tab
- Middle-click: Close tab
- Drag within group: Reorder
- Drag to edge: Split and move to new group
- Drag to other group: Move to that group
- Right-click: Context menu (Close, Close Others, Close All, Split Right, Split Down)

**Drop Zones**

When dragging a tab, drop zones appear:
- Center: Add as tab in target group
- Left/Right: Split horizontally, create new group
- Top/Bottom: Split vertically, create new group

### 7.3 Default Layouts

**Simple (Default)**
```
┌──────────┬──────────────────────────────────────────────────┐
│  Files   │  Tab Bar (all panels)                           │
├──────────┼──────────────────────────────────────────────────┤
│          │                                                  │
│  Tree    │  Content (editor/terminal/etc)                  │
│          │                                                  │
└──────────┴──────────────────────────────────────────────────┘
```

**Editor + Bottom Terminal**
```
┌──────────┬──────────────────────────────────────────────────┐
│  Files   │  Editor Tabs                                     │
├──────────┼──────────────────────────────────────────────────┤
│          │  Editor Content                                  │
│  Tree    │                                                  │
│          ├──────────────────────────────────────────────────┤
│          │  Terminal Tabs                                   │
│          │  Terminal Content                                │
└──────────┴──────────────────────────────────────────────────┘
```

**Side-by-Side**
```
┌──────────┬─────────────────────┬────────────────────────────┐
│  Files   │  Editor Tabs        │  Terminal Tabs             │
├──────────┼─────────────────────┼────────────────────────────┤
│          │                     │                            │
│  Tree    │  Editor             │  Terminal                  │
│          │                     │                            │
└──────────┴─────────────────────┴────────────────────────────┘
```

### 7.4 Layout Presets

Users can:
- Switch between preset layouts (View → Layout)
- Save current layout as preset
- Reset to default layout

### 7.5 Mobile Layout

On mobile (Android/iOS), layout adapts:

```
┌─────────────────────────────┐
│  ☰   filename.py       ⋮   │  ← Header with menu
├─────────────────────────────┤
│                             │
│                             │
│   Single panel view         │  ← Swipe to switch panels
│   (full screen)             │
│                             │
│                             │
├─────────────────────────────┤
│ 📁  📄  >_  🤖  ⚙️         │  ← Bottom nav (Files, Editor, Terminal, AI, Settings)
└─────────────────────────────┘
```

**Mobile Adaptations:**
- Single panel visible at a time
- Bottom navigation for panel switching
- Slide-over file tree (hamburger menu)
- Toolbar for common actions (save, search, etc.)
- Virtual keyboard awareness

### 7.6 Status Bar

```
┌─────────────────────────────────────────────────────────────────┐
│ 🟢 user@host │ 📁 ~/project │ Ln 42, Col 8 │ UTF-8 │ Python   │
└─────────────────────────────────────────────────────────────────┘
```

**Elements:**
- Connection status (green=connected, yellow=reconnecting, red=disconnected)
- Click to disconnect/reconnect
- Current project root
- Cursor position (active editor)
- File encoding
- Language mode

---

## 8. User Flows

### 8.1 First Launch Flow

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   App Launch    │────▶│  Welcome Screen │────▶│ Connection Form │
│                 │     │  "Get Started"  │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                        │
                              ┌─────────────────────────┘
                              ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Main IDE View  │◀────│  Select Folder  │◀────│ Connection Test │
│                 │     │  to Open        │     │  "Connected!"   │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

### 8.2 Returning User Flow

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   App Launch    │────▶│ Saved Profiles  │────▶│  Auto-Connect   │
│                 │     │ "Home Dev PC"   │     │  (if enabled)   │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                        │
                                                        ▼
                                                ┌─────────────────┐
                                                │  Main IDE View  │
                                                │ (last project)  │
                                                └─────────────────┘
```

### 8.3 Editing Flow

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ Click file in   │────▶│ Check cache vs  │────▶│ Load in editor  │
│ tree            │     │ remote mtime    │     │ tab             │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                                               │
        │                                               ▼
        │                                       ┌─────────────────┐
        │                                       │ User edits...   │
        │                                       │ Tab shows "●"   │
        │                                       └─────────────────┘
        │                                               │
        │                                               ▼
        │                                       ┌─────────────────┐
        │                                       │ User saves      │
        │                                       │ (Ctrl+S)        │
        │                                       └─────────────────┘
        │                                               │
        │                       ┌───────────────────────┴───────────────────────┐
        │                       ▼                                               ▼
        │               ┌─────────────────┐                             ┌─────────────────┐
        │               │ Remote unchanged│                             │ Remote changed! │
        │               │ Save succeeds   │                             │ Conflict!       │
        │               └─────────────────┘                             └─────────────────┘
        │                                                                       │
        │                                                                       ▼
        │                                                               ┌─────────────────┐
        │                                                               │ Show diff view  │
        │                                                               │ User resolves   │
        │                                                               └─────────────────┘
```

### 8.4 Disconnection Flow

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ Connection      │────▶│ Status: Yellow  │────▶│ Auto-reconnect  │
│ drops           │     │ "Reconnecting"  │     │ attempt 1, 2... │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                        │
                        ┌───────────────────────────────┴───────────────────────────────┐
                        ▼                                                               ▼
                ┌─────────────────┐                                             ┌─────────────────┐
                │ Reconnected!    │                                             │ Failed after N  │
                │ Resume working  │                                             │ Show error      │
                │ Queue flushed   │                                             │ Manual retry    │
                └─────────────────┘                                             └─────────────────┘
```

---

## 9. Technical Specifications

### 9.1 Frontend Component Hierarchy

```
App
├── ConnectionScreen (shown when not connected)
│   ├── ConnectionList
│   └── ConnectionForm
│
└── MainLayout (shown when connected)
    ├── MenuBar
    ├── SplitPane (recursive)
    │   ├── FileTreePanel (fixed left)
    │   └── PanelArea
    │       └── PanelGroup
    │           ├── TabBar
    │           │   └── Tab (multiple)
    │           └── PanelContent
    │               ├── EditorPanel
    │               │   └── CodeMirror
    │               ├── TerminalPanel
    │               │   └── XTerm
    │               ├── NotebookPanel (Phase 2)
    │               └── AIChatPanel (Phase 2)
    └── StatusBar
```

### 9.2 State Management

**Global Stores (Svelte stores)**

```typescript
// connectionStore
{
  status: 'disconnected' | 'connecting' | 'connected' | 'reconnecting',
  activeConnection: ConnectionProfile | null,
  savedProfiles: ConnectionProfile[],
  error: string | null
}

// fileStore
{
  projectRoot: string,
  tree: FileNode[],
  openFiles: Map<string, OpenFile>,  // path → {content, dirty, remoteMtime}
  activeFilePath: string | null
}

// layoutStore
{
  layout: LayoutNode,  // recursive split structure
  activeGroupId: string,
  activePanelId: string
}

// settingsStore
{
  fontSize: number,
  tabSize: number,
  wordWrap: boolean,
  autosave: boolean,
  cacheEnabled: boolean,
  maxCacheSize: number
}
```

### 9.3 Tauri IPC Commands

**Connection Commands**
```
ssh_connect(profile: ConnectionProfile) → Result<ConnectionId, Error>
ssh_disconnect(connId: ConnectionId) → Result<void, Error>
ssh_test_connection(profile: ConnectionProfile) → Result<bool, Error>
```

**File Commands**
```
sftp_list_dir(connId, path) → Result<FileEntry[], Error>
sftp_read_file(connId, path) → Result<FileContent, Error>
sftp_write_file(connId, path, content) → Result<FileMeta, Error>
sftp_stat(connId, path) → Result<FileMeta, Error>
sftp_create_file(connId, path) → Result<void, Error>
sftp_create_dir(connId, path) → Result<void, Error>
sftp_delete(connId, path) → Result<void, Error>
sftp_rename(connId, oldPath, newPath) → Result<void, Error>
```

**Terminal Commands**
```
terminal_create(connId) → Result<TerminalId, Error>
terminal_write(termId, data: bytes) → Result<void, Error>
terminal_resize(termId, cols, rows) → Result<void, Error>
terminal_close(termId) → Result<void, Error>
```

**Events (Rust → Frontend)**
```
terminal_output(termId, data: bytes)
connection_status_changed(status)
file_changed_remotely(path)  // from background sync
```

### 9.4 Rust Backend Modules

```
src-tauri/src/
├── main.rs
├── lib.rs
├── commands/
│   ├── mod.rs
│   ├── connection.rs    // ssh_connect, disconnect, test
│   ├── filesystem.rs    // all sftp_* commands
│   └── terminal.rs      // terminal_* commands
├── ssh/
│   ├── mod.rs
│   ├── client.rs        // SSH connection wrapper
│   ├── sftp.rs          // SFTP session handling
│   ├── pty.rs           // PTY channel management
│   └── auth.rs          // Key/password authentication
├── cache/
│   ├── mod.rs
│   └── file_cache.rs    // Local file caching logic
├── credentials/
│   ├── mod.rs
│   └── store.rs         // Encrypted credential storage
└── state.rs             // AppState, connection pool
```

### 9.5 Data Models

**ConnectionProfile**
```typescript
interface ConnectionProfile {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  authMethod: 'key' | 'password';
  keyPath?: string;
  // Password stored separately in encrypted store
  lastProject?: string;
  autoConnect?: boolean;
}
```

**FileEntry**
```typescript
interface FileEntry {
  name: string;
  path: string;
  isDirectory: boolean;
  size: number;
  mtime: number;  // Unix timestamp
  permissions: string;
}
```

**OpenFile**
```typescript
interface OpenFile {
  path: string;
  content: string;
  language: string;
  dirty: boolean;
  localMtime: number;
  remoteMtime: number;
}
```

**LayoutNode**
```typescript
type LayoutNode = 
  | { type: 'leaf'; groupId: string }
  | { type: 'split'; direction: 'horizontal' | 'vertical'; children: LayoutNode[]; sizes: number[] };

interface PanelGroup {
  id: string;
  panels: Panel[];
  activePanelId: string;
}

interface Panel {
  id: string;
  type: 'editor' | 'terminal' | 'notebook' | 'chat';
  title: string;
  filePath?: string;      // for editor/notebook
  terminalId?: string;    // for terminal
}
```

---

## 10. Security Considerations

### 10.1 Credential Storage

| Platform | Storage Method |
|----------|---------------|
| Linux | libsecret (GNOME Keyring / KWallet) or encrypted file |
| macOS | Keychain |
| Windows | Windows Credential Manager |
| Android | EncryptedSharedPreferences |
| iOS | Keychain |

**Fallback:** Encrypted local file using platform-appropriate encryption (Tauri plugin handles this).

### 10.2 SSH Key Handling

- Keys never transmitted; used locally to authenticate
- Key passphrase: Prompt each session OR use OS keychain
- Key files: Read-only access, no modification
- Support formats: OpenSSH, PEM

### 10.3 No Server Component

- DriftCode installs nothing on the server
- All code runs client-side
- SSH is the only attack surface (which is already exposed)

### 10.4 Cache Security

- Cached files stored in app's private data directory
- Optional: Encrypt cache at rest (user setting)
- Cache clearable by user

### 10.5 Network

- All traffic encrypted via SSH
- No telemetry or analytics (unless user opts in)
- No external API calls (except optional update check)

---

## 11. Performance Requirements

### 11.1 Startup

| Metric | Target |
|--------|--------|
| Cold start to connection screen | < 1 second |
| Reconnect to saved profile | < 3 seconds |
| Load project tree (100 files) | < 2 seconds |

### 11.2 Editor

| Metric | Target |
|--------|--------|
| Open small file (<100KB) | < 500ms |
| Open medium file (100KB-1MB) | < 2 seconds |
| Save file | < 1 second (network dependent) |
| Keystroke latency | < 16ms (local rendering) |

### 11.3 Terminal

| Metric | Target |
|--------|--------|
| Input latency | Network RTT + <50ms processing |
| Output rendering | 60fps for scrolling |
| New terminal spawn | < 2 seconds |

### 11.4 Resource Usage

| Metric | Target |
|--------|--------|
| Memory (idle) | < 100MB |
| Memory (10 files open) | < 200MB |
| CPU (idle) | < 1% |
| Bundle size | < 20MB |

---

## 12. Platform Support

### 12.1 Desktop

| Platform | Minimum Version | WebView |
|----------|-----------------|---------|
| Linux | Ubuntu 20.04+ / equivalent | WebKitGTK |
| Windows | Windows 10+ | WebView2 (Edge) |
| macOS | 10.15 (Catalina)+ | WKWebView |

### 12.2 Mobile

| Platform | Minimum Version | Notes |
|----------|-----------------|-------|
| Android | Android 8.0 (API 26)+ | Android WebView |
| iOS | iOS 13+ | WKWebView |

### 12.3 Platform-Specific Considerations

**Android:**
- Handle virtual keyboard overlay
- Request background execution for connection persistence
- Handle app backgrounding (connection will drop; auto-reconnect on resume)

**iOS:**
- Similar keyboard handling
- More aggressive background killing; expect reconnection
- Keychain for credential storage

**Linux:**
- May need libwebkit2gtk-4.0 installed
- Wayland and X11 support

---

## 13. Future Considerations

### Phase 2 Features

| Feature | Priority | Complexity |
|---------|----------|------------|
| Jupyter notebook support | High | Medium |
| Local AI chat integration | High | Medium |
| CSV table preview | Medium | Low |
| Git status/diff in file tree | Medium | Low |
| Port forwarding UI (for web apps) | Medium | Low |
| Image/media preview | Low | Low |
| Markdown preview | Low | Low |
| Tmux session attachment | Low | Medium |
| Theming (dark mode) | Low | Low |

### Explicitly Out of Scope

| Feature | Reason |
|---------|--------|
| Full LSP support | Complexity; use VS Code for full IDE |
| Debugger integration | Complexity; use terminal debuggers |
| Extensions/plugins | Scope creep; maintenance burden |
| Real-time collaboration | Requires server component |
| Built-in VPN/tunneling | Use Tailscale/ngrok; not our problem |

---

## 14. Success Metrics

### 14.1 Technical Metrics

| Metric | Target |
|--------|--------|
| Crash rate | < 0.1% of sessions |
| Connection success rate | > 99% (given valid credentials) |
| Reconnection success rate | > 95% |
| File save success rate | > 99.9% |

### 14.2 User Metrics

| Metric | Target |
|--------|--------|
| Time to first file edit | < 60 seconds (new user) |
| Session length | > 10 minutes average |
| Return usage | > 50% users return within 7 days |

### 14.3 Quality Metrics

| Metric | Target |
|--------|--------|
| App store rating | > 4.0 stars |
| GitHub issues resolved | < 7 day average response |

---

## 15. Appendix

### 15.1 Glossary

| Term | Definition |
|------|------------|
| PTY | Pseudo-terminal; allows interactive shell sessions |
| SFTP | SSH File Transfer Protocol; file operations over SSH |
| Channel | Multiplexed stream within an SSH connection |
| Panel | A content area in the UI (editor, terminal, etc.) |
| Panel Group | A tabbed container holding one or more panels |

### 15.2 References

- [Tauri v2 Documentation](https://v2.tauri.app/)
- [Svelte Documentation](https://svelte.dev/)
- [CodeMirror 6 Documentation](https://codemirror.net/)
- [xterm.js Documentation](https://xtermjs.org/)
- [russh Crate](https://crates.io/crates/russh)
- [SSH Protocol RFC 4251-4254](https://www.rfc-editor.org/rfc/rfc4251)

### 15.3 License Compliance

All dependencies are MIT, Apache 2.0, or BSD licensed. No GPL or copyleft licenses in the dependency tree. A LICENSES file will be included in the distribution.

### 15.4 Project Structure

```
driftcode/
├── src/                          # Svelte frontend
│   ├── lib/
│   │   ├── components/
│   │   │   ├── layout/
│   │   │   ├── panels/
│   │   │   ├── editor/
│   │   │   ├── terminal/
│   │   │   └── connection/
│   │   ├── stores/
│   │   ├── utils/
│   │   └── tauri.ts
│   ├── routes/
│   ├── app.html
│   └── app.css
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── commands/
│   │   ├── ssh/
│   │   ├── cache/
│   │   ├── credentials/
│   │   └── main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── package.json
├── svelte.config.js
├── vite.config.ts
├── tailwind.config.js
└── README.md
```

---

**Document History**

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2024-12-31 | — | Initial PRD |

---

*End of Document*
