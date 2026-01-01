import { writable, derived, get } from 'svelte/store';
import type {
	WorkspaceState,
	Session,
	SessionFileState,
	SessionLayoutState,
	ConnectionProfile,
	FileEntry,
	LayoutNode,
	PanelGroup
} from '$types';
import { invoke } from '$utils/tauri';

// Initial empty file state for new sessions
function createInitialFileState(): SessionFileState {
	return {
		tree: [],
		expandedPaths: new Set(),
		openFiles: new Map(),
		activeFilePath: null
	};
}

// Initial layout state for new sessions
function createInitialLayoutState(): SessionLayoutState {
	const mainGroup: PanelGroup = {
		id: 'main',
		panels: [],
		activePanelId: null
	};

	const root: LayoutNode = { type: 'leaf', groupId: 'main' };

	return {
		root,
		groups: new Map([['main', mainGroup]]),
		activeGroupId: 'main',
		fileTreeWidth: 250,
		fileTreeCollapsed: false
	};
}

// Sort entries: directories first, then alphabetically
function sortEntries(entries: FileEntry[]): FileEntry[] {
	return [...entries].sort((a, b) => {
		if (a.isDirectory && !b.isDirectory) return -1;
		if (!a.isDirectory && b.isDirectory) return 1;
		return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
	});
}

const initialState: WorkspaceState = {
	sessions: new Map(),
	activeSessionId: null,
	sessionOrder: []
};

