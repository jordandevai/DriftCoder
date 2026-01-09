<script lang="ts">
	import type { EditorView } from '@codemirror/view';
	import { undo, redo } from '@codemirror/commands';
	import { openSearchPanel } from '@codemirror/search';

	type EditorAction =
		| { kind: 'save'; label: string; description?: string }
		| { kind: 'undo'; label: string; description?: string }
		| { kind: 'redo'; label: string; description?: string }
		| { kind: 'find'; label: string; description?: string }
		| { kind: 'format'; label: string; description?: string };

	interface Props {
		expanded: boolean;
		disabled?: boolean;
		dirty?: boolean;
		canUndo?: boolean;
		canRedo?: boolean;
		editorView: EditorView | null;
		onToggle: () => void;
		onSave: () => void;
		onFormat?: () => void;
	}

	let {
		expanded,
		disabled = false,
		dirty = false,
		canUndo = true,
		canRedo = true,
		editorView,
		onToggle,
		onSave,
		onFormat
	}: Props = $props();

	let suppressClick = $state(false);

	const actions = $derived.by(() => {
		const primary: EditorAction[] = [
			{ kind: 'save', label: dirty ? 'Save' : 'Save', description: 'Save file (Ctrl+S)' },
			{ kind: 'undo', label: 'Undo', description: 'Undo (Ctrl+Z)' },
			{ kind: 'redo', label: 'Redo', description: 'Redo (Ctrl+Shift+Z)' },
			{ kind: 'find', label: 'Find', description: 'Find (Ctrl+F)' }
		];
		if (onFormat) {
			primary.push({ kind: 'format', label: 'Format', description: 'Format code' });
		}
		return { primary };
	});

	function handleTogglePointerDown(e: PointerEvent): void {
		e.preventDefault();
		suppressClick = true;
		queueMicrotask(() => {
			suppressClick = false;
		});
		onToggle();
	}

	function executeAction(action: EditorAction): void {
		if (disabled || !editorView) return;

		switch (action.kind) {
			case 'save':
				onSave();
				break;
			case 'undo':
				undo(editorView);
				break;
			case 'redo':
				redo(editorView);
				break;
			case 'find':
				openSearchPanel(editorView);
				break;
			case 'format':
				onFormat?.();
				break;
		}

		queueMicrotask(() => editorView?.focus());
	}

	function handleActionPointerDown(e: PointerEvent, action: EditorAction): void {
		e.preventDefault();
		suppressClick = true;
		queueMicrotask(() => {
			suppressClick = false;
		});
		if (disabled) return;
		executeAction(action);
	}

	function isActionDisabled(action: EditorAction): boolean {
		if (disabled) return true;
		if (action.kind === 'undo' && !canUndo) return true;
		if (action.kind === 'redo' && !canRedo) return true;
		return false;
	}
</script>

<div
	class="w-full border-t border-panel-border bg-panel-bg/95 backdrop-blur supports-[backdrop-filter]:bg-panel-bg/80"
	style="padding-bottom: var(--effective-safe-area-bottom, 0px);"
