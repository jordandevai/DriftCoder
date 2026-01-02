<script lang="ts">
	import { workspaceStore } from '$stores/workspace';
	import Modal from '$components/shared/Modal.svelte';
	import Button from '$components/shared/Button.svelte';
	import {
		closeActiveProject,
		closeActiveTerminalPanel,
		newTerminal,
		saveActiveFile,
		saveAllDirtyFilesInActiveSession,
		toggleFileTree
	} from '$utils/commands';

	interface Props {
		collapsed?: boolean;
		ontogglecollapse?: () => void;
		onaddproject?: () => void;
	}

	let { collapsed = false, ontogglecollapse, onaddproject }: Props = $props();

	let activeMenu = $state<string | null>(null);
	let shortcutsOpen = $state(false);
	let aboutOpen = $state(false);

	interface MenuItem {
		label: string;
		action?: string;
		shortcut?: string;
		type?: 'separator';
		disabled?: boolean;
	}

	interface Menu {
		label: string;
		items: MenuItem[];
	}

	const menus: Menu[] = [
		{
			label: 'File',
			items: [
				{ label: 'Add Project...', action: 'addProject', shortcut: 'Ctrl+Shift+N' },
				{ label: 'Close Project', action: 'closeProject' },
				{ type: 'separator', label: '' },
				{ label: 'Save', action: 'save', shortcut: 'Ctrl+S' },
				{ label: 'Save All', action: 'saveAll', shortcut: 'Ctrl+Shift+S' },
				{ type: 'separator', label: '' },
				{ label: 'Disconnect All', action: 'disconnectAll' }
			]
		},
		{
			label: 'Edit',
			items: [
				{ label: 'Undo', action: 'undo', shortcut: 'Ctrl+Z' },
				{ label: 'Redo', action: 'redo', shortcut: 'Ctrl+Shift+Z' },
				{ type: 'separator', label: '' },
				{ label: 'Cut', action: 'cut', shortcut: 'Ctrl+X' },
				{ label: 'Copy', action: 'copy', shortcut: 'Ctrl+C' },
				{ label: 'Paste', action: 'paste', shortcut: 'Ctrl+V' },
				{ type: 'separator', label: '' },
				{ label: 'Find', action: 'find', shortcut: 'Ctrl+F' },
				{ label: 'Replace', action: 'replace', shortcut: 'Ctrl+H' }
			]
		},
		{
			label: 'View',
			items: [
				{ label: 'Toggle File Tree', action: 'toggleFileTree', shortcut: 'Ctrl+B' },
				{ label: 'Toggle Menu Bar', action: 'toggleMenu' }
			]
		},
		{
			label: 'Terminal',
			items: [
				{ label: 'New Terminal', action: 'newTerminal', shortcut: 'Ctrl+Shift+`' },
				{ label: 'Close Terminal', action: 'closeTerminal' }
			]
		},
		{
			label: 'Help',
			items: [
				{ label: 'Keyboard Shortcuts', action: 'shortcuts' },
				{ label: 'About DriftCoder', action: 'about' }
			]
		}
	];

	function handleMenuClick(menuLabel: string) {
		activeMenu = activeMenu === menuLabel ? null : menuLabel;
	}

	function handleMenuHover(menuLabel: string) {
		if (activeMenu !== null) {
			activeMenu = menuLabel;
		}
	}

	async function handleAction(action: string) {
		activeMenu = null;

		switch (action) {
			case 'addProject':
				onaddproject?.();
				break;
			case 'closeProject':
				await closeActiveProject();
				break;
			case 'save':
				await saveActiveFile();
				break;
			case 'saveAll':
				await saveAllDirtyFilesInActiveSession();
				break;
			case 'disconnectAll':
				await workspaceStore.closeAll();
				break;
			case 'toggleFileTree':
				toggleFileTree();
				break;
			case 'toggleMenu':
				ontogglecollapse?.();
				break;
			case 'newTerminal':
				await newTerminal();
				break;
			case 'closeTerminal':
				await closeActiveTerminalPanel();
				break;
			case 'shortcuts':
				shortcutsOpen = true;
				break;
			case 'about':
				aboutOpen = true;
				break;
		}
	}

	function handleBackdropClick() {
		activeMenu = null;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			activeMenu = null;
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Backdrop to close menu when clicking outside -->
{#if activeMenu}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="fixed inset-0 z-40" onclick={handleBackdropClick}></div>
{/if}

<div
	class="menu-toolbar flex items-center bg-sidebar-bg border-b border-panel-border px-1 select-none relative z-50"
	style="height: calc(2.25rem + env(safe-area-inset-top, 0px)); padding-top: env(safe-area-inset-top, 0px);"
>
	<!-- Hamburger toggle button -->
	<button
		class="menu-toggle flex items-center justify-center w-11 h-11 rounded hover:bg-panel-active transition-colors"
		onclick={ontogglecollapse}
		aria-label={collapsed ? 'Expand menu' : 'Collapse menu'}
	>
		<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
		</svg>
	</button>

	{#if !collapsed}
		<!-- Menu items -->
		{#each menus as menu}
			<div class="relative">
				<button
					class="px-3 py-1.5 text-sm rounded hover:bg-panel-active transition-colors
							 {activeMenu === menu.label ? 'bg-panel-active' : ''}"
					onclick={() => handleMenuClick(menu.label)}
					onmouseenter={() => handleMenuHover(menu.label)}
				>
					{menu.label}
				</button>

				{#if activeMenu === menu.label}
					<div
						class="absolute top-full left-0 mt-0.5 bg-panel-bg border border-panel-border rounded-lg shadow-xl py-1 min-w-52 z-50"
					>
						{#each menu.items as item}
							{#if item.type === 'separator'}
								<div class="border-t border-panel-border my-1"></div>
							{:else}
								<button
									class="w-full px-4 py-2 text-sm text-left hover:bg-sidebar-hover flex justify-between items-center
											 {item.disabled ? 'opacity-50 cursor-not-allowed' : ''}"
									onclick={() => item.action && handleAction(item.action)}
									disabled={item.disabled}
								>
									<span>{item.label}</span>
									{#if item.shortcut}
										<span class="text-gray-500 text-xs ml-4">{item.shortcut}</span>
									{/if}
								</button>
							{/if}
						{/each}
					</div>
				{/if}
			</div>
		{/each}
	{/if}

	<!-- Spacer -->
	<div class="flex-1"></div>

	<!-- App title when collapsed -->
	{#if collapsed}
		<span class="text-sm text-gray-400 mr-2">DriftCoder</span>
	{/if}
</div>

<Modal bind:open={shortcutsOpen} title="Keyboard Shortcuts" size="lg">
	<div class="space-y-4 text-sm">
		<div class="text-gray-400">
			Shortcuts currently supported in the app:
		</div>
		<div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
			<div class="flex items-center justify-between gap-3 bg-editor-bg border border-panel-border rounded px-3 py-2">
				<span>Save</span>
				<code class="text-xs text-gray-300">Ctrl+S</code>
			</div>
			<div class="flex items-center justify-between gap-3 bg-editor-bg border border-panel-border rounded px-3 py-2">
				<span>Add Project</span>
				<code class="text-xs text-gray-300">Ctrl+Shift+N</code>
			</div>
			<div class="flex items-center justify-between gap-3 bg-editor-bg border border-panel-border rounded px-3 py-2">
				<span>New Terminal</span>
				<code class="text-xs text-gray-300">Ctrl+Shift+`</code>
			</div>
			<div class="flex items-center justify-between gap-3 bg-editor-bg border border-panel-border rounded px-3 py-2">
				<span>Toggle File Tree</span>
				<code class="text-xs text-gray-300">Ctrl+B</code>
			</div>
			<div class="flex items-center justify-between gap-3 bg-editor-bg border border-panel-border rounded px-3 py-2">
				<span>Close Menus/Dialogs</span>
				<code class="text-xs text-gray-300">Esc</code>
			</div>
		</div>

		<div class="flex justify-end">
			<Button onclick={() => (shortcutsOpen = false)}>Close</Button>
		</div>
	</div>
</Modal>

<Modal bind:open={aboutOpen} title="About DriftCoder" size="md">
	<div class="space-y-3 text-sm">
		<div class="text-gray-200">
			DriftCoder is a lightweight SSH-based code editor built with Tauri and Svelte.
		</div>
		<div class="text-gray-400">
			Key features: remote editing over SSH (no server install), CodeMirror editor, xterm terminal, multi-project tabs.
		</div>
		<div class="flex justify-end">
			<Button onclick={() => (aboutOpen = false)}>Close</Button>
		</div>
	</div>
</Modal>

<style>
	.menu-toggle {
		min-width: 44px;
		min-height: 44px;
	}
</style>
