<script lang="ts">
	import { onMount } from 'svelte';
	import '../app.css';
	import { connectionStore } from '$stores/connection';
	import { debugStore } from '$stores/debug';
	import { settingsStore } from '$stores/settings';
	import { terminalStore } from '$stores/terminal';
	import { workspaceStore } from '$stores/workspace';

	let { children } = $props();

	onMount(() => {
		connectionStore.init();
		debugStore.init();
		void settingsStore.init();
		void workspaceStore.init().then(() => {
			terminalStore.hydrateFromWorkspace();
		});
	});
</script>

<svelte:head>
	<link rel="preconnect" href="https://fonts.googleapis.com" />
	<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
	<link
		href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap"
		rel="stylesheet"
	/>
</svelte:head>

<div class="h-screen w-screen overflow-hidden bg-editor-bg text-editor-fg">
	{@render children()}
</div>
