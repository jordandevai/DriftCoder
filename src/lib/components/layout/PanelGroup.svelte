<script lang="ts">
	import { layoutStore } from '$stores/layout';
	import { workspaceStore, activeSession } from '$stores/workspace';
	import { fileStore } from '$stores/files';
	import { terminalStore } from '$stores/terminal';
	import { notificationsStore } from '$stores/notifications';
	import { connectionStore } from '$stores/connection';
	import TabBar from './TabBar.svelte';
	import EditorPanel from '$components/panels/EditorPanel.svelte';
	import TerminalPanel from '$components/panels/TerminalPanel.svelte';
	import type { Panel } from '$types';

	interface Props {
		groupId: string;
	}

	let { groupId }: Props = $props();

	// Current session's group (for tab bar)
	const group = $derived($layoutStore.groups.get(groupId));
	const activePanelId = $derived(group?.activePanelId);
	const currentSessionId = $derived($activeSession?.id);

	// Collect ALL terminal panels from ALL sessions for persistence
	interface SessionPanel extends Panel {
		sessionId: string;
	}

	const allTerminalPanels = $derived.by(() => {
		const panels: SessionPanel[] = [];
		const ws = $workspaceStore;

		for (const [sessionId, session] of ws.sessions) {
			const sessionGroup = session.layoutState.groups.get(groupId);
			if (sessionGroup) {
				for (const panel of sessionGroup.panels) {
					if (panel.type === 'terminal') {
						panels.push({ ...panel, sessionId });
					}
				}
			}
		}
		return panels;
	});

	// Keep active file path in sync with the active panel
	$effect(() => {
		if (!group) return;

		const activePanel = group.activePanelId
			? group.panels.find((p) => p.id === group.activePanelId) || null
			: null;

		if (!activePanel || activePanel.type !== 'editor' || !activePanel.filePath) {
			fileStore.setActiveFile(null);
			return;
		}

		fileStore.setActiveFile(activePanel.filePath);
	});

	function isActiveEditorPanel(panel: Panel): boolean {
		return panel.id === activePanelId;
	}

	function isVisibleTerminalPanel(panel: SessionPanel): boolean {
		return panel.sessionId === currentSessionId && panel.id === activePanelId;
	}

	function getTerminalConnection(panel: SessionPanel): {
		connectionId: string;
		status: 'connected' | 'reconnecting' | 'disconnected';
	} {
		const session = $workspaceStore.sessions.get(panel.sessionId);
		const connectionId = session?.connectionId ?? '';
		const conn = connectionId ? $connectionStore.activeConnections.get(connectionId) : undefined;
		const status = (conn?.status ??
			(session?.connectionStatus === 'disconnected' ? 'disconnected' : 'connected')) as
			| 'connected'
			| 'reconnecting'
			| 'disconnected';
		return { connectionId, status };
	}

	function handleTabClose(panelId: string) {
		if (!group) return;

		const panel = group.panels.find((p) => p.id === panelId);
		if (panel?.type === 'editor' && panel.filePath) {
			fileStore.closeFile(panel.filePath);
		}
		if (panel?.type === 'terminal' && panel.terminalId) {
			terminalStore.closeTerminal(panel.terminalId).catch((error) => {
				console.error('Failed to close terminal:', error);
				notificationsStore.notify({
					severity: 'error',
					title: 'Terminal Close Failed',
					message: 'Could not close the terminal.',
					detail: error instanceof Error ? error.message : String(error)
				});
			});
		}

		layoutStore.removePanel(panelId);
	}

	function handleTabSelect(panelId: string) {
		layoutStore.setActivePanel(panelId);
	}
</script>

<div class="h-full flex flex-col">
	{#if group && group.panels.length > 0}
		<!-- Tab Bar - only shows current session's tabs -->
		<TabBar
			panels={group.panels}
			activePanelId={group.activePanelId}
			onselect={handleTabSelect}
			onclose={handleTabClose}
		/>
	{/if}

	<!-- Panel Content -->
	<div class="flex-1 overflow-hidden relative">
		{#if group && group.panels.length > 0}
			<!-- Editor panels: only the active session (editors are session-scoped via fileStore) -->
			{#each group.panels.filter((p) => p.type === 'editor') as panel (panel.id)}
				<div class="absolute inset-0 {isActiveEditorPanel(panel) ? '' : 'invisible pointer-events-none'}">
					<EditorPanel filePath={panel.filePath || ''} />
				</div>
			{/each}
		{:else}
			<!-- No panels state (keep terminals mounted below for cross-session persistence) -->
			<div class="absolute inset-0 flex items-center justify-center text-gray-500">
				<div class="text-center">
					<svg class="w-16 h-16 mx-auto mb-4 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="1"
							d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
						/>
					</svg>
					<p class="text-lg mb-1">No files open</p>
					<p class="text-sm">Open a file from the file tree to start editing</p>
				</div>
			</div>
		{/if}

		<!-- Terminal panels: keep ALL sessions mounted for persistence -->
		{#each allTerminalPanels as panel (`${panel.sessionId}-${panel.id}`)}
			{@const conn = getTerminalConnection(panel)}
			<div class="absolute inset-0 {isVisibleTerminalPanel(panel) ? '' : 'invisible pointer-events-none'}">
				<TerminalPanel
					terminalId={panel.terminalId || ''}
					active={isVisibleTerminalPanel(panel)}
					connectionId={conn.connectionId}
					connectionStatus={conn.status}
				/>
			</div>
		{/each}
	</div>
</div>
