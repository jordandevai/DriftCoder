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
				class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-3 py-2 touch-device:px-4 touch-device:py-2.5 disabled:opacity-40 flex items-center gap-1.5"
				disabled={disabled}
				title="Save file (Ctrl+S)"
				onpointerdown={(e) => handleActionPointerDown(e, { kind: 'save', label: 'Save' })}
				onclick={() => {
					if (!suppressClick && !disabled) executeAction({ kind: 'save', label: 'Save' });
				}}
			>
				Save
				{#if dirty}
					<span class="w-1.5 h-1.5 rounded-full bg-accent-primary"></span>
				{/if}
			</button>
			<button
				class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-3 py-2 touch-device:px-4 touch-device:py-2.5 disabled:opacity-40"
				disabled={disabled || !canUndo}
				title="Undo (Ctrl+Z)"
				onpointerdown={(e) => handleActionPointerDown(e, { kind: 'undo', label: 'Undo' })}
				onclick={() => {
					if (!suppressClick && !disabled && canUndo) executeAction({ kind: 'undo', label: 'Undo' });
				}}
			>
				Undo
			</button>
			<button
				class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-3 py-2 touch-device:px-4 touch-device:py-2.5 disabled:opacity-40"
				disabled={disabled || !canRedo}
				title="Redo (Ctrl+Shift+Z)"
				onpointerdown={(e) => handleActionPointerDown(e, { kind: 'redo', label: 'Redo' })}
				onclick={() => {
					if (!suppressClick && !disabled && canRedo) executeAction({ kind: 'redo', label: 'Redo' });
				}}
			>
				Redo
			</button>
			<button
				class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-3 py-2 touch-device:px-4 touch-device:py-2.5 disabled:opacity-40"
				disabled={disabled}
				title="Find (Ctrl+F)"
				onpointerdown={(e) => handleActionPointerDown(e, { kind: 'find', label: 'Find' })}
				onclick={() => {
					if (!suppressClick && !disabled) executeAction({ kind: 'find', label: 'Find' });
				}}
			>
				Find
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
						class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2 py-1.5 touch-device:text-sm touch-device:px-3 touch-device:py-2 disabled:opacity-40 flex items-center gap-1.5"
						disabled={isActionDisabled(action)}
						title={action.description ?? action.label}
						aria-label={action.description ? `${action.label} (${action.description})` : action.label}
						onpointerdown={(e) => handleActionPointerDown(e, action)}
						onclick={() => {
							if (!suppressClick && !isActionDisabled(action)) executeAction(action);
						}}
					>
						{action.label}
						{#if action.kind === 'save' && dirty}
							<span class="w-1.5 h-1.5 rounded-full bg-accent-primary"></span>
						{/if}
					</button>
				{/each}
			</div>
		</div>
	{/if}
</div>
