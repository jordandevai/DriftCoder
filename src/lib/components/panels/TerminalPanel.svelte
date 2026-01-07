	<script lang="ts">
		import { onMount, onDestroy } from 'svelte';
		import { invoke, listen } from '$utils/tauri';
		import { notificationsStore } from '$stores/notifications';
		import { settingsStore } from '$stores/settings';
		import { connectionStore } from '$stores/connection';
		import { workspaceStore } from '$stores/workspace';
		import { TERMINAL_THEME_KEYS, toKebabCase } from '$utils/theme';
		import TerminalHotkeysBar from '$components/terminal/TerminalHotkeysBar.svelte';

		import type { Terminal as TerminalType } from 'xterm';
		import type { FitAddon as FitAddonType } from '@xterm/addon-fit';

	interface Props {
		terminalId: string;
		sessionId: string;
		active?: boolean;
		connectionId?: string;
		connectionStatus?: 'connected' | 'reconnecting' | 'disconnected';
	}

	let {
		terminalId,
		sessionId,
		active = false,
		connectionId,
		connectionStatus = 'connected'
	}: Props = $props();

	let terminalContainer: HTMLDivElement;
	let terminal: TerminalType | null = null;
	let fitAddon: FitAddonType | null = null;
	let unlisten: (() => void) | null = null;
	let resizeObserver: ResizeObserver | null = null;
	let themeObserver: MutationObserver | null = null;
	let writeErrorNotified = false;
	let resizeErrorNotified = false;
	let ptyDisconnected = $state(false);
	let scrolledBack = $state(false);
	let outputTail = $state<number[]>([]);
	let applicationCursorKeys = $state(false);
	const scrollback = $derived($settingsStore.terminalScrollback ?? 50_000);
	const fontSize = $derived($settingsStore.fontSize ?? 14);
	const themeMode = $derived($settingsStore.themeMode);
	const themeOverrides = $derived($settingsStore.themeOverrides);
	const connectionDown = $derived(connectionStatus !== 'connected');
	const isReconnecting = $derived(connectionStatus === 'reconnecting');
	const arrowMode = $derived.by(() => (applicationCursorKeys ? 'ss3' : 'csi') as 'ss3' | 'csi');

	type HotkeyAction = { kind: 'bytes'; bytes: number[] } | { kind: 'text'; text: string };

	function isCoarsePointer(): boolean {
		if (typeof window === 'undefined') return false;
		try {
			return window.matchMedia?.('(pointer: coarse)')?.matches ?? false;
		} catch {
			return false;
		}
	}

	const hotkeysExpanded = $derived.by(() => {
		const session = $workspaceStore.sessions.get(sessionId);
		const stored = session?.terminalHotkeysExpandedById?.[terminalId];
		if (typeof stored === 'boolean') return stored;
		return isCoarsePointer();
	});

	function toggleHotkeys(): void {
		workspaceStore.setTerminalHotkeysExpanded(sessionId, terminalId, !hotkeysExpanded);
		queueMicrotask(() => terminal?.focus());
	}

	function updateApplicationCursorMode(data: number[]): void {
		// DECCKM (Application Cursor Keys): CSI ? 1 h / CSI ? 1 l
		const ON = [0x1b, 0x5b, 0x3f, 0x31, 0x68];
		const OFF = [0x1b, 0x5b, 0x3f, 0x31, 0x6c];
		const combined = [...outputTail, ...data];
		let next = applicationCursorKeys;

		for (let i = 0; i <= combined.length - 5; i += 1) {
			const slice = combined.slice(i, i + 5);
			if (slice.every((b, idx) => b === ON[idx])) next = true;
			if (slice.every((b, idx) => b === OFF[idx])) next = false;
		}

		applicationCursorKeys = next;
		outputTail = combined.slice(-8);
	}

	async function writeBytes(bytes: Uint8Array): Promise<void> {
		if (connectionDown || ptyDisconnected) return;
		try {
			await invoke('terminal_write', { termId: terminalId, data: Array.from(bytes) });
		} catch (error) {
			console.error('Failed to write to terminal:', error);
			ptyDisconnected = true;
			// If the SSH connection is already down, avoid spamming "terminal disconnected" noise;
			// the reconnect flow will surface the underlying connection issue.
			if (!connectionDown && !writeErrorNotified) {
				writeErrorNotified = true;
				notificationsStore.notify({
					severity: 'error',
					title: 'Terminal Disconnected',
					message: 'Terminal input failed. The remote terminal may have closed or disconnected.',
					detail: error instanceof Error ? error.message : String(error)
				});
			}
		}
	}

	function sendHotkey(action: HotkeyAction): void {
		if (action.kind === 'bytes') {
			void writeBytes(new Uint8Array(action.bytes));
		} else {
			void writeBytes(new TextEncoder().encode(action.text));
		}
		queueMicrotask(() => terminal?.focus());
	}

	$effect(() => {
		if (!connectionDown) {
			// When the SSH connection is healthy again (auto-reconnect), clear terminal error state so input works.
			ptyDisconnected = false;
			writeErrorNotified = false;
			resizeErrorNotified = false;
		}
	});

	$effect(() => {
		if (!terminal) return;
		terminal.options.scrollback = scrollback;
	});

	function safeFit(): void {
		if (!fitAddon || !terminal) return;
		const dims = fitAddon.proposeDimensions?.();
		if (!dims) return;
		if (dims.cols < 2 || dims.rows < 1) return;
		fitAddon.fit();
	}

	$effect(() => {
		if (!terminal) return;
		terminal.options.fontSize = fontSize;
		// Changing font size changes rows/cols; refit so the remote PTY gets an updated window size.
		requestAnimationFrame(() => {
			safeFit();
			if (terminal && terminal.rows > 0) terminal.refresh(0, terminal.rows - 1);
		});
	});

	function readRootCssVar(name: string): string {
		if (typeof window === 'undefined') return '';
		return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
	}

	function tripletToRgbCss(triplet: string): string | null {
		const parts = triplet
			.trim()
			.split(/\s+/)
			.map((p) => Number.parseInt(p, 10))
			.filter((n) => Number.isFinite(n));
		if (parts.length !== 3) return null;
		const [r, g, b] = parts.map((n) => Math.max(0, Math.min(255, n)));
		return `rgb(${r}, ${g}, ${b})`;
	}

	function getTerminalThemeFromCss(): Record<string, string> {
		const theme: Record<string, string> = {};
		for (const key of TERMINAL_THEME_KEYS) {
			const triplet = readRootCssVar(`--c-terminal-${toKebabCase(key)}`);
			const rgb = tripletToRgbCss(triplet);
			if (rgb) theme[key] = rgb;
		}
		return theme;
	}

	function getTerminalMinimumContrastRatioFromCss(): number {
		const raw = readRootCssVar('--terminal-min-contrast');
		const n = Number.parseFloat(raw);
		if (!Number.isFinite(n)) return 4.5;
		return Math.max(1, Math.min(21, n));
	}

	function applyTerminalThemeFromCss(): void {
		if (!terminal) return;
		terminal.options.theme = getTerminalThemeFromCss();
		terminal.options.minimumContrastRatio = getTerminalMinimumContrastRatioFromCss();
		if (terminal.rows > 0) terminal.refresh(0, terminal.rows - 1);
	}

	function updateScrolledBack(): void {
		if (!terminal) return;
		const buf = terminal.buffer.active;
		scrolledBack = buf.viewportY < buf.baseY;
	}

	function jumpToBottom(): void {
		if (!terminal) return;
		terminal.scrollToBottom();
		updateScrolledBack();
		queueMicrotask(() => terminal?.focus());
	}

	function startThemeObserver(): void {
		if (typeof window === 'undefined') return;
		if (typeof MutationObserver === 'undefined') return;
		if (themeObserver) return;
		themeObserver = new MutationObserver(() => applyTerminalThemeFromCss());
		themeObserver.observe(document.documentElement, {
			attributes: true,
			attributeFilter: ['data-theme', 'style']
		});
	}

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
			fontSize,
			lineHeight: 1.15,
			scrollback,
			minimumContrastRatio: getTerminalMinimumContrastRatioFromCss(),
			theme: getTerminalThemeFromCss()
		});

		fitAddon = new FitAddon();
		terminal.loadAddon(fitAddon);
		terminal.loadAddon(new WebLinksAddon());

		// Handle user input (keyboard/IME via xterm)
		terminal.onData((data) => {
			void writeBytes(new TextEncoder().encode(data));
		});

		// Handle resize
		terminal.onResize(async ({ cols, rows }) => {
			try {
				if (connectionDown || ptyDisconnected) return;
				// Avoid sending 0x0 sizes which can confuse remote TTY apps and cause redraw glitches.
				if (cols < 2 || rows < 1) return;
				await invoke('terminal_resize', { termId: terminalId, cols, rows });
			} catch (error) {
				console.error('Failed to resize terminal:', error);
				if (!connectionDown && !resizeErrorNotified) {
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

		terminal.open(terminalContainer);
		// Fit after the element is laid out; this also triggers the initial onResize so backend sees correct cols/rows.
		requestAnimationFrame(() => {
			safeFit();
			updateScrolledBack();
			if (active) {
				// Focus the terminal input when first created (especially important on mobile/keyboard-driven workflows).
				terminal?.focus();
			}
		});

		terminal.onScroll(() => {
			updateScrolledBack();
		});

		// Listen for terminal output
		unlisten = (await listen<{ terminal_id: string; data: number[] }>('terminal_output', (event) => {
			if (event.terminal_id === terminalId && terminal) {
				updateApplicationCursorMode(event.data);
				const bytes = new Uint8Array(event.data);
				terminal.write(bytes);
				// If the user isn't reviewing history, keep the view pinned to the live bottom output.
				if (!scrolledBack) {
					terminal.scrollToBottom();
				}
			}
		})) as () => void;

		// Resize observer
		resizeObserver = new ResizeObserver(() => {
			safeFit();
		});
		resizeObserver.observe(terminalContainer);

		startThemeObserver();
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
				updateScrolledBack();
				terminal?.focus();
			} catch {
				// ignore focus/fit errors
			}
		});
	});

	$effect(() => {
		// Re-apply terminal palette/contrast whenever the user edits theme settings.
		themeMode;
		themeOverrides;
		applyTerminalThemeFromCss();
	});

	onDestroy(() => {
		if (unlisten) {
			unlisten();
		}
		if (resizeObserver) {
			resizeObserver.disconnect();
		}
		if (themeObserver) {
			themeObserver.disconnect();
		}
		if (terminal) {
			terminal.dispose();
		}
	});
