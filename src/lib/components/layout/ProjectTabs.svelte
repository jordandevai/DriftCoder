<script lang="ts">
	import { workspaceStore, orderedSessions, activeSession } from '$stores/workspace';
	import { confirmStore } from '$stores/confirm';
	import type { Session, SessionFileState } from '$types';

	interface Props {
		onaddproject: () => void;
	}

	let { onaddproject }: Props = $props();

	// Check if a session has unsaved files
	function hasUnsavedFiles(fileState: SessionFileState): boolean {
		for (const file of fileState.openFiles.values()) {
			if (file.dirty) return true;
		}
		return false;
	}

	function switchToSession(sessionId: string) {
		workspaceStore.switchSession(sessionId);
	}

	async function closeSession(e: MouseEvent, session: Session) {
		e.stopPropagation();

		// Check for unsaved files
		if (hasUnsavedFiles(session.fileState)) {
			const confirmed = await confirmStore.confirm({
				title: 'Close Project',
				message: `"${session.displayName}" has unsaved changes. Close anyway?`,
				confirmText: 'Close',
				cancelText: 'Cancel',
				destructive: true
			});
			if (!confirmed) return;
		}

		await workspaceStore.closeSession(session.id);
	}

	function handleMiddleClick(e: MouseEvent, session: Session) {
		if (e.button === 1) {
			e.preventDefault();
			closeSession(e, session);
		}
	}
</script>

<div
	class="project-tabs h-12 flex items-stretch bg-sidebar-bg border-b border-panel-border overflow-x-auto scrollbar-thin"
>
	{#each $orderedSessions as session (session.id)}
		{@const isActive = session.id === $activeSession?.id}
		{@const hasUnsaved = hasUnsavedFiles(session.fileState)}
		{@const folderName = session.projectRoot.split('/').pop() || session.projectRoot}
		<div
			class="group flex items-center gap-2 px-3 py-1 min-w-0 max-w-52 border-r border-panel-border transition-colors cursor-pointer
					 {isActive ? 'bg-editor-bg border-b-2 border-b-accent' : 'hover:bg-panel-active'}"
			role="button"
			tabindex="0"
			onclick={() => switchToSession(session.id)}
			onmousedown={(e) => handleMiddleClick(e, session)}
			onkeydown={(e) => e.key === 'Enter' && switchToSession(session.id)}
			title="{session.connectionProfile.host}:{session.projectRoot}"
		>
			<!-- Connection status indicator -->
			<span class="w-2 h-2 rounded-full bg-green-500 flex-shrink-0 self-center"></span>

			<!-- Two-line content: connection + folder -->
			<div class="flex flex-col min-w-0 flex-1">
				<!-- Connection name (small) -->
				<span class="text-[10px] text-gray-500 truncate leading-tight">
					{session.connectionProfile.host}
				</span>
				<!-- Folder name with icon -->
				<div class="flex items-center gap-1.5">
					<svg class="w-3.5 h-3.5 text-yellow-500 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
						<path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
					</svg>
					<span class="text-sm truncate leading-tight">
						{#if hasUnsaved}<span class="text-accent">● </span>{/if}{folderName}
					</span>
				</div>
			</div>

			<!-- Close button -->
			<button
				class="p-1 rounded opacity-0 group-hover:opacity-100 hover:bg-panel-border transition-opacity flex-shrink-0"
				onclick={(e) => closeSession(e, session)}
				aria-label="Close project"
			>
				<svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
				</svg>
			</button>
		</div>
	{/each}

	<!-- Add Project Button -->
	<button
		class="flex items-center justify-center gap-1 px-4 text-gray-400 hover:text-editor-fg hover:bg-panel-active transition-colors"
		onclick={onaddproject}
		title="Add Project"
		aria-label="Add project"
	>
		<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
		</svg>
	</button>
</div>

<style>
	.scrollbar-thin {
		scrollbar-width: thin;
		scrollbar-color: rgba(255, 255, 255, 0.1) transparent;
	}

	.scrollbar-thin::-webkit-scrollbar {
		height: 4px;
	}

	.scrollbar-thin::-webkit-scrollbar-track {
		background: transparent;
	}

	.scrollbar-thin::-webkit-scrollbar-thumb {
		background-color: rgba(255, 255, 255, 0.1);
		border-radius: 2px;
	}
</style>
