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
		function updateViewportVars(): void {
			if (typeof window === 'undefined') return;
			const root = document.documentElement;
			const vv = window.visualViewport;

			const height = vv?.height ?? window.innerHeight;
			const keyboardInsetBottom = vv
				? Math.max(0, window.innerHeight - vv.height - vv.offsetTop)
				: 0;

			root.style.setProperty('--app-viewport-height', `${Math.max(0, Math.round(height))}px`);
			root.style.setProperty(
				'--keyboard-inset-bottom',
				`${Math.max(0, Math.round(keyboardInsetBottom))}px`
			);
		}

		updateViewportVars();

		const vv = typeof window !== 'undefined' ? window.visualViewport : null;
		vv?.addEventListener('resize', updateViewportVars);
		vv?.addEventListener('scroll', updateViewportVars);
		window.addEventListener('resize', updateViewportVars);

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
			vv?.removeEventListener('resize', updateViewportVars);
			vv?.removeEventListener('scroll', updateViewportVars);
			window.removeEventListener('resize', updateViewportVars);
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

<div
	class="w-screen overflow-hidden bg-editor-bg text-editor-fg"
	style="height: var(--app-viewport-height, 100vh);"
>
	{@render children()}
</div>
