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
	import EditorHotkeysBar from '$components/editor/EditorHotkeysBar.svelte';

	interface Props {
		filePath: string;
	}

	let { filePath }: Props = $props();

	let editorContainer = $state<HTMLDivElement | null>(null);
	let editorView = $state<EditorView | null>(null);
	let currentLanguage = $state<string | null>(null);
	let suppressStoreUpdate = false;
	let languageLoadVersion = 0;
	let debugLogs = $state<string[]>([]);

	function debugLog(msg: string) {
		debugLogs = [...debugLogs.slice(-19), msg];
		console.log(msg);
	}

	const file = $derived($fileStore.openFiles.get(filePath));
	const wordWrap = $derived($settingsStore.wordWrap);
	const fontSize = $derived($settingsStore.fontSize ?? 14);
	const languageCompartment = new Compartment();
	const wrapCompartment = new Compartment();
	const fontCompartment = new Compartment();

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
		},
		// Search panel styles
		'.cm-panel': {
			backgroundColor: 'rgb(var(--c-panel-bg))',
			color: 'rgb(var(--c-editor-fg))',
			fontFamily: 'var(--font-sans)',
			fontSize: '12px'
		},
		'.cm-panel.cm-search': {
			padding: '10px 12px',
			borderTop: '1px solid rgb(var(--c-panel-border))',
			display: 'flex',
			flexWrap: 'wrap',
			alignItems: 'center',
			gap: '6px 8px',
			position: 'relative'
		},
		// Use <br> as a flex line break for two-row layout
		'.cm-panel.cm-search br': {
			display: 'block',
			width: '100%',
			height: '0',
			margin: '2px 0'
		},
		'.cm-panel.cm-search label': {
			display: 'inline-flex',
			alignItems: 'center',
			gap: '4px',
			color: 'rgba(var(--c-editor-fg), 0.7)',
			fontSize: '11px',
			cursor: 'pointer',
			userSelect: 'none',
			padding: '4px 6px',
			borderRadius: '4px',
			transition: 'background-color 150ms'
		},
		'.cm-panel.cm-search label:hover': {
			backgroundColor: 'rgba(255, 255, 255, 0.05)'
		},
		'.cm-panel.cm-search input[type="checkbox"]': {
			accentColor: 'rgb(var(--c-accent))',
			width: '13px',
			height: '13px',
			cursor: 'pointer'
		},
		'.cm-textfield': {
			backgroundColor: 'rgb(var(--c-editor-bg))',
			border: '1px solid rgb(var(--c-panel-border))',
			borderRadius: '4px',
			padding: '7px 12px',
			color: 'rgb(var(--c-editor-fg))',
			fontSize: '12px',
			outline: 'none',
			transition: 'border-color 150ms, box-shadow 150ms',
			width: '200px',
			flexShrink: '0'
		},
		'.cm-textfield:focus': {
			borderColor: 'rgb(var(--c-accent))',
			boxShadow: '0 0 0 1px rgb(var(--c-accent))'
		},
		'.cm-textfield::placeholder': {
			color: 'rgb(107 114 128)'
		},
		'.cm-button, .cm-panel.cm-search button': {
			backgroundColor: 'rgb(var(--c-panel-active)) !important',
			border: '1px solid rgb(var(--c-panel-border)) !important',
			borderRadius: '4px',
			padding: '6px 12px',
			color: 'rgb(var(--c-editor-fg)) !important',
			fontSize: '11px',
			fontWeight: '500',
			cursor: 'pointer',
			transition: 'background-color 150ms, border-color 150ms',
			backgroundImage: 'none !important'
		},
		'.cm-button:hover, .cm-panel.cm-search button:hover': {
			backgroundColor: 'rgb(var(--c-accent)) !important',
			borderColor: 'rgb(var(--c-accent)) !important'
		},
		'.cm-button:active, .cm-panel.cm-search button:active': {
			backgroundColor: 'rgb(var(--c-accent-hover)) !important'
		},
		'.cm-panel.cm-search button[name="close"]': {
			position: 'absolute',
			top: '10px',
			right: '8px',
			backgroundColor: 'transparent !important',
			border: 'none !important',
			borderRadius: '4px',
			padding: '4px 8px',
			color: 'rgb(var(--c-editor-fg)) !important',
			fontSize: '18px',
			lineHeight: '1',
			cursor: 'pointer',
			opacity: '0.5'
		},
		'.cm-panel.cm-search button[name="close"]:hover': {
			backgroundColor: 'rgba(255, 255, 255, 0.1) !important',
			opacity: '1'
		},
		'.cm-searchMatch': {
			backgroundColor: 'rgba(var(--c-warning), 0.3)',
			borderRadius: '2px'
		},
		'.cm-searchMatch-selected': {
			backgroundColor: 'rgba(var(--c-warning), 0.6)'
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

		// Log focus events on the editor
		editorView.contentDOM.addEventListener('focus', () => {
			debugLog(`[FOCUS] scroll: ${editorView!.scrollDOM.scrollTop}`);
		});

		// Save scroll position on scroll (debounced)
		let scrollTimeout: ReturnType<typeof setTimeout> | null = null;
		let lastScrollTop = editorView.scrollDOM.scrollTop;
		
		const onScroll = () => {
			if (!editorView) return;
			const newScrollTop = editorView.scrollDOM.scrollTop;
			const delta = newScrollTop - lastScrollTop;
			// Only log significant jumps that aren't obviously user scrolling
			if (Math.abs(delta) > 100) {
				// We don't log here anymore to reduce noise, the ResizeObserver handles the fix
			}
			lastScrollTop = newScrollTop;

			if (scrollTimeout) clearTimeout(scrollTimeout);
			scrollTimeout = setTimeout(() => {
				if (editorView) {
					fileStore.setScrollPosition(filePath, editorView.scrollDOM.scrollTop);
				}
			}, 150);
		};

		editorView.scrollDOM.addEventListener('scroll', onScroll);

		// Watch for resizing (Android soft keyboard) and restore scroll if it jumps
		const resizeObserver = new ResizeObserver(() => {
			if (!editorView) return;
			const currentScroll = editorView.scrollDOM.scrollTop;
			// If scroll jumped to ~0 (or significantly changed) during a resize, restore it
			if (Math.abs(currentScroll - lastScrollTop) > 50 && lastScrollTop > 0) {
				debugLog(`[RESIZE-FIX] Restoring ${currentScroll} -> ${lastScrollTop}`);
				editorView.scrollDOM.scrollTop = lastScrollTop;
			}
		});
		resizeObserver.observe(editorContainer!);

		// Cleanup function attached to the view's destruction is hard to hook directly here
		// without extra state, so we register a destroy handler on the view itself or return cleanup.
		// Since createEditor doesn't return cleanup, we'll patch the destroy method.
		const originalDestroy = editorView.destroy.bind(editorView);
		editorView.destroy = () => {
			resizeObserver.disconnect();
			if (editorView) {
				editorView.scrollDOM.removeEventListener('scroll', onScroll);
			}
			originalDestroy();
		};
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
		debugLog(`[SYNC] preserving scroll: ${scrollTop}`);
		const selection = editorView.state.selection;
		suppressStoreUpdate = true;
		editorView.dispatch({
			changes: { from: 0, to: editorView.state.doc.length, insert: content },
			selection: {
				anchor: Math.min(selection.main.anchor, content.length),
				head: Math.min(selection.main.head, content.length)
			},
			scrollIntoView: false
		});
		suppressStoreUpdate = false;
		requestAnimationFrame(() => {
			if (editorView) {
				debugLog(`[SYNC] restoring: ${scrollTop}, was: ${editorView.scrollDOM.scrollTop}`);
				editorView.scrollDOM.scrollTop = scrollTop;
			}
		});
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
			debugLog('[EFFECT] content diverged');
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

	<!-- Debug overlay -->
	{#if debugLogs.length > 0}
		<div class="absolute top-0 right-0 bg-black/80 text-green-400 text-[10px] font-mono p-2 max-w-[50%] max-h-[40%] overflow-auto z-50 pointer-events-none">
			{#each debugLogs as log}
				<div>{log}</div>
			{/each}
		</div>
	{/if}

	{#if !file}
		<div class="absolute inset-0 flex items-center justify-center text-gray-500">
			<p>Loading file...</p>
		</div>
	{/if}

	<!-- Editor actions bar -->
	<EditorHotkeysBar
		disabled={!file}
		dirty={file?.dirty ?? false}
		{editorView}
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