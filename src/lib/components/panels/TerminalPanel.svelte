<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { invoke, listen } from '$utils/tauri';
	import { notificationsStore } from '$stores/notifications';
	import { settingsStore } from '$stores/settings';

	import type { Terminal as TerminalType } from 'xterm';
	import type { FitAddon as FitAddonType } from '@xterm/addon-fit';

	interface Props {
		terminalId: string;
		active?: boolean;
		connectionDisconnected?: boolean;
	}

	let { terminalId, active = false, connectionDisconnected = false }: Props = $props();

	let terminalContainer: HTMLDivElement;
	let terminal: TerminalType | null = null;
	let fitAddon: FitAddonType | null = null;
	let unlisten: (() => void) | null = null;
	let resizeObserver: ResizeObserver | null = null;
	let writeErrorNotified = false;
	let resizeErrorNotified = false;
	let disconnected = $state(false);
	const scrollback = $derived($settingsStore.terminalScrollback ?? 50_000);

	$effect(() => {
		if (connectionDisconnected) disconnected = true;
	});

	$effect(() => {
		if (!terminal) return;
		terminal.options.scrollback = scrollback;
	});

	function getDefaultFontSize(): number {
		if (typeof window === 'undefined') return 14;
		// Prefer a larger, more readable default on touch devices / tablets.
		if (window.matchMedia?.('(pointer: coarse)').matches) return 16;
		return 14;
	}

	async function initTerminal() {
		if (!terminalContainer) return;

		const [{ Terminal }, { FitAddon }, { WebLinksAddon }] = await Promise.all([
			import('xterm'),
			import('@xterm/addon-fit'),
			import('@xterm/addon-web-links')
		]);
		await import('xterm/css/xterm.css');

		const theme = {
			background: '#0f172a', // slate-900-ish (darker + consistent on mobile)
			foreground: '#e5e7eb', // gray-200 for high contrast
			cursor: '#f8fafc',
			cursorAccent: '#0f172a',
			selectionBackground: '#334155', // slate-700
			black: '#0f172a',
			red: '#ef4444',
			green: '#22c55e',
			yellow: '#eab308',
			blue: '#60a5fa',
			magenta: '#a78bfa',
			cyan: '#22d3ee',
			white: '#e5e7eb',
			brightBlack: '#475569',
			brightRed: '#f87171',
			brightGreen: '#4ade80',
			brightYellow: '#fde047',
			brightBlue: '#93c5fd',
			brightMagenta: '#c4b5fd',
			brightCyan: '#67e8f9',
			brightWhite: '#f9fafb'
		} as const;

		terminal = new Terminal({
			cursorBlink: true,
			fontFamily: 'JetBrains Mono, Fira Code, Consolas, monospace',
			fontSize: getDefaultFontSize(),
			lineHeight: 1.15,
			scrollback,
			theme
		});

		fitAddon = new FitAddon();
		terminal.loadAddon(fitAddon);
		terminal.loadAddon(new WebLinksAddon());

		terminal.open(terminalContainer);
		fitAddon.fit();
		if (active) {
			// Focus the terminal input when first created (especially important on mobile/keyboard-driven workflows).
			terminal.focus();
		}

		// Handle user input
		terminal.onData(async (data) => {
			if (disconnected) return;
			try {
				const bytes = new TextEncoder().encode(data);
				await invoke('terminal_write', { termId: terminalId, data: Array.from(bytes) });
			} catch (error) {
				console.error('Failed to write to terminal:', error);
				disconnected = true;
				if (!writeErrorNotified) {
					writeErrorNotified = true;
					notificationsStore.notify({
						severity: 'error',
						title: 'Terminal Disconnected',
						message: 'Terminal input failed. The remote terminal may have closed or disconnected.',
						detail: error instanceof Error ? error.message : String(error)
					});
				}
			}
		});

		// Handle resize
		terminal.onResize(async ({ cols, rows }) => {
			try {
				await invoke('terminal_resize', { termId: terminalId, cols, rows });
			} catch (error) {
				console.error('Failed to resize terminal:', error);
				if (!resizeErrorNotified) {
					resizeErrorNotified = true;
					notificationsStore.notify({
						severity: 'warning',
						title: 'Terminal Resize Failed',
						message: 'Could not resize the remote terminal.',
						detail: error instanceof Error ? error.message : String(error)
					});
				}
			}
		});

		// Listen for terminal output
		unlisten = (await listen<{ terminal_id: string; data: number[] }>('terminal_output', (event) => {
			if (event.terminal_id === terminalId && terminal) {
				const bytes = new Uint8Array(event.data);
				terminal.write(bytes);
			}
		})) as () => void;

		// Resize observer
		resizeObserver = new ResizeObserver(() => {
			if (fitAddon) {
				fitAddon.fit();
			}
		});
		resizeObserver.observe(terminalContainer);
	}

	onMount(() => {
		initTerminal();
	});

		$effect(() => {
			if (!active) return;
			if (!terminal) return;

		// When a terminal panel becomes visible again, refit and focus so input works immediately.
		queueMicrotask(() => {
			try {
				fitAddon?.fit();
				terminal?.focus();
			} catch {
				// ignore focus/fit errors
			}
		});
	});

	onDestroy(() => {
		if (unlisten) {
			unlisten();
		}
		if (resizeObserver) {
			resizeObserver.disconnect();
		}
		if (terminal) {
			terminal.dispose();
		}
	});
</script>

<div class="h-full w-full bg-editor-bg p-1">
	<div class="relative h-full w-full">
		<div bind:this={terminalContainer} class="h-full w-full"></div>
		{#if disconnected}
			<div class="absolute inset-0 flex items-center justify-center bg-black/50 pointer-events-auto">
				<div class="bg-panel-bg border border-panel-border rounded px-4 py-3 text-sm text-gray-100 max-w-sm">
					<div class="font-medium mb-1">Terminal disconnected</div>
					<div class="text-xs text-gray-300">
						The remote terminal closed or the SSH connection dropped. Open a new terminal after reconnecting.
					</div>
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	:global(.xterm) {
		height: 100%;
		padding: 4px;
	}
	:global(.xterm-viewport) {
		overflow-y: auto !important;
	}
	/* Ensure we never end up with a white/transparent terminal background on mobile browsers. */
	:global(.xterm),
	:global(.xterm-viewport),
	:global(.xterm-screen) {
		background-color: #0f172a !important;
	}

	/*
	 * Android WebView + some IMEs can surface xterm.js' hidden textarea as a visible white input field.
	 * Force it to remain visually hidden without breaking focus-based input capture.
	 */
	:global(.xterm-helper-textarea) {
		background: transparent !important;
		color: transparent !important;
		caret-color: transparent !important;
		opacity: 0 !important;
		border: 0 !important;
		outline: 0 !important;
		box-shadow: none !important;
	}
</style>
