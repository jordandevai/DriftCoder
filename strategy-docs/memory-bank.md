---
schema_version: 2.0
last_updated_utc: 2026-01-07T13:33:25Z
processed_scopes:
  - directory: "/src"
    commit_hash: "792caa1a8f268a01a790c9739832aa19aa77a149"
---

# Project Memory Bank: DriftCode

## 1. Project Summary
DriftCode is a lightweight, cross-platform SSH-based code editor built on Tauri v2. It connects to your existing machines via standard SSH (no server-side agent) and provides a file explorer, multi-file editor, and integrated terminal. The UI is a SvelteKit app running in a native WebView, while the Rust backend handles SSH/SFTP/PTY and exposes capabilities via Tauri IPC. The project also targets mobile (Android/iOS) with lifecycle-aware connection persistence.

## 2. Technology Stack
*   **Frontend:** Svelte 5, SvelteKit, TypeScript, Tailwind CSS, CodeMirror 6, xterm.js
*   **Backend:** Tauri v2 (Rust), russh + russh-sftp, tauri-plugin-store, custom Android persistence plugin
*   **DevOps:** GitHub Actions (Android debug APK), Vite, npm scripts

## 3. Core Concepts & Data Models
*   **ConnectionProfile:** Saved SSH target + auth method, plus recent projects and bookmarks.
*   **ActiveConnection:** Runtime connection instance (`connectionId`) with status and session usage count.
*   **Workspace / Session:** Multi-project model; each session binds a connection to a project root and owns editor/layout/terminal state.
*   **FileEntry:** Remote directory entry used to build the explorer tree.
*   **OpenFile:** In-memory editor buffer with dirty flag and remote sync metadata.
*   **Layout / Panel:** Tabbed panel model for editor + terminal (per session, with global terminal registry for persistence).
*   **SettingsState / ThemeConfig:** Editor/terminal settings, theme mode, and per-theme overrides persisted locally.
*   **Notification / TraceEvent:** User-facing event stream plus optional debug tracing for connection diagnostics.

## 4. Primary User/Data Flows
*   **App Boot / Store Init:**
    1.  App shell mounts in `src/routes/+layout.svelte`.
    2.  Stores initialize (`connectionStore`, `settingsStore`, `workspaceStore`, `debugStore`) and may hydrate from persisted state.
    3.  When running in Tauri, Android lifecycle flags and disconnect requests are handled via `src/lib/utils/tauri.ts` IPC.
*   **Connect + Open Project:**
    1.  User enters connection details in `src/lib/components/connection/ConnectionForm.svelte`.
    2.  SSH test/connect flows invoke backend commands via `src/lib/utils/tauri.ts` and `src/lib/stores/connection.ts`.
    3.  After connect, user selects a remote folder in `src/lib/components/workspace/FolderSelectEmbedded.svelte`.
    4.  A workspace session is created in `src/lib/stores/workspace.ts` and the initial file tree is loaded via SFTP.
*   **Browse + Edit + Save Remote Files:**
    1.  User opens a file from `src/lib/components/panels/FileTreePanel.svelte`.
    2.  `src/lib/stores/files.ts` loads content via `sftp_read_file_with_stat` and tracks dirty buffers.
    3.  Editing happens in `src/lib/components/panels/EditorPanel.svelte` (CodeMirror), updating `fileStore`.
    4.  Save flows call SFTP write operations; conflicts open `src/lib/components/modals/ConflictResolutionModal.svelte`.
*   **Terminal Sessions (Optional tmux Persistence):**
    1.  User creates a terminal via menu/statusbar (`src/lib/utils/commands.ts`, `src/lib/components/layout/StatusBar.svelte`).
    2.  `src/lib/stores/terminal.ts` requests a PTY from Rust via `terminal_create` and registers a terminal panel.
    3.  `src/lib/components/panels/TerminalPanel.svelte` streams output via `terminal_output` events and writes input via IPC.
    4.  If tmux persistence is enabled, `terminalStore` injects a deterministic attach command to survive reconnects.

