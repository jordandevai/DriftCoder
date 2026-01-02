<script lang="ts">
	import { fileStore } from '$stores/files';
	import { layoutStore } from '$stores/layout';
	import { activeSession } from '$stores/workspace';
	import { confirmStore } from '$stores/confirm';
	import type { FileEntry } from '$types';

	let contextMenu = $state<{ x: number; y: number; entry: FileEntry } | null>(null);
	let renaming = $state<{ path: string; name: string } | null>(null);
	let creating = $state<{ parentPath: string; type: 'file' | 'folder'; name: string } | null>(null);

	function toggleDirectory(entry: FileEntry) {
		if (entry.isDirectory) {
			fileStore.toggleDirectory(entry.path);
		}
	}

	async function openFile(entry: FileEntry) {
		if (!entry.isDirectory) {
			const sessionId = $activeSession?.id;
			if (!sessionId) return;

			await fileStore.openFile(entry.path);
			// Add editor panel if not already open
			const existingPanel = layoutStore.findPanelByFilePath(entry.path, sessionId);
			if (!existingPanel) {
				layoutStore.addPanelForSession(sessionId, {
					type: 'editor',
					title: entry.name,
					filePath: entry.path
				});
			} else {
				layoutStore.setActivePanelForSession(sessionId, existingPanel.id);
			}
		}
	}

	function handleContextMenu(e: MouseEvent, entry: FileEntry) {
		e.preventDefault();
		contextMenu = { x: e.clientX, y: e.clientY, entry };
	}

	function closeContextMenu() {
		contextMenu = null;
	}

	async function handleDelete(entry: FileEntry) {
		const confirmed = await confirmStore.confirm({
			title: 'Delete',
			message: `Delete "${entry.name}"?`,
			confirmText: 'Delete',
			cancelText: 'Cancel',
			destructive: true
		});
		if (confirmed) {
			await fileStore.deleteEntry(entry.path);
		}
		closeContextMenu();
	}

	function startRename(entry: FileEntry) {
		renaming = { path: entry.path, name: entry.name };
		closeContextMenu();
	}

	async function finishRename() {
		if (renaming) {
			const oldPath = renaming.path;
			const newPath = oldPath.substring(0, oldPath.lastIndexOf('/') + 1) + renaming.name;
			if (newPath !== oldPath) {
				await fileStore.renameEntry(oldPath, newPath);
			}
			renaming = null;
		}
	}

	function startCreate(type: 'file' | 'folder', parentPath: string) {
		creating = { parentPath, type, name: '' };
		closeContextMenu();
	}

	async function finishCreate() {
		if (creating && creating.name) {
			const fullPath = `${creating.parentPath}/${creating.name}`;
			if (creating.type === 'file') {
				await fileStore.createFile(fullPath);
			} else {
				await fileStore.createDirectory(fullPath);
			}
			creating = null;
		}
	}

	function getFileIcon(entry: FileEntry): string {
		if (entry.isDirectory) return 'folder';
		const ext = entry.name.split('.').pop()?.toLowerCase();
		return ext || 'file';
	}

	function isExpanded(path: string): boolean {
		return $fileStore.expandedPaths.has(path);
	}

	function getIndentStyle(depth: number): string {
		return `padding-left: ${12 + depth * 16}px`;
	}
</script>

