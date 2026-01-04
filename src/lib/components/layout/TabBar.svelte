<script lang="ts">
	import type { Panel } from '$types';
	import { fileStore } from '$stores/files';

	interface Props {
		panels: Panel[];
		activePanelId: string | null;
		onselect: (panelId: string) => void;
		onclose: (panelId: string) => void;
	}

	let { panels, activePanelId, onselect, onclose }: Props = $props();

	function getIcon(panel: Panel): string {
		if (panel.type === 'terminal') return 'terminal';
		// Get file extension for icon
		if (panel.filePath) {
			const ext = panel.filePath.split('.').pop()?.toLowerCase();
			return ext || 'file';
		}
		return 'file';
	}

	function isDirty(panel: Panel): boolean {
		if (panel.type === 'editor' && panel.filePath) {
			const file = $fileStore.openFiles.get(panel.filePath);
			return file?.dirty || false;
		}
		return false;
	}

	function handleMiddleClick(e: MouseEvent, panelId: string) {
		if (e.button === 1) {
			e.preventDefault();
			onclose(panelId);
		}
	}
</script>

<div class="flex items-center bg-tab-bg border-b border-panel-border overflow-x-auto">
	{#each panels as panel (panel.id)}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="group flex items-center gap-2 px-3 py-1.5 border-r border-panel-border cursor-pointer transition-colors {panel.id ===
			activePanelId
				? 'bg-editor-bg border-b-2 border-b-accent'
				: 'hover:bg-panel-active'}"
			onclick={() => onselect(panel.id)}
			onmousedown={(e) => handleMiddleClick(e, panel.id)}
		>
			<!-- Icon -->
			{#if panel.type === 'terminal'}
				<svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
					/>
				</svg>
			{:else}
				<svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
					/>
				</svg>
			{/if}

			<!-- Title -->
			<span class="text-sm truncate max-w-32 {isDirty(panel) ? 'italic' : ''}">
				{#if isDirty(panel)}
					<span class="text-accent mr-1">●</span>
				{/if}
				{panel.title}
			</span>

			<!-- Close button -->
			<button
				class="flex items-center justify-center rounded transition-all
				       p-0.5 opacity-0 group-hover:opacity-100 hover:bg-panel-border
				       touch-device:w-11 touch-device:h-11 touch-device:opacity-60 touch-device:-mr-2"
				onclick={(e) => {
					e.stopPropagation();
					onclose(panel.id);
				}}
				aria-label="Close tab"
			>
				<svg class="w-3.5 h-3.5 touch-device:w-4 touch-device:h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M6 18L18 6M6 6l12 12"
					/>
				</svg>
			</button>
		</div>
	{/each}

	<!-- Spacer -->
	<div class="flex-1"></div>
</div>