</script>

<div class="h-full w-full p-1" style="background-color: rgb(var(--c-terminal-background));">
	<div class="h-full w-full flex flex-col">
		<div class="relative flex-1 min-h-0">
			<div bind:this={terminalContainer} class="h-full w-full"></div>
			{#if scrolledBack && !connectionDown && !ptyDisconnected}
				<button
					class="absolute bottom-2 right-2 z-10 rounded bg-white/10 px-2 py-1 text-[11px] text-gray-100 hover:bg-white/20 transition-colors"
					onclick={jumpToBottom}
				>
					Jump to bottom
				</button>
			{/if}
			{#if connectionDown || ptyDisconnected}
				<div class="absolute left-2 right-2 top-2 z-10 pointer-events-auto">
					<div class="flex items-center justify-between gap-2 rounded border border-panel-border bg-panel-bg/95 px-3 py-2 text-xs text-gray-100 shadow">
						<div class="min-w-0">
							{#if isReconnecting}
								<div class="font-medium truncate">Reconnecting…</div>
								<div class="text-[11px] text-gray-300 truncate">
									Keeping this terminal open and restoring the session when the connection returns.
								</div>
							{:else if connectionDown}
								<div class="font-medium truncate">Disconnected</div>
								<div class="text-[11px] text-gray-300 truncate">
									Connection dropped. DriftCoder will try to reconnect automatically.
								</div>
							{:else}
								<div class="font-medium truncate">Terminal closed</div>
								<div class="text-[11px] text-gray-300 truncate">
									The remote shell ended. Reopen this terminal or create a new one.
								</div>
							{/if}
						</div>
						{#if connectionId && connectionStatus === 'disconnected'}
							<button
								class="shrink-0 rounded bg-white/10 px-2 py-1 hover:bg-white/20 transition-colors"
								onclick={async () => {
									try {
										await connectionStore.reconnect(connectionId);
									} catch {
										// reconnect flow handles prompts/errors
									}
								}}
							>
								Reconnect
							</button>
						{/if}
					</div>
				</div>
			{/if}
		</div>

		<TerminalHotkeysBar
			expanded={hotkeysExpanded}
			disabled={connectionDown || ptyDisconnected}
			arrowMode={arrowMode}
			onToggle={toggleHotkeys}
			onSend={sendHotkey}
		/>
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
		background-color: rgb(var(--c-terminal-background)) !important;
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
