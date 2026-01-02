<script lang="ts">
	import { activeSession } from '$stores/workspace';
	import { connectionStore } from '$stores/connection';
	import { activeFile } from '$stores/files';
	import { terminalStore } from '$stores/terminal';
	import { notificationsStore, unreadCount } from '$stores/notifications';
	import { diagnosticsStore } from '$stores/diagnostics';
	import { getLanguageLabel } from '$utils/languages';

	interface Props {
		onaddproject?: () => void;
	}

	let { onaddproject }: Props = $props();

	// Mock cursor position - would be connected to editor state
	let cursorLine = $state(1);
	let cursorColumn = $state(1);

	const isConnecting = $derived($connectionStore.status === 'connecting');

	async function handleNewTerminal() {
		try {
			await terminalStore.createTerminal();
		} catch (error) {
			console.error('Failed to create terminal:', error);
			notificationsStore.notify({
				severity: 'error',
				title: 'Terminal Failed',
				message: 'Could not create a new terminal.',
				detail: error instanceof Error ? error.message : String(error)
			});
		}
	}
</script>

<div class="h-6 flex items-center px-2 bg-status-bg text-status-fg text-xs select-none" style="padding-bottom: env(safe-area-inset-bottom, 0px);">
	<!-- Connection Status -->
	{#if $activeSession}
		<div
			class="flex items-center gap-1.5 px-2 py-0.5"
		>
			<span class="w-2 h-2 rounded-full bg-success"></span>
			<span>
				{$activeSession.connectionProfile.username}@{$activeSession.connectionProfile.host}
			</span>
		</div>

		<!-- Project Root -->
		<div
			class="flex items-center gap-1 px-2 py-0.5 border-l border-white/20"
			title={$activeSession.projectRoot}
		>
			<svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
				/>
			</svg>
			<span class="truncate max-w-48">{$activeSession.projectRoot}</span>
		</div>
	{:else if isConnecting}
		<div class="flex items-center gap-1.5 px-2 py-0.5">
			<span class="w-2 h-2 rounded-full bg-warning animate-pulse"></span>
			<span>Connecting...</span>
		</div>
	{:else}
		<div class="flex items-center gap-1.5 px-2 py-0.5">
			<span class="w-2 h-2 rounded-full bg-gray-500"></span>
			<span>No project open</span>
		</div>
	{/if}

	<!-- New Terminal Button -->
	{#if $activeSession}
		<button
			class="flex items-center gap-1 px-2 py-0.5 hover:bg-white/10 rounded transition-colors border-l border-white/20"
			onclick={handleNewTerminal}
			title="New Terminal"
		>
			<svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
				/>
			</svg>
			<span>Terminal</span>
		</button>
	{/if}

	<!-- Add Project Button -->
	{#if onaddproject}
		<button
			class="flex items-center gap-1 px-2 py-0.5 hover:bg-white/10 rounded transition-colors border-l border-white/20"
			onclick={onaddproject}
			title="Add Project"
		>
			<svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
			</svg>
			<span>Add Project</span>
		</button>
	{/if}

	<!-- Spacer -->
	<div class="flex-1"></div>

	<!-- Diagnostics -->
	<button
		class="flex items-center gap-1 px-2 py-0.5 hover:bg-white/10 rounded transition-colors"
		onclick={() => diagnosticsStore.open()}
		title="Diagnostics"
		aria-label="Diagnostics"
	>
		<svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path
				stroke-linecap="round"
				stroke-linejoin="round"
				stroke-width="2"
				d="M9 17v-2a4 4 0 014-4h2M9 7h.01M15 7h.01M9 12h6"
			/>
		</svg>
		<span class="text-[10px]">DIAG</span>
	</button>

	<!-- Notifications -->
	<button
		class="flex items-center gap-1 px-2 py-0.5 hover:bg-white/10 rounded transition-colors border-l border-white/20"
		onclick={() => notificationsStore.toggleCenter()}
		title="Notifications"
		aria-label="Notifications"
	>
		<svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path
				stroke-linecap="round"
				stroke-linejoin="round"
				stroke-width="2"
				d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"
			/>
		</svg>
		{#if $unreadCount > 0}
			<span class="text-[10px] px-1.5 py-0.5 rounded bg-accent text-black">
				{$unreadCount}
			</span>
		{/if}
	</button>

	<!-- Right side info (when file is open) -->
	{#if $activeFile}
		<!-- Cursor Position -->
		<div class="px-2 border-r border-white/20">
			Ln {cursorLine}, Col {cursorColumn}
		</div>

		<!-- Encoding -->
		<div class="px-2 border-r border-white/20">UTF-8</div>

		<!-- Language -->
		<div class="px-2">{getLanguageLabel($activeFile.language)}</div>
	{/if}
</div>