function createWorkspaceStore() {
	const { subscribe, set, update } = writable<WorkspaceState>(initialState);

	// Track active connections and their session counts
	const activeConnections = writable<Map<string, { profile: ConnectionProfile; sessionCount: number }>>(new Map());

	return {
		subscribe,
		activeConnections: { subscribe: activeConnections.subscribe },

		/**
		 * Create a new session with the given connection and project root
		 */
		async createSession(
			connectionId: string,
			profile: ConnectionProfile,
			projectRoot: string
		): Promise<string> {
			const sessionId = crypto.randomUUID();

			// Load initial file tree
			let tree: FileEntry[] = [];
			try {
				tree = await invoke<FileEntry[]>('sftp_list_dir', {
					connId: connectionId,
					path: projectRoot
				});
				tree = sortEntries(tree.filter((e) => e.isDirectory || !e.name.startsWith('.')));
			} catch (error) {
				console.error('Failed to load initial file tree:', error);
			}

			// Generate display name
			const folderName = projectRoot.split('/').pop() || projectRoot;
			const displayName = `${profile.host}:${folderName}`;

			const session: Session = {
				id: sessionId,
				connectionId,
				connectionProfile: profile,
				projectRoot,
				displayName,
				fileState: {
					...createInitialFileState(),
					tree,
					expandedPaths: new Set([projectRoot])
				},
				terminalIds: [],
				layoutState: createInitialLayoutState()
			};

			// Update workspace state
			update((s) => ({
				...s,
				sessions: new Map(s.sessions).set(sessionId, session),
				sessionOrder: [...s.sessionOrder, sessionId],
				activeSessionId: sessionId
			}));

			// Track connection usage
			activeConnections.update((conns) => {
				const existing = conns.get(connectionId);
				if (existing) {
					conns.set(connectionId, { ...existing, sessionCount: existing.sessionCount + 1 });
				} else {
					conns.set(connectionId, { profile, sessionCount: 1 });
				}
				return new Map(conns);
			});

			return sessionId;
		},

		/**
		 * Close a session and auto-disconnect if it's the last session on a connection
		 */
		async closeSession(sessionId: string): Promise<void> {
			const state = get({ subscribe });
			const session = state.sessions.get(sessionId);
			if (!session) return;

			const connectionId = session.connectionId;

			// Close all terminals belonging to this session
			for (const terminalId of session.terminalIds) {
				try {
					await invoke('terminal_close', { termId: terminalId });
				} catch (error) {
					console.error('Failed to close terminal:', error);
				}
			}

			// Remove session from state
			update((s) => {
				const newSessions = new Map(s.sessions);
				newSessions.delete(sessionId);

				const newOrder = s.sessionOrder.filter((id) => id !== sessionId);

				// If closing active session, switch to another
				let newActiveId = s.activeSessionId;
				if (s.activeSessionId === sessionId) {
					const idx = s.sessionOrder.indexOf(sessionId);
					newActiveId = newOrder[Math.max(0, idx - 1)] || newOrder[0] || null;
				}

				return {
					...s,
					sessions: newSessions,
					sessionOrder: newOrder,
					activeSessionId: newActiveId
				};
			});

			// Decrement connection usage and auto-disconnect if needed
			activeConnections.update((conns) => {
				const existing = conns.get(connectionId);
				if (existing) {
					if (existing.sessionCount <= 1) {
						// Last session on this connection - disconnect
						conns.delete(connectionId);
						invoke('ssh_disconnect', { connId: connectionId }).catch((e) =>
							console.error('Failed to disconnect:', e)
						);
					} else {
						conns.set(connectionId, { ...existing, sessionCount: existing.sessionCount - 1 });
					}
				}
				return new Map(conns);
			});
		},

		/**
		 * Switch to a different session
		 */
		switchSession(sessionId: string): void {
			update((s) => {
				if (!s.sessions.has(sessionId)) return s;
				return { ...s, activeSessionId: sessionId };
			});
		},

		/**
		 * Reorder sessions (for drag-drop tab reordering)
		 */
		reorderSessions(fromIndex: number, toIndex: number): void {
			update((s) => {
				const newOrder = [...s.sessionOrder];
				const [moved] = newOrder.splice(fromIndex, 1);
				newOrder.splice(toIndex, 0, moved);
				return { ...s, sessionOrder: newOrder };
			});
		},

		/**
		 * Rename a session's display name
		 */
		renameSession(sessionId: string, displayName: string): void {
			update((s) => {
				const session = s.sessions.get(sessionId);
				if (!session) return s;

				const newSessions = new Map(s.sessions);
				newSessions.set(sessionId, { ...session, displayName });
				return { ...s, sessions: newSessions };
			});
		},

		/**
		 * Update a session's file state
		 */
		updateSessionFileState(
			sessionId: string,
			updater: (state: SessionFileState) => SessionFileState
		): void {
			update((s) => {
				const session = s.sessions.get(sessionId);
				if (!session) return s;

				const newSessions = new Map(s.sessions);
				newSessions.set(sessionId, {
					...session,
					fileState: updater(session.fileState)
				});
				return { ...s, sessions: newSessions };
			});
		},

		/**
		 * Update a session's layout state
		 */
		updateSessionLayoutState(
			sessionId: string,
			updater: (state: SessionLayoutState) => SessionLayoutState
		): void {
			update((s) => {
				const session = s.sessions.get(sessionId);
				if (!session) return s;

				const newSessions = new Map(s.sessions);
				newSessions.set(sessionId, {
					...session,
					layoutState: updater(session.layoutState)
				});
				return { ...s, sessions: newSessions };
			});
		},

		/**
		 * Add a terminal ID to a session
		 */
		addTerminalToSession(sessionId: string, terminalId: string): void {
			update((s) => {
				const session = s.sessions.get(sessionId);
				if (!session) return s;

				const newSessions = new Map(s.sessions);
				newSessions.set(sessionId, {
					...session,
					terminalIds: [...session.terminalIds, terminalId]
				});
				return { ...s, sessions: newSessions };
			});
		},

		/**
		 * Remove a terminal ID from a session
		 */
		removeTerminalFromSession(sessionId: string, terminalId: string): void {
			update((s) => {
				const session = s.sessions.get(sessionId);
				if (!session) return s;

				const newSessions = new Map(s.sessions);
				newSessions.set(sessionId, {
					...session,
					terminalIds: session.terminalIds.filter((id) => id !== terminalId)
				});
				return { ...s, sessions: newSessions };
			});
		},

		/**
		 * Get the connection ID for a session
		 */
		getConnectionId(sessionId: string): string | null {
			const state = get({ subscribe });
			return state.sessions.get(sessionId)?.connectionId || null;
		},

		/**
		 * Check if a connection has any active sessions
		 */
		hasSessionsOnConnection(connectionId: string): boolean {
			const state = get({ subscribe });
			for (const session of state.sessions.values()) {
				if (session.connectionId === connectionId) return true;
			}
			return false;
		},

		/**
		 * Close all sessions and disconnect all connections
		 */
		async closeAll(): Promise<void> {
			const state = get({ subscribe });
			for (const sessionId of state.sessionOrder) {
				await this.closeSession(sessionId);
			}
		},

		/**
		 * Reset to initial state
		 */
		reset(): void {
			set(initialState);
			activeConnections.set(new Map());
		}
	};
}

export const workspaceStore = createWorkspaceStore();

// Derived store for the active session
export const activeSession = derived(workspaceStore, ($ws) =>
	$ws.activeSessionId ? $ws.sessions.get($ws.activeSessionId) || null : null
);

// Derived store for ordered sessions (for tab rendering)
export const orderedSessions = derived(workspaceStore, ($ws) =>
	$ws.sessionOrder.map((id) => $ws.sessions.get(id)!).filter(Boolean)
);

// Derived store to check if there are any sessions
export const hasSessions = derived(workspaceStore, ($ws) => $ws.sessions.size > 0);
