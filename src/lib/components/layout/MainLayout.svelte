<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { layoutStore } from '$stores/layout';
	import { hasSessions, activeSession, orderedSessions } from '$stores/workspace';
	import MenuToolbar from './MenuToolbar.svelte';
	import ProjectTabs from './ProjectTabs.svelte';
	import FileTreePanel from '$components/panels/FileTreePanel.svelte';
	import PanelGroup from './PanelGroup.svelte';
	import StatusBar from './StatusBar.svelte';
	import NotificationCenter from './NotificationCenter.svelte';
	import DiagnosticsModal from './DiagnosticsModal.svelte';
	import SettingsModal from './SettingsModal.svelte';
	import ConfirmHost from './ConfirmHost.svelte';
	import PromptModal from './PromptModal.svelte';
	import ConflictResolutionModal from '$components/modals/ConflictResolutionModal.svelte';
	import ConnectionScreen from '$components/connection/ConnectionScreen.svelte';
	import AddProjectModal from '$components/workspace/AddProjectModal.svelte';
	import FolderSelectEmbedded from '$components/workspace/FolderSelectEmbedded.svelte';
	import { workspaceStore } from '$stores/workspace';
	import { connectionStore } from '$stores/connection';
	import { notificationsStore } from '$stores/notifications';
	import { fileStore } from '$stores/files';
	import type { ConnectionProfile } from '$types';
	import {
		closeActivePanel,
		newTerminal,
		saveActiveFile,
		saveAllDirtyFilesInActiveSession,
		toggleFileTree
	} from '$utils/commands';

	let resizing = $state(false);
	let menuCollapsed = $state(false);
	let addProjectOpen = $state(false);

	// Initial connection flow state
	let pendingConnectionId = $state<string | null>(null);
	let pendingProfile = $state<ConnectionProfile | null>(null);

	const lastReconnectAttempt = new Map<string, number>();
	const AUTO_RECONNECT_DEBOUNCE_MS = 30_000;

	onMount(() => {
		fileStore.initRemoteSync();

		const maybeAutoReconnect = (trigger: 'focus' | 'visibility') => {
			const session = get(activeSession);
			if (!session) return;
			if (session.connectionStatus !== 'disconnected') return;

			const now = Date.now();
			const last = lastReconnectAttempt.get(session.connectionId) ?? 0;
			if (now - last < AUTO_RECONNECT_DEBOUNCE_MS) return;
			lastReconnectAttempt.set(session.connectionId, now);

			// Best-effort: reconnect in the background. Password auth will prompt once if needed.
			void connectionStore.reconnect(session.connectionId).catch(() => {
				// ignore; user can use the explicit Reconnect button
			});
		};

		const onFocus = () => maybeAutoReconnect('focus');
		const onVisibility = () => {
			if (!document.hidden) maybeAutoReconnect('visibility');
		};

		window.addEventListener('focus', onFocus);
		document.addEventListener('visibilitychange', onVisibility);

		return () => {
			window.removeEventListener('focus', onFocus);
			document.removeEventListener('visibilitychange', onVisibility);
		};
	});

	onDestroy(() => {
		fileStore.destroyRemoteSync();
	});

	function handleAddProject() {
		addProjectOpen = true;
	}

	// Handle connection from ConnectionScreen when no sessions exist
	async function handleFirstConnect(profile: ConnectionProfile, password?: string, projectPath?: string) {
		try {
			const { connectionId, profile: enrichedProfile } = await connectionStore.connect(profile, password);
			if (projectPath) {
				// Directly open the project
				await workspaceStore.createSession(connectionId, enrichedProfile, projectPath);
				connectionStore.addRecentProject(enrichedProfile.id, projectPath);
			} else {
				// Show folder select for this connection
				pendingConnectionId = connectionId;
				pendingProfile = enrichedProfile;
			}
		} catch (error) {
			console.error('Connection failed:', error);
			notificationsStore.notify({
				severity: 'error',
				title: 'Connection Failed',
				message: `Could not connect to ${profile.username}@${profile.host}:${profile.port}.`,
				detail: error instanceof Error ? error.message : String(error)
			});
		}
	}

	async function handleFolderSelected(path: string) {
		if (pendingConnectionId && pendingProfile) {
			await workspaceStore.createSession(pendingConnectionId, pendingProfile, path);
			connectionStore.addRecentProject(pendingProfile.id, path);
			pendingConnectionId = null;
			pendingProfile = null;
		}
	}

	function handleFolderSelectCancel() {
		// Disconnect since we're canceling before opening a project
		if (pendingConnectionId) {
			connectionStore.disconnectById(pendingConnectionId);
			pendingConnectionId = null;
			pendingProfile = null;
		}
	}

	function startResize(e: MouseEvent) {
		e.preventDefault();
		resizing = true;
		document.addEventListener('mousemove', handleResize);
		document.addEventListener('mouseup', stopResize);
	}

	function handleResize(e: MouseEvent) {
		if (resizing) {
			layoutStore.setFileTreeWidth(e.clientX);
		}
	}

	function stopResize() {
		resizing = false;
		document.removeEventListener('mousemove', handleResize);
		document.removeEventListener('mouseup', stopResize);
	}

	function startTouchResize(e: TouchEvent) {
		if (e.touches.length !== 1) return;
		e.preventDefault();
		resizing = true;
		document.addEventListener('touchmove', handleTouchResize, { passive: false });
		document.addEventListener('touchend', stopTouchResize);
		document.addEventListener('touchcancel', stopTouchResize);
	}

	function handleTouchResize(e: TouchEvent) {
		if (!resizing || e.touches.length !== 1) return;
		e.preventDefault();
		layoutStore.setFileTreeWidth(e.touches[0].clientX);
	}

	function stopTouchResize() {
		resizing = false;
		document.removeEventListener('touchmove', handleTouchResize);
		document.removeEventListener('touchend', stopTouchResize);
		document.removeEventListener('touchcancel', stopTouchResize);
	}

	function terminalHasFocus(): boolean {
		if (typeof document === 'undefined') return false;
		const active = document.activeElement as HTMLElement | null;
		if (!active) return false;
		// xterm.js focuses a hidden textarea; avoid capturing Ctrl shortcuts while the terminal is active
		if (active.classList.contains('xterm-helper-textarea')) return true;
		return Boolean(active.closest('.xterm'));
	}

	function handleGlobalKeydown(e: KeyboardEvent) {
		const mod = e.metaKey || e.ctrlKey;
		if (!mod) return;

		// Avoid double-handling when a focused component already handled the shortcut (e.g. CodeMirror Ctrl+S)
		if (e.defaultPrevented) return;

		// Only handle app-level shortcuts when a project/session exists
		if (!$hasSessions) return;

		// Let the terminal keep its native shortcuts (tmux Ctrl+B, flow control Ctrl+S, etc.)
		if (terminalHasFocus()) return;

		const key = e.key.toLowerCase();
		const code = e.code;

		// Save
		if (!e.shiftKey && key === 's') {
			e.preventDefault();
			saveActiveFile();
			return;
		}

		// Save All
		if (e.shiftKey && key === 's') {
			e.preventDefault();
			saveAllDirtyFilesInActiveSession();
			return;
		}

		// Toggle file tree
		if (!e.shiftKey && key === 'b') {
			e.preventDefault();
			toggleFileTree();
			return;
		}

		// New terminal
		if (e.shiftKey && (key === '`' || code === 'Backquote')) {
			e.preventDefault();
			newTerminal();
			return;
		}

		// Close active tab/panel
		if (!e.shiftKey && key === 'w') {
			e.preventDefault();
			closeActivePanel();
			return;
		}

		// Add project
		if (e.shiftKey && key === 'n') {
			e.preventDefault();
			addProjectOpen = true;
			return;
		}
	}
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<div class="h-full flex flex-col bg-editor-bg {resizing ? 'select-none' : ''}">
	{#if $hasSessions}
		<!-- Full IDE Layout with MenuToolbar + ProjectTabs -->
		<MenuToolbar collapsed={menuCollapsed} ontogglecollapse={() => menuCollapsed = !menuCollapsed} onaddproject={handleAddProject} />
		<ProjectTabs onaddproject={handleAddProject} />

		<!-- Main IDE Content -->
		<div class="flex-1 flex overflow-hidden">
			<!-- File Tree Sidebar -->
			{#if !$layoutStore.fileTreeCollapsed}
				<div
					class="flex-shrink-0 bg-sidebar-bg border-r border-panel-border overflow-hidden"
					style="width: {$layoutStore.fileTreeWidth}px"
				>
					<FileTreePanel />
				</div>

				<!-- Resizer -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="cursor-col-resize bg-transparent transition-colors flex-shrink-0 flex items-center justify-center
					       w-1 hover:bg-accent/50
					       touch-device:w-5 touch-device:bg-panel-border/30 touch-device:hover:bg-accent/30"
					onmousedown={startResize}
					ontouchstart={startTouchResize}
				>
					<!-- Grip indicator (3 horizontal lines) - visible on touch devices -->
					<div class="hidden touch-device:flex flex-col gap-1 opacity-40">
						<div class="w-3 h-0.5 rounded-full bg-gray-400"></div>
						<div class="w-3 h-0.5 rounded-full bg-gray-400"></div>
						<div class="w-3 h-0.5 rounded-full bg-gray-400"></div>
					</div>
				</div>
			{:else}
				<!-- Expand button when file tree is collapsed -->
				<button
					class="flex-shrink-0 flex items-center justify-center bg-sidebar-bg border-r border-panel-border transition-colors hover:bg-sidebar-hover
					       w-8 touch-device:w-12"
					onclick={toggleFileTree}
					title="Expand file tree (Ctrl+B)"
					aria-label="Expand file tree"
				>
					<svg class="w-4 h-4 touch-device:w-5 touch-device:h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7" />
					</svg>
				</button>
			{/if}

			<!-- Panel Area -->
			<div class="flex-1 overflow-hidden">
				<PanelGroup groupId="main" />
			</div>
		</div>

		<!-- Status Bar -->
		<StatusBar onaddproject={handleAddProject} />

	{:else if pendingConnectionId && pendingProfile}
		<!-- Connected but need to select folder -->
		<div class="h-full flex flex-col items-center justify-center p-8 bg-editor-bg">
			<div class="w-full max-w-2xl">
				<div class="text-center mb-6">
					<h1 class="text-2xl font-bold text-editor-fg mb-2">Select Project Folder</h1>
					<p class="text-gray-400">
						Connected to <span class="text-accent">{pendingProfile.host}</span>
					</p>
				</div>
				<FolderSelectEmbedded
					connectionId={pendingConnectionId}
					profile={pendingProfile}
					onselect={handleFolderSelected}
					oncancel={handleFolderSelectCancel}
				/>
			</div>
		</div>

	{:else}
		<!-- Connection Screen (no sessions) -->
		<ConnectionScreen onconnected={handleFirstConnect} />
	{/if}

	<!-- Add Project Modal -->
	<AddProjectModal bind:open={addProjectOpen} />

	<NotificationCenter />
	<DiagnosticsModal />
	<SettingsModal />
	<ConfirmHost />
	<PromptModal />
	<ConflictResolutionModal />
</div>
