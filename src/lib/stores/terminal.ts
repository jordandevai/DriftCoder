import { writable, derived, get } from 'svelte/store';
import type { TerminalSession } from '$types';
import { invoke } from '$utils/tauri';
import { workspaceStore, activeSession } from './workspace';
import { layoutStore } from './layout';
import { settingsStore } from './settings';
import { notificationsStore } from './notifications';

function sanitizeTmuxToken(value: string): string {
	return value
		.trim()
		.replace(/[^a-zA-Z0-9._-]+/g, '_')
		.replace(/^_+|_+$/g, '')
		.slice(0, 40);
}

function fnv1aHash36(input: string): string {
	let hash = 0x811c9dc5;
	for (let i = 0; i < input.length; i += 1) {
		hash ^= input.charCodeAt(i);
		hash = Math.imul(hash, 0x01000193);
	}
	return (hash >>> 0).toString(36);
}

function projectSlugFromRoot(projectRoot: string): string {
	const base = projectRoot.split('/').filter(Boolean).pop() || 'project';
	return sanitizeTmuxToken(base.toLowerCase()) || 'project';
}

function computeNextTerminalOrdinal(existing: Record<string, number> | undefined): number {
	const ordinals = existing ?? {};
	const max = Object.values(ordinals).reduce((m, n) => (Number.isFinite(n) ? Math.max(m, n) : m), 0);
	return max + 1;
}

function buildStartupCommandForTerminal(sessionId: string, terminalId: string): string | null {
	const settings = get(settingsStore);
	if (settings.terminalSessionPersistence !== 'tmux') return null;
	const prefix = sanitizeTmuxToken(settings.terminalTmuxSessionPrefix || 'driftcoder') || 'driftcoder';
	const clientSuffix = sanitizeTmuxToken(settings.clientInstanceId || '').slice(0, 6) || 'client';
	const ws = get(workspaceStore);
	const session = ws.sessions.get(sessionId);
	const projectRoot = session?.projectRoot || '';
	const profile = session?.connectionProfile;
	const identity = profile
		? `${profile.username}@${profile.host}:${profile.port}|${projectRoot}`
		: `${projectRoot}`;
	const suffix = fnv1aHash36(identity).slice(0, 6) || '000000';
	const projectSlug = projectSlugFromRoot(projectRoot);
	// Include a stable per-install suffix to avoid multiple devices attaching to the same tmux session by default.
	const tmuxSession = `${prefix}-${projectSlug}-${suffix}-${clientSuffix}`;
	const ordinal =
		session?.terminalOrdinals?.[terminalId] ?? computeNextTerminalOrdinal(session?.terminalOrdinals);
	const window = `term${ordinal}`;
	const legacyWindow = `t-${sanitizeTmuxToken(terminalId.slice(0, 8)) || 'term'}`;

	// One tmux session per project + one tmux window per DriftCoder terminal tab.
	// Guard against nesting: if the user is already inside tmux ($TMUX set), do nothing.
	return (
		`if [ -z "$TMUX" ] && command -v tmux >/dev/null 2>&1; then ` +
		`session="${tmuxSession}"; window="${window}"; legacy="${legacyWindow}"; ` +
		`if ! tmux has-session -t "$session" 2>/dev/null; then ` +
		`tmux new-session -d -s "$session" -n "$window" -c "$PWD"; ` +
		`else ` +
		`if ! tmux list-windows -t "$session" -F "#{window_name}" 2>/dev/null | grep -Fxq "$window"; then ` +
		`if tmux list-windows -t "$session" -F "#{window_name}" 2>/dev/null | grep -Fxq "$legacy"; then ` +
		`tmux rename-window -t "$session:$legacy" "$window" 2>/dev/null || true; ` +
		`fi; ` +
		`tmux list-windows -t "$session" -F "#{window_name}" 2>/dev/null | grep -Fxq "$window" || tmux new-window -t "$session" -n "$window" -c "$PWD"; ` +
		`fi; ` +
		`fi; ` +
		`tmux attach -t "$session:$window"; ` +
		`fi`
	);
}

const tmuxAvailabilityCache = new Map<string, boolean>();

async function ensureTmuxAvailable(connectionId: string): Promise<boolean> {
	const settings = get(settingsStore);
	if (settings.terminalSessionPersistence !== 'tmux') return true;
	if (!connectionId) return false;
	if (tmuxAvailabilityCache.has(connectionId)) return tmuxAvailabilityCache.get(connectionId)!;

	try {
		const ok = await invoke<boolean>('ssh_check_tmux', { connId: connectionId });
		tmuxAvailabilityCache.set(connectionId, ok);
		return ok;
	} catch {
		// If the check fails (e.g. servers that restrict exec), don't block terminal creation.
		return true;
	}
}

