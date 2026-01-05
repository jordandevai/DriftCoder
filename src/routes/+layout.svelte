<script lang="ts">
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
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

		let running = false;
		const sync = async () => {
			const shouldRun = document.hidden && get(hasSessions);
			if (shouldRun === running) return;
			running = shouldRun;
			try {
				if (shouldRun) await invoke('android_persistence_start');
				else await invoke('android_persistence_stop');
			} catch (e) {
				console.warn('Android background persistence toggle failed:', e);
			}
		};

		const onVisibility = () => {
			void sync();
		};

		document.addEventListener('visibilitychange', onVisibility);
		const unsub = hasSessions.subscribe(() => {
			void sync();
		});

		void sync();

		return () => {
			document.removeEventListener('visibilitychange', onVisibility);
			unsub();
			// Best-effort: stop the service when the webview is torn down.
			void invoke('android_persistence_stop').catch(() => {});
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
