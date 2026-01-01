import { derived, get } from 'svelte/store';
import type { LayoutState, PanelGroup, Panel } from '$types';
import { workspaceStore, activeSession } from './workspace';

function generateId(): string {
	return crypto.randomUUID();
}

// Empty state for when no session is active
const emptyState: LayoutState = {
	root: { type: 'leaf', groupId: 'main' },
	groups: new Map(),
	activeGroupId: null,
	fileTreeWidth: 250,
	fileTreeCollapsed: false
};

// Get the active session ID
function getActiveSessionId(): string {
	const ws = get(workspaceStore);
	if (!ws.activeSessionId) {
		throw new Error('No active session');
	}
	return ws.activeSessionId;
}

// Update the active session's layout state
function updateLayoutState(
	sessionId: string,
	updater: (state: LayoutState) => LayoutState
): void {
	workspaceStore.updateSessionLayoutState(sessionId, updater);
}

// Derived store that reflects the active session's layout state
const layoutStateStore = derived(activeSession, ($session) => {
	if (!$session) return emptyState;
	return $session.layoutState as LayoutState;
});

// Create the layout store with methods
function createLayoutStore() {
	return {
		// Subscribe to the derived state
		subscribe: layoutStateStore.subscribe,

		addPanelForSession(
			sessionId: string,
			panel: Omit<Panel, 'id'>,
			groupId?: string
		): string {
			const id = generateId();
			const fullPanel: Panel = { ...panel, id };

			updateLayoutState(sessionId, (s) => {
				const targetGroupId = groupId || s.activeGroupId || 'main';
				const group = s.groups.get(targetGroupId);

				if (!group) return s;

				const newGroups = new Map(s.groups);
				newGroups.set(targetGroupId, {
					...group,
					panels: [...group.panels, fullPanel],
					activePanelId: id
				});

				return { ...s, groups: newGroups, activeGroupId: targetGroupId };
			});

			return id;
		},

		addPanel(panel: Omit<Panel, 'id'>, groupId?: string): string {
			return this.addPanelForSession(getActiveSessionId(), panel, groupId);
		},

		removePanelForSession(sessionId: string, panelId: string): void {
			updateLayoutState(sessionId, (s) => {
				const newGroups = new Map(s.groups);

				for (const [groupId, group] of newGroups) {
					const panelIndex = group.panels.findIndex((p) => p.id === panelId);
					if (panelIndex === -1) continue;

					const newPanels = group.panels.filter((p) => p.id !== panelId);
					let newActivePanelId = group.activePanelId;

					if (group.activePanelId === panelId) {
						// Activate adjacent panel
						if (newPanels.length > 0) {
							const newIndex = Math.min(panelIndex, newPanels.length - 1);
							newActivePanelId = newPanels[newIndex].id;
						} else {
							newActivePanelId = null;
						}
					}

					newGroups.set(groupId, {
						...group,
						panels: newPanels,
						activePanelId: newActivePanelId
					});

					break;
				}

				return { ...s, groups: newGroups };
			});
		},

		removePanel(panelId: string): void {
			this.removePanelForSession(getActiveSessionId(), panelId);
		},

		setActivePanelForSession(sessionId: string, panelId: string): void {
			updateLayoutState(sessionId, (s) => {
				const newGroups = new Map(s.groups);

				for (const [groupId, group] of newGroups) {
					if (group.panels.some((p) => p.id === panelId)) {
						newGroups.set(groupId, { ...group, activePanelId: panelId });
						return { ...s, groups: newGroups, activeGroupId: groupId };
					}
				}

				return s;
			});
		},

		setActivePanel(panelId: string): void {
			this.setActivePanelForSession(getActiveSessionId(), panelId);
		},

		setActiveGroup(groupId: string): void {
			updateLayoutState(getActiveSessionId(), (s) => ({ ...s, activeGroupId: groupId }));
		},

		updatePanelTitleForSession(sessionId: string, panelId: string, title: string): void {
			updateLayoutState(sessionId, (s) => {
				const newGroups = new Map(s.groups);

				for (const [groupId, group] of newGroups) {
					const panel = group.panels.find((p) => p.id === panelId);
					if (panel) {
						const newPanels = group.panels.map((p) => (p.id === panelId ? { ...p, title } : p));
						newGroups.set(groupId, { ...group, panels: newPanels });
						break;
					}
				}

				return { ...s, groups: newGroups };
			});
		},

		updatePanelTitle(panelId: string, title: string): void {
			this.updatePanelTitleForSession(getActiveSessionId(), panelId, title);
		},

		findPanelByFilePath(filePath: string, sessionId?: string): Panel | undefined {
			const ws = get(workspaceStore);
			const resolvedSessionId = sessionId ?? ws.activeSessionId ?? null;
			if (!resolvedSessionId) return undefined;

			const session = ws.sessions.get(resolvedSessionId);
			if (!session) return undefined;

			for (const group of session.layoutState.groups.values()) {
				const panel = group.panels.find((p) => p.filePath === filePath);
				if (panel) return panel;
			}
			return undefined;
		},

		findPanelByTerminalId(terminalId: string, sessionId?: string): Panel | undefined {
			const ws = get(workspaceStore);
			const resolvedSessionId = sessionId ?? ws.activeSessionId ?? null;
			if (!resolvedSessionId) return undefined;

			const session = ws.sessions.get(resolvedSessionId);
			if (!session) return undefined;

			for (const group of session.layoutState.groups.values()) {
				const panel = group.panels.find((p) => p.terminalId === terminalId);
				if (panel) return panel;
			}
			return undefined;
		},

		setFileTreeWidth(width: number): void {
			updateLayoutState(getActiveSessionId(), (s) => ({
				...s,
				fileTreeWidth: Math.max(150, Math.min(500, width))
			}));
		},

		toggleFileTree(): void {
			updateLayoutState(getActiveSessionId(), (s) => ({ ...s, fileTreeCollapsed: !s.fileTreeCollapsed }));
		},

		reset(): void {
			// Reset is handled by workspace session closing
		}
	};
}

export const layoutStore = createLayoutStore();

// Derived stores
export const activeGroup = derived(layoutStore, ($store) =>
	$store.activeGroupId ? $store.groups.get($store.activeGroupId) : null
);

export const activePanel = derived(layoutStore, ($store) => {
	if (!$store.activeGroupId) return null;
	const group = $store.groups.get($store.activeGroupId);
	if (!group || !group.activePanelId) return null;
	return group.panels.find((p) => p.id === group.activePanelId);
});

export const allPanels = derived(layoutStore, ($store) => {
	const panels: Panel[] = [];
	for (const group of $store.groups.values()) {
		panels.push(...group.panels);
	}
	return panels;
});