## 5. Codebase Index

- 📁 **src/** - SvelteKit frontend (SSR disabled; runs inside Tauri WebView).
  - 📄 **app.html**
    - **Responsibility:** Defines the HTML shell and `<head>` metadata for the SvelteKit app.
    - **Tags:** `sveltekit, html-shell, metadata`
    - **Uses:** `n/a`
  - 📄 **app.css**
    - **Responsibility:** Defines global styles, Tailwind layers, and app-wide UI tokens.
    - **Tags:** `css, tailwind, theming`
    - **Uses:** `n/a`
  - 📁 **routes/** - App entry + top-level layout.
    - 📄 **+layout.ts**
      - **Responsibility:** Forces client-side rendering and static prerender configuration.
      - **Tags:** `sveltekit, routing, config`
      - **Uses:** `n/a`
    - 📄 **+layout.svelte**
      - **Responsibility:** Bootstraps the app shell, initializes stores, and wires Android lifecycle/persistence IPC.
      - **Tags:** `startup, lifecycle, tauri`
      - **Uses:** `src/app.css, src/lib/stores/connection.ts, src/lib/stores/workspace.ts, src/lib/stores/settings.ts, src/lib/stores/debug.ts, src/lib/stores/terminal.ts, src/lib/utils/tauri.ts`
    - 📄 **+page.svelte**
      - **Responsibility:** Renders the main application layout container.
      - **Tags:** `routing, entry-point, ui`
      - **Uses:** `src/lib/components/layout/MainLayout.svelte`
  - 📁 **lib/** - Shared UI components, state stores, and utilities.
    - 📁 **types/** - Canonical frontend types used across stores/components.
      - 📄 **index.ts**
        - **Responsibility:** Defines core TypeScript interfaces for connections, workspace sessions, files, layout, settings, and IPC errors.
        - **Tags:** `types, data-models, contracts`
        - **Uses:** `n/a`
    - 📁 **stores/** - Stateful domains (Svelte stores) and cross-cutting derived selectors.
      - 📄 **index.ts**
        - **Responsibility:** Re-exports all stores and commonly-used derived selectors.
        - **Tags:** `state, re-exports, api-surface`
        - **Uses:** `src/lib/stores/connection.ts, src/lib/stores/files.ts, src/lib/stores/layout.ts, src/lib/stores/workspace.ts, src/lib/stores/terminal.ts, src/lib/stores/settings.ts, src/lib/stores/notifications.ts, src/lib/stores/debug.ts, src/lib/stores/diagnostics.ts, src/lib/stores/confirm.ts, src/lib/stores/prompt.ts, src/lib/stores/conflict.ts, src/lib/stores/settings-ui.ts`
      - 📄 **connection.ts**
        - **Responsibility:** Manages SSH connection lifecycle, saved profiles, host-key trust UX, and auto-reconnect for active sessions.
        - **Tags:** `state, ssh, reconnection`
        - **Uses:** `src/lib/utils/tauri.ts, src/lib/utils/storage.ts, src/lib/utils/ssh-hostkey.ts, src/lib/stores/confirm.ts, src/lib/stores/prompt.ts, src/lib/stores/notifications.ts, src/lib/types/index.ts`
      - 📄 **workspace.ts**
        - **Responsibility:** Manages multi-project workspace sessions (per-connection project roots) and persists/restores workspace state.
        - **Tags:** `state, workspace, persistence`
        - **Uses:** `src/lib/utils/tauri.ts, src/lib/utils/storage.ts, src/lib/utils/file-tree.ts, src/lib/stores/connection.ts, src/lib/stores/notifications.ts, src/lib/types/index.ts`
      - 📄 **files.ts**
        - **Responsibility:** Owns remote file tree state, open buffers, save/rename/create/delete operations, and remote change detection.
        - **Tags:** `state, sftp, editor-buffers`
        - **Uses:** `src/lib/utils/tauri.ts, src/lib/utils/languages.ts, src/lib/utils/file-tree.ts, src/lib/stores/workspace.ts, src/lib/types/index.ts`
      - 📄 **layout.ts**
        - **Responsibility:** Manages per-session tab/panel layout state (active panel, panel groups, file-tree sizing).
        - **Tags:** `state, layout, panels`
        - **Uses:** `src/lib/stores/workspace.ts, src/lib/types/index.ts`
      - 📄 **terminal.ts**
        - **Responsibility:** Manages terminal session registry, PTY creation/close, and optional tmux-based persistence across reconnects.
        - **Tags:** `state, terminal, tmux`
        - **Uses:** `src/lib/utils/tauri.ts, src/lib/stores/workspace.ts, src/lib/stores/layout.ts, src/lib/stores/settings.ts, src/lib/stores/notifications.ts, src/lib/types/index.ts`
      - 📄 **settings.ts**
        - **Responsibility:** Stores UI/editor/terminal settings and applies theme values to CSS variables with persisted storage.
        - **Tags:** `state, settings, theming`
        - **Uses:** `src/lib/utils/storage.ts, src/lib/utils/tauri.ts, src/lib/utils/theme.ts, src/lib/types/index.ts`
      - 📄 **settings-ui.ts**
        - **Responsibility:** Controls visibility state for the settings modal UI.
        - **Tags:** `state, ui, modal`
        - **Uses:** `n/a`
      - 📄 **notifications.ts**
        - **Responsibility:** Central notification center store with read/dismiss semantics and de-duplication (`notifyOnce`).
        - **Tags:** `state, notifications, ux`
        - **Uses:** `n/a`
      - 📄 **debug.ts**
        - **Responsibility:** Toggles and streams backend connection trace events for diagnostics and support.
        - **Tags:** `state, diagnostics, tracing`
        - **Uses:** `src/lib/utils/tauri.ts, src/lib/stores/notifications.ts`
      - 📄 **diagnostics.ts**
        - **Responsibility:** Controls visibility state for the diagnostics modal UI.
        - **Tags:** `state, ui, modal`
        - **Uses:** `n/a`
      - 📄 **confirm.ts**
        - **Responsibility:** Provides a queued confirm-dialog promise API to serialize confirmation prompts.
        - **Tags:** `state, modal, ux`
        - **Uses:** `n/a`
      - 📄 **prompt.ts**
        - **Responsibility:** Provides a single-active prompt-dialog promise API (text/password input).
        - **Tags:** `state, modal, ux`
        - **Uses:** `n/a`
      - 📄 **conflict.ts**
        - **Responsibility:** Tracks an active save-conflict context (file path + owning session) for merge resolution UX.
        - **Tags:** `state, conflicts, editor`
        - **Uses:** `src/lib/stores/workspace.ts`
    - 📁 **utils/** - Stateless helpers and wrappers.
      - 📄 **tauri.ts**
        - **Responsibility:** Wraps Tauri `invoke`/`listen` with typed errors and environment detection.
        - **Tags:** `tauri, ipc, errors`
        - **Uses:** `src/lib/types/index.ts`
      - 📄 **storage.ts**
        - **Responsibility:** Persists and restores connections, settings, and workspace state using `tauri-plugin-store`.
        - **Tags:** `persistence, storage, tauri`
        - **Uses:** `src/lib/types/index.ts`
      - 📄 **commands.ts**
        - **Responsibility:** Implements app-level command actions (save, new terminal, close panel/project) used by UI shortcuts.
        - **Tags:** `commands, shortcuts, ux`
        - **Uses:** `src/lib/stores/workspace.ts, src/lib/stores/files.ts, src/lib/stores/layout.ts, src/lib/stores/terminal.ts, src/lib/stores/confirm.ts, src/lib/stores/notifications.ts, src/lib/stores/conflict.ts`
      - 📄 **file-tree.ts**
        - **Responsibility:** Provides canonical sorting behavior for remote file entries (folders first).
        - **Tags:** `files, sorting, utility`
        - **Uses:** `src/lib/types/index.ts`
      - 📄 **languages.ts**
        - **Responsibility:** Detects language mode labels from filenames/extensions for editor configuration.
        - **Tags:** `editor, language-detection, utility`
        - **Uses:** `n/a`
      - 📄 **codemirror-languages.ts**
        - **Responsibility:** Lazily loads CodeMirror language extensions by name to keep bundle size small.
        - **Tags:** `codemirror, lazy-load, editor`
        - **Uses:** `n/a`
      - 📄 **ssh-hostkey.ts**
        - **Responsibility:** Parses typed host-key error contexts to drive trust/replace UX flows.
        - **Tags:** `ssh, security, parsing`
        - **Uses:** `n/a`
      - 📄 **theme.ts**
        - **Responsibility:** Defines default theme tokens and helper functions for applying UI/terminal theme values to CSS.
        - **Tags:** `theming, css-vars, ui`
        - **Uses:** `n/a`
    - 📁 **components/** - UI components (Svelte) organized by domain.
      - 📁 **shared/** - Reusable UI primitives.
        - 📄 **Button.svelte**
          - **Responsibility:** Provides a styled button primitive with variants, sizes, and loading state.
          - **Tags:** `ui, component, primitive`
          - **Uses:** `n/a`
        - 📄 **Input.svelte**
          - **Responsibility:** Provides a labeled input primitive with validation/error display.
          - **Tags:** `ui, component, primitive`
          - **Uses:** `n/a`
        - 📄 **Modal.svelte**
          - **Responsibility:** Provides a generic modal shell with sizing, backdrop close, and Escape handling.
          - **Tags:** `ui, component, modal`
          - **Uses:** `n/a`
      - 📁 **connection/** - Connection creation and selection UX.
        - 📄 **ConnectionScreen.svelte**
          - **Responsibility:** Entry screen for creating/editing/selecting saved SSH connections and launching connect flow.
          - **Tags:** `ui, ssh, onboarding`
          - **Uses:** `src/lib/stores/connection.ts, src/lib/stores/settings.ts, src/lib/stores/settings-ui.ts, src/lib/stores/notifications.ts, src/lib/stores/diagnostics.ts, src/lib/components/connection/ConnectionForm.svelte, src/lib/components/connection/ConnectionList.svelte, src/lib/types/index.ts`
        - 📄 **ConnectionForm.svelte**
          - **Responsibility:** Collects SSH connection details, supports test connection, and handles host-key trust prompts.
          - **Tags:** `ui, ssh, hostkey`
          - **Uses:** `src/lib/utils/tauri.ts, src/lib/utils/ssh-hostkey.ts, src/lib/stores/confirm.ts, src/lib/components/shared/Button.svelte, src/lib/components/shared/Input.svelte, src/lib/types/index.ts`
        - 📄 **ConnectionList.svelte**
          - **Responsibility:** Lists saved connections, supports quick connect (including password prompt), edit, delete, and recent/bookmark shortcuts.
          - **Tags:** `ui, ssh, navigation`
          - **Uses:** `src/lib/stores/confirm.ts, src/lib/components/shared/Button.svelte, src/lib/components/shared/Modal.svelte, src/lib/types/index.ts`
      - 📁 **workspace/** - Multi-project session (workspace) UX.
        - 📄 **AddProjectModal.svelte**
          - **Responsibility:** Adds an additional project tab by reusing an existing connection or creating a new one, then selecting a folder.
          - **Tags:** `ui, workspace, modal`
          - **Uses:** `src/lib/stores/connection.ts, src/lib/stores/workspace.ts, src/lib/stores/notifications.ts, src/lib/components/connection/ConnectionForm.svelte, src/lib/components/shared/Modal.svelte, src/lib/components/shared/Button.svelte, src/lib/components/workspace/FolderSelectEmbedded.svelte, src/lib/types/index.ts`
        - 📄 **FolderSelectEmbedded.svelte**
          - **Responsibility:** Embedded remote folder browser for selecting project roots and managing connection bookmarks/recent paths.
          - **Tags:** `ui, sftp, navigation`
          - **Uses:** `src/lib/utils/tauri.ts, src/lib/stores/connection.ts, src/lib/components/shared/Button.svelte, src/lib/types/index.ts`
      - 📁 **panels/** - Main IDE panels rendered in the layout system.
        - 📄 **FileTreePanel.svelte**
          - **Responsibility:** Renders the remote file explorer tree with expand/collapse, context menu, and create/rename/delete actions.
          - **Tags:** `ui, explorer, sftp`
          - **Uses:** `src/lib/stores/files.ts, src/lib/stores/layout.ts, src/lib/stores/confirm.ts, src/lib/stores/workspace.ts, src/lib/utils/commands.ts, src/lib/types/index.ts`
        - 📄 **EditorPanel.svelte**
          - **Responsibility:** Provides the CodeMirror editor surface bound to an `OpenFile` buffer with save/reload handling.
          - **Tags:** `ui, editor, codemirror`
          - **Uses:** `src/lib/stores/files.ts, src/lib/stores/settings.ts, src/lib/stores/notifications.ts, src/lib/stores/confirm.ts, src/lib/stores/prompt.ts, src/lib/stores/conflict.ts, src/lib/utils/codemirror-languages.ts`
        - 📄 **TerminalPanel.svelte**
          - **Responsibility:** Provides the xterm.js terminal surface, wiring IPC streams for PTY input/output and resize.
          - **Tags:** `ui, terminal, ipc`
          - **Uses:** `src/lib/utils/tauri.ts, src/lib/stores/settings.ts, src/lib/utils/theme.ts, src/lib/stores/notifications.ts`
      - 📁 **modals/** - Specialized modal flows.
        - 📄 **ConflictResolutionModal.svelte**
          - **Responsibility:** Resolves remote save conflicts by fetching server content and presenting a merge UI.
          - **Tags:** `ui, conflicts, codemirror`
          - **Uses:** `src/lib/stores/conflict.ts, src/lib/stores/files.ts, src/lib/stores/workspace.ts, src/lib/stores/confirm.ts, src/lib/stores/notifications.ts, src/lib/utils/codemirror-languages.ts, src/lib/components/shared/Modal.svelte, src/lib/components/shared/Button.svelte`
      - 📁 **layout/** - App chrome, layout, and global modals.
        - 📄 **MainLayout.svelte**
          - **Responsibility:** Top-level IDE container: session tabs, menu, file tree, panel area, and global keyboard shortcuts.
          - **Tags:** `ui, layout, shell`
          - **Uses:** `src/lib/stores/workspace.ts, src/lib/stores/connection.ts, src/lib/stores/layout.ts, src/lib/stores/files.ts, src/lib/stores/notifications.ts, src/lib/utils/commands.ts, src/lib/components/layout/MenuToolbar.svelte, src/lib/components/layout/ProjectTabs.svelte, src/lib/components/layout/PanelGroup.svelte, src/lib/components/panels/FileTreePanel.svelte`
        - 📄 **MenuToolbar.svelte**
          - **Responsibility:** Provides top menu actions (file/view/terminal/help) and opens settings/about/shortcuts modals.
          - **Tags:** `ui, menu, commands`
          - **Uses:** `src/lib/utils/commands.ts, src/lib/stores/workspace.ts, src/lib/stores/settings-ui.ts, src/lib/components/shared/Modal.svelte, src/lib/components/shared/Button.svelte`
        - 📄 **ProjectTabs.svelte**
          - **Responsibility:** Renders workspace session tabs (projects) with close-confirmation for unsaved changes.
          - **Tags:** `ui, workspace, tabs`
          - **Uses:** `src/lib/stores/workspace.ts, src/lib/stores/confirm.ts, src/lib/types/index.ts`
        - 📄 **PanelGroup.svelte**
          - **Responsibility:** Renders tab bar + active panels, while keeping terminals mounted for cross-session persistence.
          - **Tags:** `ui, panels, terminal`
          - **Uses:** `src/lib/stores/layout.ts, src/lib/stores/workspace.ts, src/lib/stores/files.ts, src/lib/stores/terminal.ts, src/lib/stores/notifications.ts, src/lib/stores/connection.ts, src/lib/components/layout/TabBar.svelte, src/lib/components/panels/EditorPanel.svelte, src/lib/components/panels/TerminalPanel.svelte, src/lib/types/index.ts`
        - 📄 **TabBar.svelte**
          - **Responsibility:** Renders a single panel group's tabs with close affordances and dirty indicators.
          - **Tags:** `ui, tabs, panels`
          - **Uses:** `src/lib/stores/files.ts, src/lib/types/index.ts`
        - 📄 **StatusBar.svelte**
          - **Responsibility:** Displays connection/project status and provides quick actions (new terminal, diagnostics, notifications).
          - **Tags:** `ui, status, actions`
          - **Uses:** `src/lib/stores/workspace.ts, src/lib/stores/connection.ts, src/lib/stores/files.ts, src/lib/stores/terminal.ts, src/lib/stores/notifications.ts, src/lib/stores/diagnostics.ts, src/lib/utils/languages.ts`
        - 📄 **NotificationCenter.svelte**
          - **Responsibility:** Displays notifications with export/copy utilities and a combined debug report (frontend + backend).
          - **Tags:** `ui, notifications, diagnostics`
          - **Uses:** `src/lib/stores/notifications.ts, src/lib/stores/debug.ts, src/lib/utils/tauri.ts, src/lib/components/shared/Modal.svelte, src/lib/components/shared/Button.svelte`
        - 📄 **DiagnosticsModal.svelte**
          - **Responsibility:** Displays trace events, manages tracing toggle, and surfaces trusted host keys for inspection/forget.
          - **Tags:** `ui, diagnostics, ssh`
          - **Uses:** `src/lib/stores/diagnostics.ts, src/lib/stores/debug.ts, src/lib/utils/tauri.ts, src/lib/components/shared/Modal.svelte, src/lib/components/shared/Button.svelte`
        - 📄 **SettingsModal.svelte**
          - **Responsibility:** Edits app settings (appearance/editor/terminal) including theme overrides applied live.
          - **Tags:** `ui, settings, theming`
          - **Uses:** `src/lib/stores/settings.ts, src/lib/stores/settings-ui.ts, src/lib/utils/theme.ts, src/lib/components/shared/Modal.svelte, src/lib/components/shared/Button.svelte`
        - 📄 **ConfirmHost.svelte**
          - **Responsibility:** Presents queued confirmation dialogs issued by `confirmStore`.
          - **Tags:** `ui, modal, confirmation`
          - **Uses:** `src/lib/stores/confirm.ts, src/lib/components/shared/Modal.svelte, src/lib/components/shared/Button.svelte`
        - 📄 **PromptModal.svelte**
          - **Responsibility:** Presents the active prompt dialog issued by `promptStore` and returns user input.
          - **Tags:** `ui, modal, input`
          - **Uses:** `src/lib/stores/prompt.ts, src/lib/components/shared/Modal.svelte, src/lib/components/shared/Button.svelte`
        - 📄 **FolderSelect.svelte**
          - **Responsibility:** Deprecated folder selection UI kept for migration/back-compat.
          - **Tags:** `ui, deprecated, migration`
          - **Uses:** `n/a`
