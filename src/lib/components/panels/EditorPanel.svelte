<script lang="ts">
	import { onDestroy } from 'svelte';
	import { fileStore } from '$stores/files';
	import { settingsStore } from '$stores/settings';
	import { Compartment, EditorState, type Extension } from '@codemirror/state';
	import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter } from '@codemirror/view';
	import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
	import { bracketMatching, indentOnInput, syntaxHighlighting, defaultHighlightStyle } from '@codemirror/language';
	import { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
	import { autocompletion, completionKeymap } from '@codemirror/autocomplete';
	import { notificationsStore } from '$stores/notifications';
	import { conflictStore } from '$stores/conflict';
	import { loadLanguageExtension } from '$utils/codemirror-languages';
	import Button from '$components/shared/Button.svelte';
	import { confirmStore } from '$stores/confirm';
	import { promptStore } from '$stores/prompt';
	import { workspaceStore, activeSession } from '$stores/workspace';
	import EditorHotkeysBar from '$components/editor/EditorHotkeysBar.svelte';

	interface Props {
		filePath: string;
	}

	let { filePath }: Props = $props();

	let editorContainer = $state<HTMLDivElement | null>(null);
	let editorView: EditorView | null = null;
	let currentLanguage = $state<string | null>(null);
	let suppressStoreUpdate = false;
	let languageLoadVersion = 0;

	const file = $derived($fileStore.openFiles.get(filePath));
	const wordWrap = $derived($settingsStore.wordWrap);
	const fontSize = $derived($settingsStore.fontSize ?? 14);
	const languageCompartment = new Compartment();
	const wrapCompartment = new Compartment();
	const fontCompartment = new Compartment();

	// Touch device detection for hotkeys bar auto-show
	function isCoarsePointer(): boolean {
		if (typeof window === 'undefined') return false;
		try {
			return window.matchMedia?.('(pointer: coarse)')?.matches ?? false;
		} catch {
			return false;
		}
	}

	// Store-backed expanded state for editor hotkeys (auto-show on touch devices)
	const hotkeysExpanded = $derived.by(() => {
		const session = $activeSession;
		if (!session) return isCoarsePointer();
		const stored = session.editorHotkeysExpandedByPath?.[filePath];
		if (typeof stored === 'boolean') return stored;
		return isCoarsePointer();
	});

	function toggleHotkeys(): void {
		const session = $activeSession;
		if (!session) return;
		workspaceStore.setEditorHotkeysExpanded(session.id, filePath, !hotkeysExpanded);
		queueMicrotask(() => editorView?.focus());
	}

	// Dark theme
	const darkTheme = EditorView.theme({
		'&': {
			height: '100%',
			backgroundColor: 'rgb(var(--c-editor-bg))',
			color: 'rgb(var(--c-editor-fg))'
		},
		'.cm-content': {
			fontFamily: 'var(--font-mono)',
			caretColor: 'rgb(var(--c-editor-cursor))'
		},
		'.cm-cursor': {
			borderLeftColor: 'rgb(var(--c-editor-cursor))'
		},
		'&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
			backgroundColor: 'rgb(var(--c-editor-selection))'
		},
		'.cm-gutters': {
			backgroundColor: 'rgb(var(--c-editor-bg))',
			color: '#858585',
			border: 'none'
		},
		'.cm-activeLineGutter': {
			backgroundColor: 'rgb(var(--c-editor-line))'
		},
		'.cm-activeLine': {
			backgroundColor: 'rgb(var(--c-editor-line))'
		},
		'.cm-line': {
			padding: '0 8px'
		}
	});

	async function applyLanguage(language: string) {
		const version = ++languageLoadVersion;
		const extension = await loadLanguageExtension(language);
		if (version !== languageLoadVersion) return;
		if (!editorView) return;

		editorView.dispatch({
			effects: languageCompartment.reconfigure(extension)
		});
		currentLanguage = language;
	}

	function createEditor(content: string, language: string) {
		if (!editorContainer) return;

		if (editorView) {
			editorView.destroy();
		}

		const extensions = [
			lineNumbers(),
			highlightActiveLineGutter(),
			highlightActiveLine(),
			history(),
			indentOnInput(),
			bracketMatching(),
			highlightSelectionMatches(),
			autocompletion(),
			syntaxHighlighting(defaultHighlightStyle),
			darkTheme,
			keymap.of([
				...defaultKeymap,
				...historyKeymap,
				...searchKeymap,
				...completionKeymap,
				{
					key: 'Mod-s',
					run: () => {
						handleSave();
						return true;
					}
				}
			]),
			EditorView.updateListener.of((update) => {
				if (suppressStoreUpdate) return;
				if (update.docChanged && file) {
					const newContent = update.state.doc.toString();
					fileStore.updateFileContent(filePath, newContent);
				}
			}),
			languageCompartment.of([]),
			wrapCompartment.of(wordWrap ? EditorView.lineWrapping : []),
			fontCompartment.of(
				EditorView.theme({
					'&': { fontSize: `${fontSize}px` },
					'.cm-content': { fontSize: `${fontSize}px` }
				})
			)
		];

		const state = EditorState.create({
			doc: content,
			extensions
		});

		editorView = new EditorView({
			state,
			parent: editorContainer!
		});

		currentLanguage = null;
		applyLanguage(language);

		// Restore saved scroll position after editor is ready
		queueMicrotask(() => {
			if (editorView) {
				const savedScroll = fileStore.getScrollPosition(filePath);
				if (savedScroll > 0) {
					editorView.scrollDOM.scrollTop = savedScroll;
				}
			}
		});

		// Save scroll position on scroll (debounced)
		let scrollTimeout: ReturnType<typeof setTimeout> | null = null;
		editorView.scrollDOM.addEventListener('scroll', () => {
			if (scrollTimeout) clearTimeout(scrollTimeout);
			scrollTimeout = setTimeout(() => {
				if (editorView) {
					fileStore.setScrollPosition(filePath, editorView.scrollDOM.scrollTop);
				}
			}, 150);
		});
	}

	async function handleSave() {
		try {
			await fileStore.saveFile(filePath);
		} catch (error) {
			if (error instanceof Error && error.message === 'CONFLICT') {
				conflictStore.open(filePath);
				notificationsStore.notify({
					severity: 'warning',
					title: 'Save Conflict',
					message:
						'This file changed on the server since it was opened. Your local changes are still in the editor.',
					detail: `File: ${filePath}`,
					actions: [
						{
							label: 'Resolve',
							run: () => conflictStore.open(filePath)
						}
					]
				});
			} else if (error instanceof Error && error.message === 'MISSING') {
				const newPath = await promptStore.prompt({
					title: 'File Missing on Server',
					message:
						'This file no longer exists at its original path (it may have been renamed or deleted).\n\n' +
						'Choose a new path to save your current buffer (prevents recreating a ghost copy at the old path).',
					placeholder: '/path/to/new-file',
					initialValue: filePath,
					confirmText: 'Save As',
					cancelText: 'Cancel'
				});
				if (newPath && newPath !== filePath) {
					await fileStore.saveFileAs(filePath, newPath);
				}
			} else {
				console.error('Save failed:', error);
				notificationsStore.notify({
					severity: 'error',
					title: 'Save Failed',
					message: `Failed to save ${filePath}.`,
					detail: error instanceof Error ? error.message : String(error)
				});
			}
		}
	}

	async function handleReloadFromServer() {
		const confirmed = await confirmStore.confirm({
			title: 'Reload From Server',
			message: 'Discard local changes and reload the latest server version?',
			confirmText: 'Reload',
			cancelText: 'Cancel',
			destructive: true
		});
		if (!confirmed) return;
		await fileStore.reloadFileFromRemote(filePath);
	}

	onDestroy(() => {
		if (editorView) {
			// Save scroll position before destroying
			fileStore.setScrollPosition(filePath, editorView.scrollDOM.scrollTop);
			editorView.destroy();
		}
	});

	function setEditorContent(content: string) {
		if (!editorView) return;
		const scrollTop = editorView.scrollDOM.scrollTop;
		const selection = editorView.state.selection;
		suppressStoreUpdate = true;
		editorView.dispatch({
			changes: { from: 0, to: editorView.state.doc.length, insert: content },
			selection: {
				anchor: Math.min(selection.main.anchor, content.length),
				head: Math.min(selection.main.head, content.length)
			}
		});
		suppressStoreUpdate = false;
		editorView.scrollDOM.scrollTop = scrollTop;
	}

	// Keep editor instance stable across reactivity changes
	$effect(() => {
		if (!editorContainer) return;

		// If backing file is unavailable, tear down any existing editor
		if (!file) {
			if (editorView) {
				editorView.destroy();
				editorView = null;
				currentLanguage = null;
			}
			return;
		}

		// Ensure we have a live editor instance attached to the current container
		if (!editorView || editorView.dom.parentElement !== editorContainer) {
			createEditor(file.content, file.language);
			return;
		}

		// Update language mode without recreating the editor
		if (currentLanguage !== file.language) {
			applyLanguage(file.language);
		}

		// Sync document content if it diverges from store state
		const currentContent = editorView.state.doc.toString();
		if (currentContent !== file.content) {
			setEditorContent(file.content);
		}
	});

	// React to wordWrap setting changes
	$effect(() => {
		if (!editorView) return;
		editorView.dispatch({
			effects: wrapCompartment.reconfigure(wordWrap ? EditorView.lineWrapping : [])
		});
	});

	$effect(() => {
		if (!editorView) return;
		editorView.dispatch({
			effects: fontCompartment.reconfigure(
				EditorView.theme({
					'&': { fontSize: `${fontSize}px` },
					'.cm-content': { fontSize: `${fontSize}px` }
				})
			)
		});
	});
