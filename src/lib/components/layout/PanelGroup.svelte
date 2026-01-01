<script lang="ts">
	import { layoutStore } from '$stores/layout';
	import { workspaceStore, activeSession } from '$stores/workspace';
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

	// Collect ALL panels from ALL sessions for persistence
	interface SessionPanel extends Panel {
		sessionId: string;
	}

	const allPanels = $derived.by(() => {
		const panels: SessionPanel[] = [];
		const ws = $workspaceStore;

		for (const [sessionId, session] of ws.sessions) {
			const sessionGroup = session.layoutState.groups.get(groupId);
			if (sessionGroup) {
				for (const panel of sessionGroup.panels) {
					panels.push({ ...panel, sessionId });
				}
			}
		}
		return panels;
	});

	// Check if a panel should be visible
	function isPanelVisible(panel: SessionPanel): boolean {
		return panel.sessionId === currentSessionId && panel.id === activePanelId;
	}

	function handleTabClose(panelId: string) {
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

		<!-- Panel Content - render ALL panels from ALL sessions, show only active one -->
		<div class="flex-1 overflow-hidden relative">
			{#each allPanels as panel (`${panel.sessionId}-${panel.id}`)}
				<div
					class="absolute inset-0 {isPanelVisible(panel) ? '' : 'invisible pointer-events-none'}"
				>
					{#if panel.type === 'editor'}
						<EditorPanel filePath={panel.filePath || ''} />
					{:else if panel.type === 'terminal'}
						<TerminalPanel terminalId={panel.terminalId || ''} />
					{/if}
				</div>
			{/each}
		</div>
	{:else}
		<!-- No panels state -->
		<div class="h-full flex items-center justify-center text-gray-500">
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
</div>
