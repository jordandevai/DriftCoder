import { load, type Store } from '@tauri-apps/plugin-store';
import type { ConnectionProfile, SettingsState, WorkspaceState, Session, SessionFileState, SessionLayoutState, PanelGroup, LayoutNode, OpenFile } from '$types';

let store: Store | null = null;

async function getStore(): Promise<Store> {
	if (!store) {
		store = await load('settings.json');
	}
	return store;
}

export async function loadSavedConnections(): Promise<ConnectionProfile[]> {
	try {
		const s = await getStore();
		const profiles = await s.get<ConnectionProfile[]>('connections');
		return profiles || [];
	} catch (error) {
		console.error('Failed to load saved connections:', error);
		return [];
	}
}

export async function saveConnections(profiles: ConnectionProfile[]): Promise<void> {
	try {
		const s = await getStore();
		await s.set('connections', profiles);
		await s.save();
	} catch (error) {
		console.error('Failed to save connections:', error);
	}
}

export async function loadSavedSettings(): Promise<Partial<SettingsState> | null> {
	try {
		const s = await getStore();
		const settings = await s.get<Partial<SettingsState>>('settings');
		return settings || null;
	} catch (error) {
		console.error('Failed to load settings:', error);
		return null;
	}
}

export async function saveSettings(settings: SettingsState): Promise<void> {
	try {
		const s = await getStore();
		await s.set('settings', settings);
		await s.save();
	} catch (error) {
		console.error('Failed to save settings:', error);
	}
}

type PersistedWorkspace = {
	activeSessionId: string | null;
	sessionOrder: string[];
	sessions: PersistedSession[];
};

type PersistedSession = Omit<Session, 'fileState' | 'layoutState' | 'terminalIds'> & {
	fileState: PersistedFileState;
	layoutState: PersistedLayoutState;
	terminalIds: string[];
};

type PersistedFileState = Omit<SessionFileState, 'tree' | 'expandedPaths' | 'openFiles'> & {
	tree: []; // don't persist file tree
	expandedPaths: string[];
	openFiles: PersistedOpenFile[];
};

type PersistedOpenFile = OpenFile;

type PersistedLayoutState = Omit<SessionLayoutState, 'groups'> & {
	groups: Array<[string, PanelGroup]>;
};

function serializeWorkspace(state: WorkspaceState): PersistedWorkspace {
	const sessions: PersistedSession[] = [];
	for (const session of state.sessions.values()) {
		const fileState: PersistedFileState = {
			...session.fileState,
			tree: [],
			expandedPaths: Array.from(session.fileState.expandedPaths.values()),
			openFiles: Array.from(session.fileState.openFiles.values())
		};

		const layoutState: PersistedLayoutState = {
			...session.layoutState,
			groups: Array.from(session.layoutState.groups.entries())
		};

		sessions.push({
			...session,
			fileState,
			layoutState,
			terminalIds: session.terminalIds
		});
	}

	return {
		activeSessionId: state.activeSessionId,
		sessionOrder: state.sessionOrder,
		sessions
	};
}

function deserializeWorkspace(persisted: PersistedWorkspace): WorkspaceState {
	const sessions = new Map<string, Session>();

	for (const session of persisted.sessions) {
		const fileState: SessionFileState = {
			...session.fileState,
			tree: [],
			expandedPaths: new Set(session.fileState.expandedPaths),
			openFiles: new Map(session.fileState.openFiles.map((f) => [f.path, f]))
		};

		const layoutState: SessionLayoutState = {
			...session.layoutState,
			root: session.layoutState.root as LayoutNode,
			groups: new Map(session.layoutState.groups)
		};

		sessions.set(session.id, {
			...session,
			fileState,
			layoutState,
			terminalOrdinals:
				session.terminalOrdinals ??
				Object.fromEntries((session.terminalIds ?? []).map((id, idx) => [id, idx + 1]))
		});
	}

	return {
		sessions,
		activeSessionId: persisted.activeSessionId,
		sessionOrder: persisted.sessionOrder
	};
}

export async function loadSavedWorkspace(): Promise<WorkspaceState | null> {
	try {
		const s = await getStore();
		const raw = await s.get<PersistedWorkspace>('workspace');
		if (!raw) return null;
		return deserializeWorkspace(raw);
	} catch (error) {
		console.error('Failed to load workspace:', error);
		return null;
	}
}

export async function saveWorkspace(state: WorkspaceState): Promise<void> {
	try {
		const s = await getStore();
		await s.set('workspace', serializeWorkspace(state));
		await s.save();
	} catch (error) {
		console.error('Failed to save workspace:', error);
	}
}
