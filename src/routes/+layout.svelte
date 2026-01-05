<script lang="ts">
	import { onMount } from 'svelte';
	import '../app.css';
	import { connectionStore } from '$stores/connection';
	import { debugStore } from '$stores/debug';
	import { settingsStore } from '$stores/settings';
	import { terminalStore } from '$stores/terminal';
	import { hasSessions, workspaceStore } from '$stores/workspace';
	import { invoke, isTauri } from '$utils/tauri';

	let { children } = $props();

	onMount(() => {
		connectionStore.init();
		debugStore.init();
		void settingsStore.init();
		void workspaceStore.init().then(() => {
			terminalStore.hydrateFromWorkspace();
		});

		if (!isTauri()) return;

		// If the user hit "Disconnect" on the Android background notification, honor it immediately.
		void invoke<boolean>('android_persistence_consume_disconnect_request')
			.then(async (requested) => {
				if (!requested) return;
				await workspaceStore.closeAllSessions();
			})
			.catch((e) => {
				console.warn('Failed to consume android disconnect request:', e);
			});

		// Tell native layer whether we have open projects; the Android plugin uses this to decide
		// whether to run the Foreground Service when the activity is backgrounded.
		const unsub = hasSessions.subscribe((active) => {
			void invoke('android_persistence_set_active', { active }).catch((e) => {
				console.warn('Failed to set android persistence active:', e);
			});
		});

		return () => {
			unsub();
			void invoke('android_persistence_set_active', { active: false }).catch(() => {});
		};
	});
</script>

<svelte:head>
	<link rel="preconnect" href="https://fonts.googleapis.com" />
	<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
	<link
		href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap"
		rel="stylesheet"
	/>
</svelte:head>

<div class="h-screen w-screen overflow-hidden bg-editor-bg text-editor-fg">
	{@render children()}
</div>
