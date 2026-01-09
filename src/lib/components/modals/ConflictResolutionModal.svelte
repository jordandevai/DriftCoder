<script lang="ts">
	import { onDestroy } from 'svelte';
	import Modal from '$components/shared/Modal.svelte';
	import Button from '$components/shared/Button.svelte';
	import { conflictStore } from '$stores/conflict';
	import { fileStore } from '$stores/files';
	import { activeSession } from '$stores/workspace';
	import { confirmStore } from '$stores/confirm';
	import { notificationsStore } from '$stores/notifications';
	import { unifiedMergeView } from '@codemirror/merge';
	import { Compartment, EditorState } from '@codemirror/state';
	import { EditorView, lineNumbers } from '@codemirror/view';
	import { syntaxHighlighting, defaultHighlightStyle } from '@codemirror/language';
	import { loadLanguageExtension } from '$utils/codemirror-languages';

	const isOpen = $derived($conflictStore.open);
	const filePath = $derived($conflictStore.filePath);
	const conflictSessionId = $derived($conflictStore.sessionId);
	const currentSessionId = $derived($activeSession?.id ?? null);
	const openFile = $derived(filePath ? $fileStore.openFiles.get(filePath) ?? null : null);

	let editorContainer = $state<HTMLDivElement | null>(null);
	let editorView: EditorView | null = null;
	let remoteContent = $state<string>('');
	let remoteMtime = $state<number | null>(null);
	let loading = $state(false);

	const languageCompartment = new Compartment();
	let languageLoadVersion = 0;

	const mergeTheme = EditorView.theme({
		'&': {
			height: '100%',
			backgroundColor: 'rgb(var(--c-editor-bg))',
			color: 'rgb(var(--c-editor-fg))'
		},
		'.cm-content': {
			fontFamily: 'var(--font-mono)',
			caretColor: 'rgb(var(--c-editor-cursor))'
		},
		'.cm-gutters': {
			backgroundColor: 'rgb(var(--c-editor-bg))',
			color: 'rgb(var(--c-editor-fg) / 0.6)',
			border: 'none'
		}
	});

	async function loadRemote() {
		if (!filePath) return;
		loading = true;
		try {
			const remote = await fileStore.fetchRemoteFile(filePath);
			remoteContent = remote.content;
			remoteMtime = remote.mtime;
		} catch (error) {
			notificationsStore.notify({
				severity: 'error',
				title: 'Load Remote File Failed',
				message: `Could not load the latest server version for ${filePath}.`,
				detail: error instanceof Error ? error.message : String(error)
			});
			remoteContent = '';
			remoteMtime = null;
		} finally {
			loading = false;
		}
	}

	async function applyLanguage(language: string) {
		const version = ++languageLoadVersion;
		const ext = await loadLanguageExtension(language);
		if (version !== languageLoadVersion) return;
		if (!editorView) return;
		editorView.dispatch({ effects: languageCompartment.reconfigure(ext) });
	}

	function createMergeEditor(localContent: string) {
		if (!editorContainer) return;
		if (editorView) editorView.destroy();

		const extensions = [
			lineNumbers(),
			syntaxHighlighting(defaultHighlightStyle),
			mergeTheme,
			EditorView.lineWrapping,
			unifiedMergeView({
				original: remoteContent,
				collapseUnchanged: { margin: 3, minSize: 6 }
			}),
			languageCompartment.of([])
		];

		const state = EditorState.create({
			doc: localContent,
			extensions
		});

		editorView = new EditorView({
			state,
			parent: editorContainer
		});
	}

	async function handleReloadRemote() {
		if (!filePath) return;

		const confirmed = await confirmStore.confirm({
			title: 'Reload From Server',
			message: 'Discard local changes and reload the latest server version?',
			confirmText: 'Reload',
			cancelText: 'Cancel',
			destructive: true
		});
		if (!confirmed) return;

		await fileStore.reloadFileFromRemote(filePath);
		conflictStore.close();
	}

	async function handleApplyMerge() {
		if (!filePath || !editorView) return;

		const merged = editorView.state.doc.toString();
		fileStore.updateFileContent(filePath, merged);
		if (remoteMtime !== null) {
			fileStore.setRemoteMtime(filePath, remoteMtime);
		}
		conflictStore.close();
	}

	async function handleOverwriteRemote() {
		if (!filePath || !editorView) return;

		const confirmed = await confirmStore.confirm({
			title: 'Overwrite Remote File',
			message: 'Overwrite the remote file with your merged content?',
			confirmText: 'Overwrite',
			cancelText: 'Cancel',
			destructive: true
		});
		if (!confirmed) return;

		const merged = editorView.state.doc.toString();
		await fileStore.forceSaveFile(filePath, merged);
		conflictStore.close();
	}

	$effect(() => {
		if (!isOpen) {
			if (editorView) {
				editorView.destroy();
				editorView = null;
			}
			remoteContent = '';
			remoteMtime = null;
			return;
		}

		if (!filePath || !openFile) return;
		loadRemote();
	});

	$effect(() => {
		if (!isOpen || !filePath || !openFile) return;
		if (!editorContainer) return;
		if (loading) return;
		if (remoteMtime === null) return;

		createMergeEditor(openFile.content);
		applyLanguage(openFile.language);
	});

	onDestroy(() => {
		if (editorView) editorView.destroy();
	});
</script>

<Modal open={isOpen} title="Save Conflict" size="xl" onclose={() => conflictStore.close()}>
	{#if conflictSessionId && currentSessionId && conflictSessionId !== currentSessionId}
		<div class="space-y-3">
			<div class="text-sm text-gray-300">
				This conflict belongs to a different project tab. Switch back to resolve it.
			</div>
			<div class="flex justify-end">
				<Button onclick={() => conflictStore.close()}>Close</Button>
			</div>
		</div>
	{:else if !filePath || !openFile}
		<div class="text-sm text-gray-400">No conflicted file is available.</div>
	{:else}
		<div class="space-y-3">
			<div class="text-sm text-gray-300">
				The file changed on the server since it was opened. Review differences and choose how to proceed.
			</div>

			<div class="border border-panel-border rounded-lg overflow-auto h-[60vh] bg-editor-bg">
				{#if loading}
					<div class="h-full flex items-center justify-center text-gray-400 text-sm">Loading latest version…</div>
				{:else if remoteMtime === null}
					<div class="h-full flex items-center justify-center text-gray-400 text-sm">
						Failed to load remote content.
						<Button size="sm" onclick={loadRemote}>Retry</Button>
					</div>
				{:else}
					<div bind:this={editorContainer} class="h-full w-full"></div>
				{/if}
			</div>

			<div class="flex items-center justify-between gap-2">
				<div class="flex items-center gap-2">
					<Button variant="ghost" onclick={handleReloadRemote} disabled={loading}>
						Reload From Server
					</Button>
				</div>
				<div class="flex items-center gap-2">
					<Button variant="ghost" onclick={() => conflictStore.close()}>Cancel</Button>
					<Button onclick={handleApplyMerge} disabled={loading || remoteMtime === null}>
						Apply Merge to Editor
					</Button>
					<Button variant="danger" onclick={handleOverwriteRemote} disabled={loading || remoteMtime === null}>
						Overwrite Remote
					</Button>
				</div>
			</div>
		</div>
	{/if}
</Modal>

<style>
	:global(.cm-editor) {
		height: 100%;
	}
	:global(.cm-mergeView) {
		height: 100%;
	}
</style>