{#snippet fileTreeItem(entry: FileEntry, depth: number)}
	{@const expanded = isExpanded(entry.path)}
	{@const active = entry.path === $fileStore.activeFilePath}

	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="flex items-center gap-1 py-1 cursor-pointer hover:bg-sidebar-hover transition-colors {active
			? 'bg-sidebar-active'
			: ''}"
		style={getIndentStyle(depth)}
		onclick={() => (entry.isDirectory ? toggleDirectory(entry) : openFile(entry))}
		ondblclick={() => openFile(entry)}
		oncontextmenu={(e) => handleContextMenu(e, entry)}
	>
		<!-- Expand/Collapse Arrow -->
		{#if entry.isDirectory}
			<svg
				class="w-4 h-4 text-gray-500 transition-transform {expanded ? 'rotate-90' : ''}"
				fill="none"
				stroke="currentColor"
				viewBox="0 0 24 24"
			>
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
			</svg>
		{:else}
			<span class="w-4"></span>
		{/if}

		<!-- Icon -->
		{#if entry.isDirectory}
			<svg class="w-4 h-4 {expanded ? 'text-yellow-400' : 'text-yellow-500'}" fill="currentColor" viewBox="0 0 20 20">
				<path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
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

		<!-- Name -->
		{#if renaming?.path === entry.path}
			<!-- svelte-ignore a11y_autofocus -->
			<input
				type="text"
				bind:value={renaming.name}
				class="flex-1 px-1 text-sm bg-editor-bg border border-accent rounded focus:outline-none"
				onkeydown={(e) => {
					if (e.key === 'Enter') finishRename();
					if (e.key === 'Escape') renaming = null;
				}}
				onclick={(e) => e.stopPropagation()}
				autofocus
			/>
		{:else}
			<span class="flex-1 text-sm truncate">{entry.name}</span>
		{/if}
	</div>

	<!-- Render children if expanded -->
	{#if entry.isDirectory && expanded && entry.children}
		{#each entry.children as child (child.path)}
			{@render fileTreeItem(child, depth + 1)}
		{/each}
	{/if}
{/snippet}

<svelte:window onclick={closeContextMenu} />

<div class="h-full flex flex-col bg-sidebar-bg">
	<!-- Header -->
	<div class="flex items-center justify-between px-3 py-2 border-b border-panel-border">
		<span class="text-xs font-medium uppercase text-gray-400">Explorer</span>
		<div class="flex gap-1">
			<button
				class="p-1 rounded hover:bg-sidebar-hover transition-colors"
				title="Expand all (loaded)"
				onclick={() => fileStore.expandAllLoaded()}
			>
				<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7M4 5l7 7-7 7" />
				</svg>
			</button>
			<button
				class="p-1 rounded hover:bg-sidebar-hover transition-colors"
				title="Collapse all"
				onclick={() => fileStore.collapseAll()}
			>
				<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
				</svg>
			</button>
			<button
				class="p-1 rounded hover:bg-sidebar-hover transition-colors"
				title="New File"
				onclick={() => startCreate('file', $fileStore.projectRoot)}
			>
				<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
					/>
				</svg>
			</button>
			<button
				class="p-1 rounded hover:bg-sidebar-hover transition-colors"
				title="New Folder"
				onclick={() => startCreate('folder', $fileStore.projectRoot)}
			>
				<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M9 13h6m-3-3v6m-9 1V7a2 2 0 012-2h6l2 2h6a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z"
					/>
				</svg>
			</button>
			<button
				class="p-1 rounded hover:bg-sidebar-hover transition-colors"
				title="Refresh"
				onclick={() => fileStore.refreshDirectory($fileStore.projectRoot)}
			>
				<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
					/>
				</svg>
			</button>
		</div>
	</div>

	<!-- File Tree -->
	<div class="flex-1 overflow-y-auto overflow-x-hidden py-1">
		{#if creating && creating.parentPath === $fileStore.projectRoot}
			<div class="flex items-center gap-2 px-3 py-1">
				<!-- svelte-ignore a11y_autofocus -->
				<input
					type="text"
					bind:value={creating.name}
					class="flex-1 px-2 py-0.5 text-sm bg-editor-bg border border-accent rounded focus:outline-none"
					placeholder={creating.type === 'file' ? 'filename' : 'folder name'}
					onkeydown={(e) => {
						if (e.key === 'Enter') finishCreate();
						if (e.key === 'Escape') creating = null;
					}}
					autofocus
				/>
			</div>
		{/if}

		{#each $fileStore.tree as entry (entry.path)}
			{@render fileTreeItem(entry, 0)}
		{/each}
	</div>
</div>

<!-- Context Menu -->
{#if contextMenu}
	<div
		class="fixed z-50 bg-panel-bg border border-panel-border rounded shadow-lg py-1 min-w-40"
		style="left: {contextMenu.x}px; top: {contextMenu.y}px"
	>
		{#if contextMenu.entry.isDirectory}
			<button
				class="w-full px-3 py-1.5 text-left text-sm hover:bg-sidebar-hover transition-colors"
				onclick={() => startCreate('file', contextMenu?.entry.path || '')}
			>
				New File
			</button>
			<button
				class="w-full px-3 py-1.5 text-left text-sm hover:bg-sidebar-hover transition-colors"
				onclick={() => startCreate('folder', contextMenu?.entry.path || '')}
			>
				New Folder
			</button>
			<div class="border-t border-panel-border my-1"></div>
		{/if}
		<button
			class="w-full px-3 py-1.5 text-left text-sm hover:bg-sidebar-hover transition-colors"
			onclick={() => contextMenu && startRename(contextMenu.entry)}
		>
			Rename
		</button>
		<button
			class="w-full px-3 py-1.5 text-left text-sm text-error hover:bg-error/10 transition-colors"
			onclick={() => contextMenu && handleDelete(contextMenu.entry)}
		>
			Delete
		</button>
	</div>
{/if}
