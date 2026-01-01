import { writable, derived, get } from 'svelte/store';
import type { TerminalSession } from '$types';
import { invoke } from '$utils/tauri';
import { workspaceStore, activeSession } from './workspace';
import { layoutStore } from './layout';

interface TerminalState {
	// Global registry of all terminals across all sessions
	allTerminals: Map<string, TerminalSession>;
}

const initialState: TerminalState = {
	allTerminals: new Map()
};

function createTerminalStore() {
	const { subscribe, set, update } = writable<TerminalState>(initialState);

	return {
		subscribe,

		/**
		 * Create a new terminal for the active session
		 */
		async createTerminal(): Promise<string> {
			const session = get(activeSession);
			if (!session) {
				throw new Error('No active session');
			}
			const sessionId = session.id;

			const terminalId = await invoke<string>('terminal_create', {
				connId: session.connectionId,
				workingDir: session.projectRoot
			});

			// Count existing terminals for this session for naming
			const state = get({ subscribe });
			const sessionTerminalCount = Array.from(state.allTerminals.values()).filter(
				(t) => t.sessionId === session.id
			).length;

			const terminalSession: TerminalSession = {
				id: terminalId,
				title: `Terminal ${sessionTerminalCount + 1}`,
				sessionId
			};

			// Add to global registry
			update((s) => {
				const newTerminals = new Map(s.allTerminals);
				newTerminals.set(terminalId, terminalSession);
				return { ...s, allTerminals: newTerminals };
			});

			// Register with workspace session
			workspaceStore.addTerminalToSession(sessionId, terminalId);

			// Add panel to the session's layout
			layoutStore.addPanelForSession(sessionId, {
				type: 'terminal',
				title: terminalSession.title,
				terminalId
			});

			return terminalId;
		},

		/**
		 * Close a terminal
		 */
		async closeTerminal(terminalId: string): Promise<void> {
			const state = get({ subscribe });
			const terminal = state.allTerminals.get(terminalId);

			try {
				await invoke('terminal_close', { termId: terminalId });
			} catch (error) {
				console.error('Failed to close terminal:', error);
			}

			// Remove from global registry
			update((s) => {
				const newTerminals = new Map(s.allTerminals);
				newTerminals.delete(terminalId);
				return { ...s, allTerminals: newTerminals };
			});

			// Remove from workspace session if still exists
			if (terminal) {
				workspaceStore.removeTerminalFromSession(terminal.sessionId, terminalId);
			}
		},

		/**
		 * Rename a terminal
		 */
		renameTerminal(terminalId: string, title: string): void {
			update((s) => {
				const terminal = s.allTerminals.get(terminalId);
				if (!terminal) return s;

				const newTerminals = new Map(s.allTerminals);
				newTerminals.set(terminalId, { ...terminal, title });
				return { ...s, allTerminals: newTerminals };
			});

			// Update panel title
			const terminal = get({ subscribe }).allTerminals.get(terminalId);
			if (!terminal) return;

			const panel = layoutStore.findPanelByTerminalId(terminalId, terminal.sessionId);
			if (!panel) return;

			layoutStore.updatePanelTitleForSession(terminal.sessionId, panel.id, title);
		},

		/**
		 * Get a terminal by ID
		 */
		getTerminal(terminalId: string): TerminalSession | undefined {
			return get({ subscribe }).allTerminals.get(terminalId);
		},

		/**
		 * Get all terminals for a session
		 */
		getSessionTerminals(sessionId: string): TerminalSession[] {
			const state = get({ subscribe });
			return Array.from(state.allTerminals.values()).filter((t) => t.sessionId === sessionId);
		},

		/**
		 * Close all terminals for a session (called when session is closed)
		 */
		async closeSessionTerminals(sessionId: string): Promise<void> {
			const terminals = this.getSessionTerminals(sessionId);
			for (const terminal of terminals) {
				await this.closeTerminal(terminal.id);
			}
		},

		/**
		 * Prune terminals whose owning session no longer exists.
		 * (Session close currently happens in workspaceStore; this keeps the terminal registry consistent.)
		 */
		reconcileSessions(validSessionIds: Set<string>): void {
			update((s) => {
				if (s.allTerminals.size === 0) return s;

				let changed = false;
				const next = new Map(s.allTerminals);
				for (const [terminalId, terminal] of next) {
					if (!validSessionIds.has(terminal.sessionId)) {
						next.delete(terminalId);
						changed = true;
					}
				}

				return changed ? { ...s, allTerminals: next } : s;
			});
		},

		reset(): void {
			set(initialState);
		}
	};
}

export const terminalStore = createTerminalStore();

// Derived store for active session's terminals
export const activeSessionTerminals = derived(
	[terminalStore, activeSession],
	([$terminals, $session]) => {
		if (!$session) return [];
		return Array.from($terminals.allTerminals.values()).filter(
			(t) => t.sessionId === $session.id
		);
	}
);

// Derived store for all terminals as array
export const allTerminals = derived(terminalStore, ($store) =>
	Array.from($store.allTerminals.values())
);

// Keep the terminal registry consistent when sessions are closed/removed.
workspaceStore.subscribe(($ws) => {
	terminalStore.reconcileSessions(new Set($ws.sessions.keys()));
});
