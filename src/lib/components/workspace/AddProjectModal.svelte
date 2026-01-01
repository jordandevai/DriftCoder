<script lang="ts">
	import Modal from '$components/shared/Modal.svelte';
	import Button from '$components/shared/Button.svelte';
	import ConnectionForm from '$components/connection/ConnectionForm.svelte';
	import FolderSelectEmbedded from './FolderSelectEmbedded.svelte';
	import { connectionStore, activeConnectionsList } from '$stores/connection';
	import { workspaceStore } from '$stores/workspace';
	import type { ConnectionProfile, ActiveConnection } from '$types';

	interface Props {
		open: boolean;
		onclose?: () => void;
	}

	let { open = $bindable(false), onclose }: Props = $props();

	// Steps: 'choose' | 'select-connection' | 'new-connection' | 'folder'
	type Step = 'choose' | 'select-connection' | 'new-connection' | 'folder';
	let step = $state<Step>('choose');

	// Selected connection for folder selection
	let selectedConnectionId = $state<string | null>(null);
	let selectedProfile = $state<ConnectionProfile | null>(null);

	// Get active connections for selection
	const activeConnections = $derived($activeConnectionsList);
	const hasActiveConnections = $derived(activeConnections.length > 0);

	function resetState() {
		step = 'choose';
		selectedConnectionId = null;
		selectedProfile = null;
	}

	function handleClose() {
		resetState();
		open = false;
		onclose?.();
	}

	function handleUseSameConnection() {
		if (activeConnections.length === 1) {
			// Only one connection, use it directly
			selectedConnectionId = activeConnections[0].id;
			selectedProfile = activeConnections[0].profile;
			step = 'folder';
		} else if (activeConnections.length > 1) {
			// Multiple connections, let user choose
			step = 'select-connection';
		}
	}

	function handleNewConnection() {
		step = 'new-connection';
	}

	function handleSelectConnection(conn: ActiveConnection) {
		selectedConnectionId = conn.id;
		selectedProfile = conn.profile;
		step = 'folder';
	}

	async function handleConnectionFormConnect(profile: ConnectionProfile, password?: string) {
		try {
			const connectionId = await connectionStore.connect(profile, password);
			selectedConnectionId = connectionId;
			selectedProfile = profile;
			step = 'folder';
		} catch (error) {
			console.error('Connection failed:', error);
		}
	}

	function handleConnectionFormSave(profile: ConnectionProfile) {
		connectionStore.addProfile(profile);
	}

	async function handleFolderSelect(path: string) {
		if (!selectedConnectionId || !selectedProfile) return;

		try {
			await workspaceStore.createSession(selectedConnectionId, selectedProfile, path);
			// Add to recent projects
			connectionStore.addRecentProject(selectedProfile.id, path);
			handleClose();
		} catch (error) {
			console.error('Failed to create session:', error);
		}
	}

	function handleBack() {
		if (step === 'folder' && activeConnections.length > 1) {
			step = 'select-connection';
		} else {
			step = 'choose';
		}
		selectedConnectionId = null;
		selectedProfile = null;
	}

	// Reset when modal opens
	$effect(() => {
		if (open) {
			resetState();
		}
	});
</script>

