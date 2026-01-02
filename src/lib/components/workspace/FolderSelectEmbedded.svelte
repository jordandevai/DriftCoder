<script lang="ts">
	import { invoke } from '$utils/tauri';
	import { connectionStore } from '$stores/connection';
	import type { FileEntry, ConnectionProfile } from '$types';
	import Button from '$components/shared/Button.svelte';

	interface Props {
		connectionId: string;
		profile: ConnectionProfile;
		onselect: (path: string) => void;
		oncancel?: () => void;
	}

	let { connectionId, profile, onselect, oncancel }: Props = $props();

	let currentPath = $state('');
	let pathInput = $state('');
	let entries = $state<FileEntry[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let homePath = $state('');

	const bookmarks = $derived(profile.bookmarkedPaths || []);
	const recentProjects = $derived(profile.recentProjects || []);
	const isBookmarked = $derived(bookmarks.includes(currentPath));

	// Quick filter
	const filterText = $derived(() => {
		if (!pathInput || !currentPath) return '';
		const prefix = currentPath === '/' ? '/' : currentPath + '/';
		if (pathInput.startsWith(prefix) && pathInput.length > prefix.length) {
			return pathInput.slice(prefix.length).toLowerCase();
		}
		return '';
	});

	const filteredEntries = $derived(() => {
		const filter = filterText();
		if (!filter) return entries;
		return entries.filter((e) => e.name.toLowerCase().startsWith(filter));
	});

	// Load home directory on mount
	$effect(() => {
		if (!currentPath && connectionId) {
			getHomeDirectory();
		}
	});

	// Sync path input with current path
	$effect(() => {
		pathInput = currentPath;
	});

	async function getHomeDirectory() {
		try {
			const home = await invoke<string>('ssh_get_home_dir', { connId: connectionId });
			homePath = home;
			currentPath = home;
			await loadDirectory(home);
		} catch (e) {
			error =
				`Could not determine home directory. ` +
				`Enter an absolute path (e.g. / or /home/<user>) and try again. ` +
				`Details: ${e instanceof Error ? e.message : String(e)}`;
			currentPath = '/';
			await loadDirectory('/', { keepError: true });
		}
	}

	async function loadDirectory(path: string, options?: { keepError?: boolean }) {
		loading = true;
		if (!options?.keepError) error = null;

		try {
			const resolvedPath =
				path === '~'
					? homePath || '/'
					: homePath && path.startsWith('~/')
						? homePath + path.slice(1)
						: path;
			entries = await invoke<FileEntry[]>('sftp_list_dir', {
				connId: connectionId,
				path: resolvedPath
			});
			entries = entries.filter((e) => e.isDirectory).sort((a, b) => a.name.localeCompare(b.name));
			currentPath = resolvedPath;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	function handlePathSubmit(e: SubmitEvent) {
		e.preventDefault();
		const filter = filterText();
		const filtered = filteredEntries();

		if (filter && filtered.length === 1) {
			navigateTo(filtered[0]);
			return;
		}

		if (pathInput && pathInput !== currentPath) {
			loadDirectory(pathInput);
		}
	}

	function navigateUp() {
		const parts = currentPath.split('/').filter(Boolean);
		if (parts.length > 0) {
			parts.pop();
			loadDirectory('/' + parts.join('/') || '/');
		}
	}

	function navigateTo(entry: FileEntry) {
		loadDirectory(entry.path);
	}

	function navigateToBreadcrumb(index: number) {
		const parts = currentPath.split('/').filter(Boolean);
		const newPath = '/' + parts.slice(0, index + 1).join('/');
		loadDirectory(newPath);
	}

	function toggleBookmark() {
		connectionStore.toggleBookmark(profile.id, currentPath);
	}

	function selectCurrent() {
		onselect(currentPath);
	}

	function getPathParts(): string[] {
		return currentPath.split('/').filter(Boolean);
	}

	function getFolderName(path: string): string {
		return path.split('/').pop() || path;
	}
</script>

<div class="flex gap-4 h-[450px]">
	<!-- Sidebar: Shortcuts & Bookmarks -->
	<div class="w-44 flex-shrink-0 space-y-3 overflow-y-auto">
		<!-- Quick Access -->
		<div>
			<p class="text-xs text-gray-500 uppercase tracking-wide mb-1.5">Quick Access</p>
			<div class="space-y-0.5">
				<button
					class="w-full flex items-center gap-2 px-2 py-1 text-sm rounded hover:bg-sidebar-hover transition-colors text-left"
					onclick={() => loadDirectory(homePath || currentPath || '/')}
				>
					<svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
							d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
					</svg>
					Home
				</button>
				<button
					class="w-full flex items-center gap-2 px-2 py-1 text-sm rounded hover:bg-sidebar-hover transition-colors text-left"
					onclick={() => loadDirectory('/')}
				>
					<svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
							d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z" />
					</svg>
					Root
				</button>
			</div>
		</div>

		<!-- Bookmarks -->
		{#if bookmarks.length > 0}
			<div>
				<p class="text-xs text-gray-500 uppercase tracking-wide mb-1.5">Bookmarks</p>
				<div class="space-y-0.5">
					{#each bookmarks as bookmark}
						<button
							class="w-full flex items-center gap-2 px-2 py-1 text-sm rounded hover:bg-sidebar-hover transition-colors text-left"
							onclick={() => loadDirectory(bookmark)}
							title={bookmark}
						>
							<svg class="w-4 h-4 text-yellow-500 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
								<path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
							</svg>
							<span class="truncate">{getFolderName(bookmark)}</span>
						</button>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Recent Projects -->
		{#if recentProjects.length > 0}
			<div>
				<p class="text-xs text-gray-500 uppercase tracking-wide mb-1.5">Recent</p>
				<div class="space-y-0.5">
					{#each recentProjects.slice(0, 5) as project}
						<button
							class="w-full flex items-center gap-2 px-2 py-1 text-sm rounded hover:bg-sidebar-hover transition-colors text-left"
							onclick={() => onselect(project)}
							title={project}
						>
							<svg class="w-4 h-4 text-gray-400 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
									d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
							</svg>
							<span class="truncate">{getFolderName(project)}</span>
						</button>
					{/each}
				</div>
			</div>
		{/if}
	</div>

	<!-- Main Browser -->
	<div class="flex-1 bg-editor-bg border border-panel-border rounded-lg overflow-hidden flex flex-col min-h-0">
		<!-- Path Input -->
		<form class="flex items-center gap-2 px-2 py-1.5 border-b border-panel-border flex-shrink-0" onsubmit={handlePathSubmit}>
			<button
				type="button"
				class="p-1 rounded hover:bg-panel-active transition-colors disabled:opacity-50"
				onclick={navigateUp}
				disabled={currentPath === '/'}
				aria-label="Navigate up"
			>
				<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
				</svg>
			</button>
			<input
				type="text"
				bind:value={pathInput}
				class="flex-1 px-2 py-1 text-sm font-mono bg-transparent border border-transparent
					   focus:border-accent focus:outline-none rounded"
				placeholder="Enter path..."
			/>
			<button
				type="button"
				class="p-1 rounded transition-colors {isBookmarked ? 'text-yellow-500' : 'text-gray-400 hover:text-yellow-500'}"
				onclick={toggleBookmark}
				aria-label={isBookmarked ? 'Remove bookmark' : 'Add bookmark'}
			>
				<svg class="w-4 h-4" fill={isBookmarked ? 'currentColor' : 'none'} stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
						d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" />
				</svg>
			</button>
		</form>

		<!-- Breadcrumbs -->
		<div class="flex items-center gap-1 px-2 py-1 border-b border-panel-border text-xs overflow-x-auto flex-shrink-0">
			<button
				class="px-1 py-0.5 rounded hover:bg-panel-active transition-colors"
				onclick={() => loadDirectory('/')}
			>/</button>
			{#each getPathParts() as part, i}
				<span class="text-gray-500">/</span>
				<button
					class="px-1 py-0.5 rounded hover:bg-panel-active transition-colors truncate max-w-20"
					onclick={() => navigateToBreadcrumb(i)}
				>{part}</button>
			{/each}
			{#if filterText()}
				<span class="text-gray-500">/</span>
				<span class="px-1 py-0.5 text-accent italic">{filterText()}*</span>
				<span class="ml-auto text-gray-500">
					{filteredEntries().length}/{entries.length}
				</span>
			{/if}
		</div>

		<!-- Directory List -->
		<div class="flex-1 overflow-y-auto min-h-0">
			{#if loading}
				<div class="p-6 text-center text-gray-400">
					<svg class="animate-spin h-5 w-5 mx-auto mb-2" viewBox="0 0 24 24">
						<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none" />
						<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
					</svg>
					Loading...
				</div>
			{:else if error}
				<div class="p-6 text-center text-error">
					<p class="text-sm">{error}</p>
					<button class="mt-2 text-xs text-accent hover:underline" onclick={() => loadDirectory(currentPath)}>
						Retry
					</button>
				</div>
			{:else if entries.length === 0}
				<div class="p-6 text-center text-gray-400 text-sm">No subdirectories</div>
			{:else}
				{@const filtered = filteredEntries()}
				{@const filter = filterText()}
				{#if filter && filtered.length === 0}
					<div class="p-4 text-center text-gray-400 text-sm">
						No folders matching "{filter}"
					</div>
				{:else}
					{#each filtered as entry (entry.path)}
						<button
							class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-sidebar-hover transition-colors text-left text-sm"
							onclick={() => navigateTo(entry)}
						>
							<svg class="w-4 h-4 text-yellow-500 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
								<path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
							</svg>
							{#if filter}
								<span class="flex-1 truncate">
									<span class="text-accent">{entry.name.slice(0, filter.length)}</span>{entry.name.slice(filter.length)}
								</span>
							{:else}
								<span class="flex-1 truncate">{entry.name}</span>
							{/if}
						</button>
					{/each}
				{/if}
			{/if}
		</div>

		<!-- Actions -->
		<div class="px-3 py-2 border-t border-panel-border flex justify-between flex-shrink-0">
			{#if oncancel}
				<Button variant="ghost" size="sm" onclick={oncancel}>Cancel</Button>
			{:else}
				<div></div>
			{/if}
			<Button size="sm" onclick={selectCurrent}>Open Folder</Button>
		</div>
	</div>
</div>
