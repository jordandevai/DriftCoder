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
			const isAndroid = /Android/i.test(navigator.userAgent);

			// Simple keyboard detection: check if native plugin reports keyboard visible
			let nativeKeyboardVisible = false;
			try {
				const insetRaw = root.style.getPropertyValue('--native-keyboard-inset-bottom');
				const insetVal = insetRaw || getComputedStyle(root).getPropertyValue('--native-keyboard-inset-bottom');
				const insetPx = Math.max(0, parseInt(String(insetVal).trim(), 10) || 0);
				nativeKeyboardVisible = insetPx > 50;
			} catch {
				nativeKeyboardVisible = false;
			}

			// On Android with adjustResize, we don't need to set custom height - the OS handles it.
			// We just need to remove safe-area-bottom padding when keyboard is visible.
			const keyboardActive = isAndroid && nativeKeyboardVisible;

			root.dataset.keyboardActive = keyboardActive ? 'true' : 'false';
			root.style.setProperty(
				'--effective-safe-area-bottom',
				keyboardActive ? '0px' : 'env(safe-area-inset-bottom, 0px)'
			);
		}

		updateViewportVars();

		window.addEventListener('resize', updateViewportVars);

		const onFocusChange = () => {
			updateViewportVars();
			window.setTimeout(updateViewportVars, 100);
		};
		window.addEventListener('focusin', onFocusChange);
		window.addEventListener('focusout', onFocusChange);

		// Native Android emits this event when it detects IME inset changes.
		window.addEventListener('native-ime-insets', updateViewportVars as EventListener);

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
			window.removeEventListener('resize', updateViewportVars);
			window.removeEventListener('focusin', onFocusChange);
			window.removeEventListener('focusout', onFocusChange);
			window.removeEventListener('native-ime-insets', updateViewportVars as EventListener);
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
	class="w-screen bg-editor-bg text-editor-fg"
	style="height: 100dvh; overflow: auto;"
>
	{@render children()}
</div>

