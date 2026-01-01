<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { fileStore } from '$stores/files';
	import { settingsStore } from '$stores/settings';
	import { EditorState } from '@codemirror/state';
	import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter } from '@codemirror/view';
	import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
	import { bracketMatching, indentOnInput, syntaxHighlighting, defaultHighlightStyle } from '@codemirror/language';
	import { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
	import { autocompletion, completionKeymap } from '@codemirror/autocomplete';
	import { javascript } from '@codemirror/lang-javascript';
	import { python } from '@codemirror/lang-python';
	import { rust } from '@codemirror/lang-rust';
	import { html } from '@codemirror/lang-html';
	import { css } from '@codemirror/lang-css';
	import { json } from '@codemirror/lang-json';
	import { markdown } from '@codemirror/lang-markdown';
	import { yaml } from '@codemirror/lang-yaml';
	import { xml } from '@codemirror/lang-xml';
	import { sql } from '@codemirror/lang-sql';

	interface Props {
		filePath: string;
	}

	let { filePath }: Props = $props();

	let editorContainer = $state<HTMLDivElement | null>(null);
	let editorView: EditorView | null = null;

	const file = $derived($fileStore.openFiles.get(filePath));

	// Language extensions map
	const languageExtensions: Record<string, () => any> = {
		javascript: javascript,
		typescript: () => javascript({ typescript: true }),
		jsx: () => javascript({ jsx: true }),
		tsx: () => javascript({ jsx: true, typescript: true }),
		python: python,
		rust: rust,
		html: html,
		css: css,
		json: json,
		markdown: markdown,
		yaml: yaml,
		xml: xml,
		sql: sql
	};

	// Dark theme
	const darkTheme = EditorView.theme({
		'&': {
			height: '100%',
			backgroundColor: '#1e1e1e',
			color: '#d4d4d4'
		},
		'.cm-content': {
			fontFamily: 'var(--font-mono)',
			caretColor: '#aeafad'
		},
		'.cm-cursor': {
			borderLeftColor: '#aeafad'
		},
		'&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
			backgroundColor: '#264f78'
		},
		'.cm-gutters': {
			backgroundColor: '#1e1e1e',
			color: '#858585',
			border: 'none'
		},
		'.cm-activeLineGutter': {
			backgroundColor: '#2d2d2d'
		},
		'.cm-activeLine': {
			backgroundColor: '#2d2d2d'
		},
		'.cm-line': {
			padding: '0 8px'
		}
	});

	function getLanguageExtension(language: string) {
		const factory = languageExtensions[language];
		return factory ? factory() : [];
	}

	function createEditor(content: string, language: string) {
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
				if (update.docChanged && file) {
					const newContent = update.state.doc.toString();
					fileStore.updateFileContent(filePath, newContent);
				}
			}),
			getLanguageExtension(language)
		];

		const state = EditorState.create({
			doc: content,
			extensions
		});

		editorView = new EditorView({
			state,
			parent: editorContainer!
		});
	}

	async function handleSave() {
		try {
			await fileStore.saveFile(filePath);
		} catch (error) {
			if (error instanceof Error && error.message === 'CONFLICT') {
				// TODO: Show conflict resolution modal
				alert('File has been modified on the server. Please resolve the conflict.');
			} else {
				console.error('Save failed:', error);
			}
		}
	}

	onMount(() => {
		if (file) {
			createEditor(file.content, file.language);
		}
	});

	onDestroy(() => {
		if (editorView) {
			editorView.destroy();
		}
	});

	// Recreate editor when file changes
	$effect(() => {
		if (file && editorContainer) {
			// Only recreate if content is different from what's in the editor
			const currentContent = editorView?.state.doc.toString();
			if (currentContent !== file.content) {
				createEditor(file.content, file.language);
			}
		}
	});
</script>

<div class="h-full w-full overflow-hidden">
	{#if file}
		<div bind:this={editorContainer} class="h-full w-full"></div>
	{:else}
		<div class="h-full flex items-center justify-center text-gray-500">
			<p>Loading file...</p>
		</div>
	{/if}
</div>

<style>
	:global(.cm-editor) {
		height: 100%;
	}
	:global(.cm-scroller) {
		overflow: auto;
	}
</style>