</script>

<div class="h-full w-full overflow-hidden relative flex flex-col">
	{#if file?.remoteChanged}
		<div class="flex items-center justify-between gap-2 px-3 py-2 text-xs border-b border-panel-border bg-warning/10">
			<div class="text-gray-200">Server version changed while you were editing.</div>
			<div class="flex items-center gap-2">
				<Button size="sm" variant="ghost" onclick={() => conflictStore.open(filePath)}>Compare</Button>
				<Button size="sm" variant="ghost" onclick={handleReloadFromServer}>Reload Server</Button>
			</div>
		</div>
	{/if}
	<div bind:this={editorContainer} class="flex-1 h-full w-full min-h-0"></div>
	{#if !file}
		<div class="absolute inset-0 flex items-center justify-center text-gray-500">
			<p>Loading file...</p>
		</div>
	{/if}

	<!-- Touch-friendly editor actions bar -->
	<EditorHotkeysBar
		expanded={hotkeysExpanded}
		disabled={!file}
		dirty={file?.dirty ?? false}
		{editorView}
		onToggle={toggleHotkeys}
		onSave={handleSave}
	/>
</div>

<style>
	:global(.cm-editor) {
		height: 100%;
	}
	:global(.cm-scroller) {
		overflow: auto;
	}
</style>
