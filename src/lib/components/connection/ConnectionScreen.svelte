<script lang="ts">
	import { connectionStore } from '$stores/connection';
	import { notificationsStore, unreadCount } from '$stores/notifications';
	import { diagnosticsStore } from '$stores/diagnostics';
	import ConnectionForm from './ConnectionForm.svelte';
	import ConnectionList from './ConnectionList.svelte';
	import type { ConnectionProfile } from '$types';

	interface Props {
		onconnected?: (profile: ConnectionProfile, password?: string, projectPath?: string) => void;
	}

	let { onconnected }: Props = $props();

	let showNewConnection = $state(false);
	let editingProfile = $state<ConnectionProfile | null>(null);

	function handleNewConnection() {
		editingProfile = null;
		showNewConnection = true;
	}

	function handleEditProfile(profile: ConnectionProfile) {
		editingProfile = profile;
		showNewConnection = true;
	}

	function handleFormClose() {
		showNewConnection = false;
		editingProfile = null;
	}

	function handleFormSave(profile: ConnectionProfile) {
		connectionStore.addProfile(profile);
		showNewConnection = false;
		editingProfile = null;
	}

	async function handleConnect(profile: ConnectionProfile, password?: string, projectPath?: string) {
		// Pass connection details to parent - parent handles actual connection
		onconnected?.(profile, password, projectPath);
	}
</script>

<div class="h-full flex flex-col items-center justify-center p-8 bg-editor-bg relative">
	<!-- Floating toolbar: Debug + Notifications -->
	<div class="absolute right-4 flex items-center gap-2" style="top: calc(1rem + env(safe-area-inset-top, 0px));">
		<button
			class="flex items-center gap-1.5 px-3 py-1.5 bg-panel-bg hover:bg-panel-border rounded-lg transition-colors text-gray-400 text-sm"
			onclick={() => diagnosticsStore.open()}
			title="Open diagnostics"
		>
			<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M9 17v-2a4 4 0 014-4h2M9 7h.01M15 7h.01M9 12h6m2 9H7a2 2 0 01-2-2V5a2 2 0 012-2h10a2 2 0 012 2v14a2 2 0 01-2 2z"
				/>
			</svg>
			<span class="hidden sm:inline">Diagnostics</span>
		</button>

		<!-- Notifications -->
		<button
			class="flex items-center gap-1.5 px-3 py-1.5 bg-panel-bg hover:bg-panel-border rounded-lg transition-colors text-gray-400 text-sm"
			onclick={() => notificationsStore.toggleCenter()}
			title="Open notification center"
		>
			<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"
				/>
			</svg>
			{#if $unreadCount > 0}
				<span class="px-1.5 py-0.5 rounded bg-accent text-black text-xs font-medium">
					{$unreadCount}
				</span>
			{/if}
		</button>
	</div>

	<div class="w-full max-w-lg">
		<!-- Logo and Title -->
		<div class="text-center mb-8">
			<h1 class="text-3xl font-bold text-editor-fg mb-2">DriftCode</h1>
			<p class="text-gray-400">Your code, wherever you drift. No server install. Just SSH.</p>
		</div>

		{#if showNewConnection}
			<!-- Connection Form -->
			<ConnectionForm
				profile={editingProfile}
				onclose={handleFormClose}
				onsave={handleFormSave}
				onconnect={handleConnect}
			/>
		{:else}
			<!-- Connection List or Empty State -->
			{#if $connectionStore.savedProfiles.length > 0}
				<ConnectionList
					profiles={$connectionStore.savedProfiles}
					onconnect={handleConnect}
					onedit={handleEditProfile}
					ondelete={(id) => connectionStore.removeProfile(id)}
					onnew={handleNewConnection}
				/>
			{:else}
				<!-- Empty State -->
				<div class="bg-panel-bg border border-panel-border rounded-lg p-8 text-center">
					<div class="mb-4">
						<svg
							class="w-16 h-16 mx-auto text-gray-500"
							fill="none"
							stroke="currentColor"
							viewBox="0 0 24 24"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="1.5"
								d="M5 12h14M12 5l7 7-7 7"
							/>
						</svg>
					</div>
					<h3 class="text-lg font-medium text-editor-fg mb-2">No connections yet</h3>
					<p class="text-gray-400 mb-6">
						Connect to your remote machine to start editing code.
					</p>
					<button
						class="btn-primary"
						onclick={handleNewConnection}
					>
						New Connection
					</button>
				</div>
			{/if}
		{/if}

		<!-- Error Display -->
		{#if $connectionStore.error}
			<div class="mt-4 p-4 bg-error/10 border border-error rounded-lg text-error text-sm">
				{$connectionStore.error}
			</div>
		{/if}

		<!-- Connection Status -->
		{#if $connectionStore.status === 'connecting'}
			<div class="mt-4 flex items-center justify-center gap-2 text-gray-400">
				<svg class="animate-spin h-4 w-4" viewBox="0 0 24 24">
					<circle
						class="opacity-25"
						cx="12"
						cy="12"
						r="10"
						stroke="currentColor"
						stroke-width="4"
						fill="none"
					/>
					<path
						class="opacity-75"
						fill="currentColor"
						d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
					/>
				</svg>
				<span>Connecting...</span>
			</div>
		{/if}
	</div>
</div>
