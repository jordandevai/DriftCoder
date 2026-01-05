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
import { invoke, isTauri } from '$utils/tauri';
import { sortEntries } from '$utils/file-tree';
import { connectionStore } from './connection';
import { notificationsStore } from './notifications';
import { loadSavedWorkspace, saveWorkspace } from '$utils/storage';

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

function nextTerminalOrdinal(session: Session): number {
	const ordinals = session.terminalOrdinals ?? {};
	const max = Object.values(ordinals).reduce((m, n) => (Number.isFinite(n) ? Math.max(m, n) : m), 0);
	return max + 1;
}

const initialState: WorkspaceState = {
	sessions: new Map(),
	activeSessionId: null,
	sessionOrder: []
};

function createWorkspaceStore() {
	const { subscribe, set, update } = writable<WorkspaceState>(initialState);
	let initialized = false;
	let saveTimer: number | null = null;

	function schedulePersist(next: WorkspaceState) {
		if (!isTauri()) return;
		if (!initialized) return;
		if (typeof window === 'undefined') return;

		if (saveTimer !== null) window.clearTimeout(saveTimer);
		saveTimer = window.setTimeout(() => {
			void saveWorkspace(next);
		}, 500);
	}

	const unsubscribePersist = subscribe((s) => schedulePersist(s));

	return {
		subscribe,

		async init(): Promise<void> {
			if (initialized) return;
			initialized = true;

			if (!isTauri()) return;
			const restored = await loadSavedWorkspace();
			if (!restored) return;

			// Mark restored sessions as disconnected (backend connections do not survive app background/kill).
			const nextSessions = new Map(restored.sessions);
			for (const [id, session] of nextSessions) {
				nextSessions.set(id, {
					...session,
					connectionStatus: 'disconnected',
					connectionDetail: session.connectionDetail ?? 'Restored session (reconnect required)',
					terminalOrdinals: session.terminalOrdinals ?? {}
				});
			}

			// Validate active session
			const activeSessionId =
				restored.activeSessionId && nextSessions.has(restored.activeSessionId)
					? restored.activeSessionId
					: restored.sessionOrder[0] || null;

			set({
				sessions: nextSessions,
				activeSessionId,
				sessionOrder: restored.sessionOrder.filter((id) => nextSessions.has(id))
			});

			// Ensure connections are registered in connectionStore so reconnect works.
			const counts = new Map<string, number>();
			for (const session of nextSessions.values()) {
				counts.set(session.connectionId, (counts.get(session.connectionId) ?? 0) + 1);
			}
			for (const [connectionId, count] of counts) {
				const profile = Array.from(nextSessions.values()).find((s) => s.connectionId === connectionId)
					?.connectionProfile;
				if (profile) connectionStore.registerDisconnected(connectionId, profile, count);
			}
		},

		/**
		 * Create a new session with the given connection and project root
		 */
		async createSession(
			connectionId: string,
			profile: ConnectionProfile,
			projectRoot: string
		): Promise<string> {
			if (!connectionStore.isConnectionActive(connectionId)) {
				throw new Error('Connection is not active');
			}

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
				notificationsStore.notify({
					severity: 'warning',
					title: 'Project Loaded With Warnings',
					message: `Connected, but failed to load the initial file tree for ${profile.host}:${projectRoot}.`,
					detail: error instanceof Error ? error.message : String(error)
				});
			}

			// Generate display name
			const folderName = projectRoot.split('/').pop() || projectRoot;
			const displayName = `${profile.host}:${folderName}`;

			const session: Session = {
				id: sessionId,
				connectionId,
				connectionProfile: profile,
				connectionStatus: 'connected',
				connectionDetail: null,
				projectRoot,
				displayName,
				fileState: {
					...createInitialFileState(),
					tree,
					expandedPaths: new Set([projectRoot])
				},
				terminalIds: [],
				terminalOrdinals: {},
				layoutState: createInitialLayoutState()
			};

			// Update workspace state
			update((s) => ({
				...s,
				sessions: new Map(s.sessions).set(sessionId, session),
				sessionOrder: [...s.sessionOrder, sessionId],
				activeSessionId: sessionId
			}));

			// Keep connection sessionCount in sync for UI and lifecycle management
			connectionStore.updateSessionCount(connectionId, +1);

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

			// Close all terminals belonging to this session (single owner: terminalStore)
			try {
				const { terminalStore } = await import('./terminal');
				await terminalStore.closeSessionTerminals(sessionId);
			} catch (error) {
				console.error('Failed to close session terminals:', error);
				notificationsStore.notify({
					severity: 'warning',
					title: 'Terminal Cleanup Failed',
					message: 'Some terminals may not have closed cleanly.',
					detail: error instanceof Error ? error.message : String(error)
				});
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
			connectionStore.updateSessionCount(connectionId, -1);
			if (!this.hasSessionsOnConnection(connectionId)) {
				await connectionStore.disconnectById(connectionId);
			}
		},

		/**
		 * Drop all sessions that depend on a connection (used when the backend reports a disconnect).
		 * This is intentionally a local-state operation; it does not attempt to disconnect or clean up
		 * remote resources because the connection is already gone.
		 */
		async dropSessionsForConnection(connectionId: string, reason?: string): Promise<void> {
			const state = get({ subscribe });
			const sessionsToDrop = Array.from(state.sessions.values()).filter(
				(s) => s.connectionId === connectionId
			);
			if (sessionsToDrop.length === 0) return;

			// Best-effort: remove from connection store counts without triggering disconnect flows.
			connectionStore.updateSessionCount(connectionId, -sessionsToDrop.length);

			update((s) => {
				const nextSessions = new Map(s.sessions);
				for (const session of sessionsToDrop) {
					nextSessions.delete(session.id);
				}

				const nextOrder = s.sessionOrder.filter(
					(id) => !sessionsToDrop.some((session) => session.id === id)
				);

				let nextActiveId = s.activeSessionId;
				if (nextActiveId && !nextSessions.has(nextActiveId)) {
					nextActiveId = nextOrder[0] || null;
				}

				return {
					...s,
					sessions: nextSessions,
					sessionOrder: nextOrder,
					activeSessionId: nextActiveId
				};
			});

			if (reason) {
				notificationsStore.notifyOnce(`sessions_dropped:${connectionId}`, {
					severity: 'warning',
					title: 'Workspace Closed',
					message: 'The workspace was closed because the SSH connection was lost.',
					detail: reason
				});
			}
		},

		/**
		 * Mark all sessions for a connection as disconnected (preserves session UI state for recovery).
		 */
		markConnectionDisconnected(connectionId: string, detail?: string | null): void {
			update((s) => {
				let changed = false;
				const nextSessions = new Map(s.sessions);
				for (const [id, session] of nextSessions) {
					if (session.connectionId !== connectionId) continue;
					if (session.connectionStatus === 'disconnected' && session.connectionDetail === (detail ?? null)) {
						continue;
					}
					nextSessions.set(id, {
						...session,
						connectionStatus: 'disconnected',
						connectionDetail: detail ?? null
					});
					changed = true;
				}
				return changed ? { ...s, sessions: nextSessions } : s;
			});
		},

		/**
		 * Mark all sessions for a connection as connected again.
		 */
		markConnectionConnected(connectionId: string): void {
			update((s) => {
				let changed = false;
				const nextSessions = new Map(s.sessions);
				for (const [id, session] of nextSessions) {
					if (session.connectionId !== connectionId) continue;
					if (session.connectionStatus === 'connected' && !session.connectionDetail) continue;
					nextSessions.set(id, { ...session, connectionStatus: 'connected', connectionDetail: null });
					changed = true;
				}
				return changed ? { ...s, sessions: nextSessions } : s;
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
					terminalIds: [...session.terminalIds, terminalId],
					terminalOrdinals: {
						...(session.terminalOrdinals ?? {}),
						[terminalId]: (session.terminalOrdinals ?? {})[terminalId] ?? nextTerminalOrdinal(session)
					}
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

				const nextOrdinals = { ...(session.terminalOrdinals ?? {}) };
				delete nextOrdinals[terminalId];

				const newSessions = new Map(s.sessions);
				newSessions.set(sessionId, {
					...session,
					terminalIds: session.terminalIds.filter((id) => id !== terminalId),
					terminalOrdinals: nextOrdinals
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
			initialized = false;
			if (saveTimer !== null && typeof window !== 'undefined') {
				window.clearTimeout(saveTimer);
				saveTimer = null;
			}
			set(initialState);
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
