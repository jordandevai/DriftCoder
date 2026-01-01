<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { invoke, listen } from '$utils/tauri';
	import { notificationsStore } from '$stores/notifications';

	import type { Terminal as TerminalType } from 'xterm';
	import type { FitAddon as FitAddonType } from '@xterm/addon-fit';

	interface Props {
		terminalId: string;
	}

	let { terminalId }: Props = $props();

	let terminalContainer: HTMLDivElement;
	let terminal: TerminalType | null = null;
	let fitAddon: FitAddonType | null = null;
	let unlisten: (() => void) | null = null;
	let resizeObserver: ResizeObserver | null = null;
	let writeErrorNotified = false;
	let resizeErrorNotified = false;

	async function initTerminal() {
		if (!terminalContainer) return;

		const [{ Terminal }, { FitAddon }, { WebLinksAddon }] = await Promise.all([
			import('xterm'),
			import('@xterm/addon-fit'),
			import('@xterm/addon-web-links')
		]);
		await import('xterm/css/xterm.css');

		terminal = new Terminal({
			cursorBlink: true,
			fontFamily: 'JetBrains Mono, Fira Code, Consolas, monospace',
			fontSize: 14,
			theme: {
				background: '#1e1e1e',
				foreground: '#d4d4d4',
				cursor: '#aeafad',
				cursorAccent: '#1e1e1e',
				selectionBackground: '#264f78',
				black: '#1e1e1e',
				red: '#f44747',
				green: '#4ec9b0',
				yellow: '#dcdcaa',
				blue: '#569cd6',
				magenta: '#c586c0',
				cyan: '#9cdcfe',
				white: '#d4d4d4',
				brightBlack: '#808080',
				brightRed: '#f44747',
				brightGreen: '#4ec9b0',
				brightYellow: '#dcdcaa',
				brightBlue: '#569cd6',
				brightMagenta: '#c586c0',
				brightCyan: '#9cdcfe',
				brightWhite: '#ffffff'
			}
		});

		fitAddon = new FitAddon();
		terminal.loadAddon(fitAddon);
		terminal.loadAddon(new WebLinksAddon());

		terminal.open(terminalContainer);
		fitAddon.fit();

		// Handle user input
		terminal.onData(async (data) => {
			try {
				const bytes = new TextEncoder().encode(data);
				await invoke('terminal_write', { termId: terminalId, data: Array.from(bytes) });
			} catch (error) {
				console.error('Failed to write to terminal:', error);
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
	<div bind:this={terminalContainer} class="h-full w-full"></div>
</div>

<style>
	:global(.xterm) {
		height: 100%;
		padding: 4px;
	}
	:global(.xterm-viewport) {
		overflow-y: auto !important;
	}
</style>
