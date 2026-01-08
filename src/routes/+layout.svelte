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
		let baselineLayoutHeight = 0;

		function updateViewportVars(): void {
			if (typeof window === 'undefined') return;
			const root = document.documentElement;
			const vv = window.visualViewport;

			const layoutHeight = window.innerHeight;
			const visualHeight = vv?.height ?? layoutHeight;
			const offsetTop = vv?.offsetTop ?? 0;

			// Track the "no keyboard" baseline. This lets us compute keyboard insets even on
			// platforms where the layout viewport shrinks alongside the visual viewport.
			if (baselineLayoutHeight === 0) baselineLayoutHeight = layoutHeight;
			baselineLayoutHeight = Math.max(baselineLayoutHeight, layoutHeight);

			const KEYBOARD_THRESHOLD_PX = 80;

			// If the layout viewport shrinks, the OS is already resizing the WebView (adjustResize).
			const resizedByKeyboard = layoutHeight < baselineLayoutHeight - KEYBOARD_THRESHOLD_PX;

			// If only the visual viewport shrinks, the keyboard is overlaying content.
			const visualOverlayKeyboard =
				!resizedByKeyboard &&
				visualHeight + offsetTop < baselineLayoutHeight - KEYBOARD_THRESHOLD_PX;

			// Native Android fallback (WebView can fail to update VisualViewport on IME open/close).
			let nativeInsetBottom = 0;
			try {
				const raw = root.style.getPropertyValue('--native-keyboard-inset-bottom');
				// If not set inline, fall back to computed style.
				const val = raw || getComputedStyle(root).getPropertyValue('--native-keyboard-inset-bottom');
				nativeInsetBottom = Math.max(0, parseInt(String(val).trim(), 10) || 0);
			} catch {
				nativeInsetBottom = 0;
			}

			const nativeOverlayKeyboard = !resizedByKeyboard && nativeInsetBottom > KEYBOARD_THRESHOLD_PX;

			const visualInsetBottom = visualOverlayKeyboard
				? Math.max(0, Math.round(baselineLayoutHeight - visualHeight - offsetTop))
				: 0;

			const overlayKeyboard = visualOverlayKeyboard || nativeOverlayKeyboard;
			const keyboardInsetBottom = overlayKeyboard ? Math.max(visualInsetBottom, nativeInsetBottom) : 0;

			// For overlay keyboards we keep the container at baseline height and use padding to
			// lift content above the keyboard. For resize keyboards we match the layout height.
			const appHeight = overlayKeyboard ? baselineLayoutHeight : layoutHeight;

			root.style.setProperty('--app-viewport-height', `${Math.max(0, Math.round(appHeight))}px`);
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

		// Some WebViews don’t reliably fire resize/visualViewport events on IME open/close.
		// Focus events are a good extra signal (login fields, xterm helper textarea, etc.).
		const onFocusChange = () => {
			updateViewportVars();
			window.setTimeout(updateViewportVars, 50);
			window.setTimeout(updateViewportVars, 250);
		};
		window.addEventListener('focusin', onFocusChange);
		window.addEventListener('focusout', onFocusChange);

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
			window.removeEventListener('focusin', onFocusChange);
			window.removeEventListener('focusout', onFocusChange);
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
	style="height: var(--app-viewport-height, 100vh); padding-bottom: var(--keyboard-inset-bottom, 0px); box-sizing: border-box;"
>
	{@render children()}
</div>