function warnTmuxMissingOnce(connectionId: string, sessionId: string): void {
	if (!connectionId) return;
	const settings = get(settingsStore);
	if (settings.terminalSessionPersistence !== 'tmux') return;

	const ws = get(workspaceStore);
	const session = ws.sessions.get(sessionId);
	const profile = session?.connectionProfile;

	notificationsStore.notifyOnce(`tmux_missing:${connectionId}`, {
		severity: 'warning',
		title: 'tmux Not Found',
		message: profile
			? `tmux persistence is enabled, but tmux is not installed on ${profile.host}.`
			: 'tmux persistence is enabled, but tmux is not installed on the server.',
		detail:
			'Install tmux on the server to keep terminals alive across disconnects/backgrounding.\n\n' +
			'Ubuntu/Debian: sudo apt-get install tmux\n' +
			'Fedora: sudo dnf install tmux\n' +
			'Arch: sudo pacman -S tmux\n' +
			'macOS: brew install tmux'
	});
}

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
		 * Hydrate the terminal registry from the persisted workspace session state.
		 * This is required so reconnect can reopen terminals after an app restart.
		 */
		hydrateFromWorkspace(): void {
			const ws = get(workspaceStore);
			const next = new Map<string, TerminalSession>();

			for (const session of ws.sessions.values()) {
				const titleByTerminalId = new Map<string, string>();
				for (const group of session.layoutState.groups.values()) {
					for (const panel of group.panels) {
						if (panel.type !== 'terminal' || !panel.terminalId) continue;
						titleByTerminalId.set(panel.terminalId, panel.title || 'Terminal');
					}
				}

				for (const terminalId of session.terminalIds) {
					next.set(terminalId, {
						id: terminalId,
						title: titleByTerminalId.get(terminalId) ?? 'Terminal',
						sessionId: session.id
					});
				}
			}

			update((s) => {
				if (next.size === 0 && s.allTerminals.size === 0) return s;
				return { ...s, allTerminals: next };
			});
		},

		/**
		 * Create a new terminal for the active session
		 */
		async createTerminal(): Promise<string> {
			const session = get(activeSession);
			if (!session) {
				throw new Error('No active session');
			}
			const sessionId = session.id;

			const requestedTerminalId = crypto.randomUUID();
			// Reserve ordinal up-front to avoid tmux window collisions when multiple terminals are created rapidly.
			workspaceStore.reserveTerminalOrdinal(sessionId, requestedTerminalId);
			const tmuxOk = await ensureTmuxAvailable(session.connectionId);
			if (!tmuxOk) warnTmuxMissingOnce(session.connectionId, sessionId);

			let terminalId: string;
			try {
				terminalId = await invoke<string>('terminal_create', {
					connId: session.connectionId,
					workingDir: session.projectRoot,
					termId: requestedTerminalId,
					startupCommand: tmuxOk ? buildStartupCommandForTerminal(sessionId, requestedTerminalId) : null
				});
			} catch (e) {
				workspaceStore.releaseTerminalOrdinal(sessionId, requestedTerminalId);
				throw e;
			}

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

			// Register with workspace session (persists across app restart).
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
		 * Re-open all terminals for a given SSH connection.
		 * Used after auto-reconnect so terminal tabs keep their scrollback and IDs.
		 */
		async reopenTerminalsForConnection(connectionId: string): Promise<void> {
			const ws = get(workspaceStore);
			const tmuxOk = await ensureTmuxAvailable(connectionId);
			const terminals = Array.from(get({ subscribe }).allTerminals.values());
			for (const terminal of terminals) {
				const session = ws.sessions.get(terminal.sessionId);
				if (!session) continue;
				if (session.connectionId !== connectionId) continue;
				if (!tmuxOk) warnTmuxMissingOnce(connectionId, session.id);
				try {
					await invoke('terminal_reopen', {
						connId: connectionId,
						termId: terminal.id,
						workingDir: session.projectRoot,
						startupCommand: tmuxOk ? buildStartupCommandForTerminal(session.id, terminal.id) : null
					});
				} catch (error) {
					// Best-effort: a single failed terminal should not block reconnect for the workspace.
					console.error(`Failed to reopen terminal ${terminal.id}:`, error);
				}
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
			tmuxAvailabilityCache.clear();
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