>
	<!-- Collapsed: Show quick-access actions + expand handle -->
	{#if !expanded}
		<div class="flex items-center gap-2 px-2 py-1.5 touch-device:gap-3">
			<!-- Save button with dirty indicator -->
			<button
				class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors p-2 touch-device:p-2.5 disabled:opacity-40 flex items-center justify-center relative"
				disabled={disabled}
				title="Save file (Ctrl+S)"
				aria-label="Save"
				onpointerdown={(e) => handleActionPointerDown(e, { kind: 'save', label: 'Save' })}
				onclick={() => {
					if (!suppressClick && !disabled) executeAction({ kind: 'save', label: 'Save' });
				}}
			>
				<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
				</svg>
				{#if dirty}
					<span class="absolute top-1 right-1 w-1.5 h-1.5 rounded-full bg-accent"></span>
				{/if}
			</button>
			<button
				class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors p-2 touch-device:p-2.5 disabled:opacity-40 flex items-center justify-center"
				disabled={disabled || !canUndo}
				title="Undo (Ctrl+Z)"
				aria-label="Undo"
				onpointerdown={(e) => handleActionPointerDown(e, { kind: 'undo', label: 'Undo' })}
				onclick={() => {
					if (!suppressClick && !disabled && canUndo) executeAction({ kind: 'undo', label: 'Undo' });
				}}
			>
				<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" />
				</svg>
			</button>
			<button
				class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors p-2 touch-device:p-2.5 disabled:opacity-40 flex items-center justify-center"
				disabled={disabled || !canRedo}
				title="Redo (Ctrl+Shift+Z)"
				aria-label="Redo"
				onpointerdown={(e) => handleActionPointerDown(e, { kind: 'redo', label: 'Redo' })}
				onclick={() => {
					if (!suppressClick && !disabled && canRedo) executeAction({ kind: 'redo', label: 'Redo' });
				}}
			>
				<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 10h-10a8 8 0 00-8 8v2M21 10l-6 6m6-6l-6-6" />
				</svg>
			</button>
			<button
				class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors p-2 touch-device:p-2.5 disabled:opacity-40 flex items-center justify-center"
				disabled={disabled}
				title="Find (Ctrl+F)"
				aria-label="Find"
				onpointerdown={(e) => handleActionPointerDown(e, { kind: 'find', label: 'Find' })}
				onclick={() => {
					if (!suppressClick && !disabled) executeAction({ kind: 'find', label: 'Find' });
				}}
			>
				<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
				</svg>
			</button>

			<div class="flex-1"></div>

			{#if disabled}
				<span class="text-xs text-gray-400 mr-2">No file</span>
			{/if}

			<!-- Expand button -->
			<button
				class="flex items-center gap-1 rounded px-2 py-1.5 text-xs text-gray-300 hover:text-white hover:bg-white/10 transition-colors touch-device:px-3 touch-device:py-2"
				aria-label="Show more actions"
				aria-expanded={expanded}
				onpointerdown={handleTogglePointerDown}
				onclick={() => {
					if (!suppressClick) onToggle();
				}}
			>
				<span>More</span>
				<svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
				</svg>
			</button>
		</div>
	{:else}
		<!-- Expanded: Full actions panel -->
		<div class="px-2 py-2">
			<!-- Header with collapse button -->
			<div class="flex items-center justify-between mb-2">
				<span class="text-xs text-gray-400 uppercase tracking-wide">Editor Actions</span>
				<button
					class="flex items-center gap-1 rounded px-2 py-1 text-xs text-gray-300 hover:text-white hover:bg-white/10 transition-colors"
					aria-label="Collapse actions"
					aria-expanded={expanded}
					onpointerdown={handleTogglePointerDown}
					onclick={() => {
						if (!suppressClick) onToggle();
					}}
				>
					<span>Less</span>
					<svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
					</svg>
				</button>
			</div>

			<!-- All actions -->
			<div class="flex gap-1.5 flex-wrap" role="toolbar" aria-label="Editor actions">
				{#each actions.primary as action (action.label)}
					<button
						class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors p-2 touch-device:p-2.5 disabled:opacity-40 flex items-center justify-center relative"
						disabled={isActionDisabled(action)}
						title={action.description ?? action.label}
						aria-label={action.description ? `${action.label} (${action.description})` : action.label}
						onpointerdown={(e) => handleActionPointerDown(e, action)}
						onclick={() => {
							if (!suppressClick && !isActionDisabled(action)) executeAction(action);
						}}
					>
						{#if action.kind === 'save'}
							<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
							</svg>
							{#if dirty}
								<span class="absolute top-1 right-1 w-1.5 h-1.5 rounded-full bg-accent"></span>
							{/if}
						{:else if action.kind === 'undo'}
							<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" />
							</svg>
						{:else if action.kind === 'redo'}
							<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 10h-10a8 8 0 00-8 8v2M21 10l-6 6m6-6l-6-6" />
							</svg>
						{:else if action.kind === 'find'}
							<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
							</svg>
						{:else if action.kind === 'format'}
							<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16m-7 6h7" />
							</svg>
						{/if}
					</button>
				{/each}
			</div>
		</div>
	{/if}
</div>
