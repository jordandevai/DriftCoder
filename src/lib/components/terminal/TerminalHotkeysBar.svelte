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
	style="padding-bottom: var(--effective-safe-area-bottom, 0px);"
>
	<!-- Collapsed: Show quick-access keys + expand handle -->
	{#if !expanded}
		<div class="flex items-center gap-1 px-2 py-1.5">
			<!-- Quick access keys when collapsed -->
			<button
				class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2 py-1.5 touch-device:px-3 touch-device:py-2 disabled:opacity-40"
				disabled={disabled}
				title="SIGINT (interrupt)"
				onpointerdown={(e) => handleKeyPointerDown(e, { kind: 'bytes', bytes: [ctrl('c')], label: 'Ctrl+C' })}
				onclick={() => { if (!suppressClick && !disabled) onSend({ kind: 'bytes', bytes: [ctrl('c')], label: 'Ctrl+C' }); }}
			>
				Ctrl+C
			</button>
			<button
				class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2 py-1.5 touch-device:px-3 touch-device:py-2 disabled:opacity-40"
				disabled={disabled}
				title="Escape"
				onpointerdown={(e) => handleKeyPointerDown(e, { kind: 'bytes', bytes: [ESC], label: 'Esc' })}
				onclick={() => { if (!suppressClick && !disabled) onSend({ kind: 'bytes', bytes: [ESC], label: 'Esc' }); }}
			>
				Esc
			</button>
			<button
				class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2 py-1.5 touch-device:px-3 touch-device:py-2 disabled:opacity-40"
				disabled={disabled}
				title="Tab"
				onpointerdown={(e) => handleKeyPointerDown(e, { kind: 'bytes', bytes: [0x09], label: 'Tab' })}
				onclick={() => { if (!suppressClick && !disabled) onSend({ kind: 'bytes', bytes: [0x09], label: 'Tab' }); }}
			>
				Tab
			</button>

			<!-- Arrow keys cluster -->
			<div class="flex gap-0.5 ml-1">
				<button
					class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-1.5 py-1.5 touch-device:px-2 touch-device:py-2 disabled:opacity-40"
					disabled={disabled}
					title="Up arrow"
					onpointerdown={(e) => handleKeyPointerDown(e, { kind: 'bytes', bytes: arrow('up'), label: '↑' })}
					onclick={() => { if (!suppressClick && !disabled) onSend({ kind: 'bytes', bytes: arrow('up'), label: '↑' }); }}
				>↑</button>
				<button
					class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-1.5 py-1.5 touch-device:px-2 touch-device:py-2 disabled:opacity-40"
					disabled={disabled}
					title="Down arrow"
					onpointerdown={(e) => handleKeyPointerDown(e, { kind: 'bytes', bytes: arrow('down'), label: '↓' })}
					onclick={() => { if (!suppressClick && !disabled) onSend({ kind: 'bytes', bytes: arrow('down'), label: '↓' }); }}
				>↓</button>
				<button
					class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-1.5 py-1.5 touch-device:px-2 touch-device:py-2 disabled:opacity-40"
					disabled={disabled}
					title="Left arrow"
					onpointerdown={(e) => handleKeyPointerDown(e, { kind: 'bytes', bytes: arrow('left'), label: '←' })}
					onclick={() => { if (!suppressClick && !disabled) onSend({ kind: 'bytes', bytes: arrow('left'), label: '←' }); }}
				>←</button>
				<button
					class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-1.5 py-1.5 touch-device:px-2 touch-device:py-2 disabled:opacity-40"
					disabled={disabled}
					title="Right arrow"
					onpointerdown={(e) => handleKeyPointerDown(e, { kind: 'bytes', bytes: arrow('right'), label: '→' })}
					onclick={() => { if (!suppressClick && !disabled) onSend({ kind: 'bytes', bytes: arrow('right'), label: '→' }); }}
				>→</button>
			</div>

			<div class="flex-1"></div>

			{#if disabled}
				<span class="text-xs text-gray-400 mr-2">Disconnected</span>
			{/if}

			<!-- Expand button -->
			<button
				class="flex items-center gap-1 rounded px-2 py-1.5 text-xs text-gray-300 hover:text-white hover:bg-white/10 transition-colors touch-device:px-3 touch-device:py-2"
				aria-label="Show more hotkeys"
				aria-expanded={expanded}
				onpointerdown={handleTogglePointerDown}
				onclick={() => { if (!suppressClick) onToggle(); }}
			>
				<span>More</span>
				<svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
				</svg>
			</button>
		</div>
	{:else}
		<!-- Expanded: Full hotkeys panel -->
		<div class="px-2 py-2">
			<!-- Header with collapse button -->
			<div class="flex items-center justify-between mb-2">
				<span class="text-xs text-gray-400 uppercase tracking-wide">Terminal Hotkeys</span>
				<button
					class="flex items-center gap-1 rounded px-2 py-1 text-xs text-gray-300 hover:text-white hover:bg-white/10 transition-colors"
					aria-label="Collapse hotkeys"
					aria-expanded={expanded}
					onpointerdown={handleTogglePointerDown}
					onclick={() => { if (!suppressClick) onToggle(); }}
				>
					<span>Less</span>
					<svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
					</svg>
				</button>
			</div>

			<!-- Control keys -->
			<div class="mb-2">
				<div class="text-[10px] text-gray-500 uppercase tracking-wide mb-1">Control</div>
				<div class="flex gap-1.5 flex-wrap" role="toolbar" aria-label="Control keys">
					{#each rows.base as action (action.label)}
						<button
							class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2 py-1.5 touch-device:text-sm touch-device:px-3 touch-device:py-2 disabled:opacity-40"
							disabled={disabled}
							title={action.description ?? action.label}
							aria-label={action.description ? `${action.label} (${action.description})` : action.label}
							onpointerdown={(e) => handleKeyPointerDown(e, action)}
							onclick={() => { if (!suppressClick && !disabled) onSend(action); }}
						>
							{action.label}
						</button>
					{/each}
				</div>
			</div>

			<!-- Navigation keys -->
			<div class="mb-2">
				<div class="text-[10px] text-gray-500 uppercase tracking-wide mb-1">Navigation</div>
				<div class="flex gap-1.5 flex-wrap" role="toolbar" aria-label="Navigation keys">
					{#each rows.nav as action (action.label)}
						<button
							class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2 py-1.5 touch-device:text-sm touch-device:px-3 touch-device:py-2 disabled:opacity-40"
							disabled={disabled}
							title={action.description ?? action.label}
							aria-label={action.description ? `${action.label} (${action.description})` : action.label}
							onpointerdown={(e) => handleKeyPointerDown(e, action)}
							onclick={() => { if (!suppressClick && !disabled) onSend(action); }}
						>
							{action.label}
						</button>
					{/each}
				</div>
			</div>

			<!-- Convenience keys -->
			<div>
				<div class="text-[10px] text-gray-500 uppercase tracking-wide mb-1">Other</div>
				<div class="flex gap-1.5 flex-wrap" role="toolbar" aria-label="Convenience keys">
					{#each rows.convenience as action (action.label)}
						<button
							class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2 py-1.5 touch-device:text-sm touch-device:px-3 touch-device:py-2 disabled:opacity-40"
							disabled={disabled}
							title={action.description ?? action.label}
							aria-label={action.description ? `${action.label} (${action.description})` : action.label}
							onpointerdown={(e) => handleKeyPointerDown(e, action)}
							onclick={() => { if (!suppressClick && !disabled) onSend(action); }}
						>
							{action.label}
						</button>
					{/each}
				</div>
			</div>
		</div>
	{/if}
</div>
