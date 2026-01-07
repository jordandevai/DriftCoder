<script lang="ts">
	export type ArrowMode = 'csi' | 'ss3';

	type HotkeyAction =
		| { kind: 'bytes'; bytes: number[]; label: string; description?: string }
		| { kind: 'text'; text: string; label: string; description?: string };

	interface Props {
		expanded: boolean;
		disabled?: boolean;
		arrowMode?: ArrowMode;
		onToggle: () => void;
		onSend: (action: HotkeyAction) => void;
	}

	let {
		expanded,
		disabled = false,
		arrowMode = 'csi',
		onToggle,
		onSend
	}: Props = $props();

	const ESC = 0x1b;
	let suppressClick = $state(false);

	function ctrl(letter: string): number {
		const c = letter.toUpperCase().charCodeAt(0);
		return c >= 0x40 && c <= 0x5f ? (c & 0x1f) : 0;
	}

	function arrow(dir: 'up' | 'down' | 'left' | 'right'): number[] {
		const prefix = arrowMode === 'ss3' ? [ESC, 0x4f] : [ESC, 0x5b];
		const code = dir === 'up' ? 0x41 : dir === 'down' ? 0x42 : dir === 'right' ? 0x43 : 0x44;
		return [...prefix, code];
	}

	const rows = $derived.by(() => {
		const base: HotkeyAction[] = [
			{ kind: 'bytes', bytes: [ctrl('c')], label: 'Ctrl+C', description: 'SIGINT (interrupt)' },
			{ kind: 'bytes', bytes: [ctrl('d')], label: 'Ctrl+D', description: 'EOF (end of input)' },
			{ kind: 'bytes', bytes: [ctrl('z')], label: 'Ctrl+Z', description: 'Suspend (SIGTSTP)' },
			{ kind: 'bytes', bytes: [ctrl('l')], label: 'Ctrl+L', description: 'Clear screen' },
			{ kind: 'bytes', bytes: [ctrl('a')], label: 'Ctrl+A', description: 'Line start / tmux start' },
			{ kind: 'bytes', bytes: [ctrl('e')], label: 'Ctrl+E', description: 'Line end' },
			{ kind: 'bytes', bytes: [ctrl('b')], label: 'Ctrl+B', description: 'tmux prefix' }
		];

		const nav: HotkeyAction[] = [
			{ kind: 'bytes', bytes: [ESC], label: 'Esc', description: 'Escape' },
			{ kind: 'bytes', bytes: [0x09], label: 'Tab', description: 'Autocomplete / next field' },
			{ kind: 'bytes', bytes: arrow('up'), label: '↑', description: 'Up arrow' },
			{ kind: 'bytes', bytes: arrow('down'), label: '↓', description: 'Down arrow' },
			{ kind: 'bytes', bytes: arrow('left'), label: '←', description: 'Left arrow' },
			{ kind: 'bytes', bytes: arrow('right'), label: '→', description: 'Right arrow' }
		];

		const convenience: HotkeyAction[] = [
			{ kind: 'bytes', bytes: [0x0d], label: 'Enter', description: 'Enter / Return' },
			{ kind: 'bytes', bytes: [0x7f], label: '⌫', description: 'Backspace' },
			{ kind: 'bytes', bytes: [ESC, 0x5b, 0x35, 0x7e], label: 'PgUp', description: 'Page Up' },
			{ kind: 'bytes', bytes: [ESC, 0x5b, 0x36, 0x7e], label: 'PgDn', description: 'Page Down' }
		];

		return { base, nav, convenience };
	});

	function handleTogglePointerDown(e: PointerEvent): void {
		e.preventDefault();
		suppressClick = true;
		queueMicrotask(() => {
			suppressClick = false;
		});
		onToggle();
	}

	function handleKeyPointerDown(e: PointerEvent, action: HotkeyAction): void {
		e.preventDefault();
		suppressClick = true;
		queueMicrotask(() => {
			suppressClick = false;
		});
		if (disabled) return;
		onSend(action);
	}
</script>

<div
	class="w-full border-t border-panel-border bg-panel-bg/95 backdrop-blur supports-[backdrop-filter]:bg-panel-bg/80"
	style="padding-bottom: env(safe-area-inset-bottom, 0px);"
>
	<div class="flex items-center justify-between px-2 py-1.5">
		<button
			class="rounded px-3 py-2 text-sm bg-white/10 hover:bg-white/20 transition-colors touch-device:px-4 touch-device:py-3"
			aria-label="Toggle hotkeys"
			aria-expanded={expanded}
			onpointerdown={handleTogglePointerDown}
			onclick={() => {
				if (suppressClick) return;
				onToggle();
			}}
		>
			Hotkeys
		</button>
		{#if disabled}
			<div class="text-2xs text-gray-300">Disconnected</div>
		{/if}
	</div>

	{#if expanded}
		<div class="px-2 pb-2">
			<div class="flex gap-2 overflow-x-auto pb-1 touch-device:gap-3" role="toolbar" aria-label="Control keys">
				{#each rows.base as action (action.label)}
					<button
						class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2 py-2 touch-device:text-sm touch-device:px-3 touch-device:py-3 disabled:opacity-40"
						disabled={disabled}
						title={action.description ?? action.label}
						aria-label={action.description ? `${action.label} (${action.description})` : action.label}
						onpointerdown={(e) => handleKeyPointerDown(e, action)}
						onclick={() => {
							if (suppressClick || disabled) return;
							onSend(action);
						}}
					>
						{action.label}
					</button>
				{/each}
			</div>

			<div class="mt-2 flex gap-2 overflow-x-auto pb-1 touch-device:gap-3" role="toolbar" aria-label="Navigation keys">
				{#each rows.nav as action (action.label)}
					<button
						class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2 py-2 touch-device:text-sm touch-device:px-3 touch-device:py-3 disabled:opacity-40"
						disabled={disabled}
						title={action.description ?? action.label}
						aria-label={action.description ? `${action.label} (${action.description})` : action.label}
						onpointerdown={(e) => handleKeyPointerDown(e, action)}
						onclick={() => {
							if (suppressClick || disabled) return;
							onSend(action);
						}}
					>
						{action.label}
					</button>
				{/each}
			</div>

			<div class="mt-2 flex gap-2 overflow-x-auto pb-1 touch-device:gap-3" role="toolbar" aria-label="Convenience keys">
				{#each rows.convenience as action (action.label)}
					<button
						class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2 py-2 touch-device:text-sm touch-device:px-3 touch-device:py-3 disabled:opacity-40"
						disabled={disabled}
						title={action.description ?? action.label}
						aria-label={action.description ? `${action.label} (${action.description})` : action.label}
						onpointerdown={(e) => handleKeyPointerDown(e, action)}
						onclick={() => {
							if (suppressClick || disabled) return;
							onSend(action);
						}}
					>
						{action.label}
					</button>
				{/each}
			</div>
		</div>
	{/if}
</div>