<Modal bind:open title="Add Project" size="lg" onclose={handleClose}>
	{#if step === 'choose'}
		<!-- Step 1: Choose connection type -->
		<div class="space-y-4">
			<p class="text-gray-400 text-sm">How would you like to connect to your project?</p>

			<div class="space-y-3">
				{#if hasActiveConnections}
					<button
						class="w-full p-4 text-left bg-editor-bg border border-panel-border rounded-lg hover:border-accent transition-colors group"
						onclick={handleUseSameConnection}
					>
						<div class="flex items-center gap-3">
							<div class="p-2 rounded-lg bg-success/10 text-success">
								<svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
										d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
								</svg>
							</div>
							<div class="flex-1">
								<div class="font-medium text-editor-fg group-hover:text-accent transition-colors">
									Use Existing Connection
								</div>
								<div class="text-sm text-gray-400">
									Open another project on an active connection
								</div>
							</div>
							<svg class="w-5 h-5 text-gray-400 group-hover:text-accent transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
							</svg>
						</div>
						{#if activeConnections.length > 0}
							<div class="mt-2 flex flex-wrap gap-2">
								{#each activeConnections as conn}
									<span class="inline-flex items-center gap-1 px-2 py-0.5 bg-panel-bg rounded text-xs text-gray-400">
										<span class="w-1.5 h-1.5 rounded-full bg-success"></span>
										{conn.profile.host}
									</span>
								{/each}
							</div>
						{/if}
					</button>
				{/if}

				<button
					class="w-full p-4 text-left bg-editor-bg border border-panel-border rounded-lg hover:border-accent transition-colors group"
					onclick={handleNewConnection}
				>
					<div class="flex items-center gap-3">
						<div class="p-2 rounded-lg bg-accent/10 text-accent">
							<svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
									d="M12 4v16m8-8H4" />
							</svg>
						</div>
						<div class="flex-1">
							<div class="font-medium text-editor-fg group-hover:text-accent transition-colors">
								New Connection
							</div>
							<div class="text-sm text-gray-400">
								Connect to a different server
							</div>
						</div>
						<svg class="w-5 h-5 text-gray-400 group-hover:text-accent transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
						</svg>
					</div>
				</button>
			</div>
		</div>

	{:else if step === 'select-connection'}
		<!-- Step 2a: Select from active connections -->
		<div class="space-y-4">
			<div class="flex items-center gap-2">
				<button
					class="p-1 rounded hover:bg-panel-active transition-colors"
					onclick={handleBack}
					aria-label="Back"
				>
					<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
					</svg>
				</button>
				<p class="text-gray-400 text-sm">Select a connection to use:</p>
			</div>

			<div class="space-y-2">
				{#each activeConnections as conn}
					<button
						class="w-full p-3 text-left bg-editor-bg border border-panel-border rounded-lg hover:border-accent transition-colors group"
						onclick={() => handleSelectConnection(conn)}
					>
						<div class="flex items-center gap-3">
							<span class="w-2 h-2 rounded-full bg-success flex-shrink-0"></span>
							<div class="flex-1 min-w-0">
								<div class="font-medium text-editor-fg truncate group-hover:text-accent transition-colors">
									{conn.profile.name || `${conn.profile.username}@${conn.profile.host}`}
								</div>
								<div class="text-xs text-gray-400 truncate">
									{conn.profile.host}:{conn.profile.port} ({conn.sessionCount} project{conn.sessionCount !== 1 ? 's' : ''})
								</div>
							</div>
							<svg class="w-5 h-5 text-gray-400 group-hover:text-accent transition-colors flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
							</svg>
						</div>
					</button>
				{/each}
			</div>
		</div>

	{:else if step === 'new-connection'}
		<!-- Step 2b: New connection form -->
		<div class="space-y-4">
			<div class="flex items-center gap-2 mb-4">
				<button
					class="p-1 rounded hover:bg-panel-active transition-colors"
					onclick={handleBack}
					aria-label="Back"
				>
					<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
					</svg>
				</button>
				<p class="text-gray-400 text-sm">Enter connection details:</p>
			</div>

			<ConnectionForm
				onclose={handleBack}
				onsave={handleConnectionFormSave}
				onconnect={handleConnectionFormConnect}
			/>
		</div>

	{:else if step === 'folder'}
		<!-- Step 3: Folder selection -->
		<div class="space-y-4">
			<div class="flex items-center gap-2 mb-4">
				<button
					class="p-1 rounded hover:bg-panel-active transition-colors"
					onclick={handleBack}
					aria-label="Back"
				>
					<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
					</svg>
				</button>
				<div class="flex items-center gap-2 text-sm text-gray-400">
					<span class="w-2 h-2 rounded-full bg-success"></span>
					<span>Connected to {selectedProfile?.host}</span>
				</div>
			</div>

			{#if selectedConnectionId && selectedProfile}
				<FolderSelectEmbedded
					connectionId={selectedConnectionId}
					profile={selectedProfile}
					onselect={handleFolderSelect}
					oncancel={handleBack}
				/>
			{/if}
		</div>
	{/if}
</Modal>
