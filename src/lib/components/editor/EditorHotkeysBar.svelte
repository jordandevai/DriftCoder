<script lang="ts">
	import type { EditorView } from '@codemirror/view';
	import { undo, redo } from '@codemirror/commands';
	import { openSearchPanel } from '@codemirror/search';

	interface Props {
		disabled?: boolean;
		dirty?: boolean;
		canUndo?: boolean;
		canRedo?: boolean;
		editorView: EditorView | null;
		onSave: () => void;
	}

	let {
		disabled = false,
		dirty = false,
		canUndo = true,
		canRedo = true,
		editorView,
		onSave
	}: Props = $props();

	let suppressClick = $state(false);

	function executeAction(kind: 'save' | 'undo' | 'redo' | 'find'): void {
		if (disabled || !editorView) return;

		switch (kind) {
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
		}

		queueMicrotask(() => editorView?.focus());
	}

	function handlePointerDown(e: PointerEvent, kind: 'save' | 'undo' | 'redo' | 'find'): void {
		e.preventDefault();
		suppressClick = true;
		queueMicrotask(() => {
			suppressClick = false;
		});
		if (disabled) return;
		if (kind === 'undo' && !canUndo) return;
		if (kind === 'redo' && !canRedo) return;
		executeAction(kind);
	}
</script>

<div
	class="w-full border-t border-panel-border bg-panel-bg/95 backdrop-blur supports-[backdrop-filter]:bg-panel-bg/80"
	style="padding-bottom: var(--effective-safe-area-bottom, 0px);"
>
	<div class="flex items-center gap-2 px-2 py-1.5 touch-device:gap-3">
		<button
			class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2.5 py-1.5 touch-device:px-3 touch-device:py-2 disabled:opacity-40 flex items-center gap-1.5"
			disabled={disabled}
			title="Save file (Ctrl+S)"
			onpointerdown={(e) => handlePointerDown(e, 'save')}
			onclick={() => {
				if (!suppressClick && !disabled) executeAction('save');
			}}
		>
			<svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
			</svg>
			<span>Save</span>
			{#if dirty}
				<span class="w-1.5 h-1.5 rounded-full bg-accent"></span>
			{/if}
		</button>
		<button
			class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2.5 py-1.5 touch-device:px-3 touch-device:py-2 disabled:opacity-40 flex items-center gap-1.5"
			disabled={disabled || !canUndo}
			title="Undo (Ctrl+Z)"
			onpointerdown={(e) => handlePointerDown(e, 'undo')}
			onclick={() => {
				if (!suppressClick && !disabled && canUndo) executeAction('undo');
			}}
		>
			<svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" />
			</svg>
			<span>Undo</span>
		</button>
		<button
			class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2.5 py-1.5 touch-device:px-3 touch-device:py-2 disabled:opacity-40 flex items-center gap-1.5"
			disabled={disabled || !canRedo}
			title="Redo (Ctrl+Shift+Z)"
			onpointerdown={(e) => handlePointerDown(e, 'redo')}
			onclick={() => {
				if (!suppressClick && !disabled && canRedo) executeAction('redo');
			}}
		>
			<svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 10h-10a8 8 0 00-8 8v2M21 10l-6 6m6-6l-6-6" />
			</svg>
			<span>Redo</span>
		</button>
		<button
			class="shrink-0 rounded bg-white/10 hover:bg-white/20 transition-colors text-xs px-2.5 py-1.5 touch-device:px-3 touch-device:py-2 disabled:opacity-40 flex items-center gap-1.5"
			disabled={disabled}
			title="Find (Ctrl+F)"
			onpointerdown={(e) => handlePointerDown(e, 'find')}
			onclick={() => {
				if (!suppressClick && !disabled) executeAction('find');
			}}
		>
			<svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
			</svg>
			<span>Find</span>
		</button>

		<div class="flex-1"></div>

		{#if disabled}
			<span class="text-xs text-gray-400">No file</span>
		{/if}
	</div>
</div>
