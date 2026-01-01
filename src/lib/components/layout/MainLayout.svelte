<script lang="ts">
	import { layoutStore } from '$stores/layout';
	import { hasSessions, activeSession, orderedSessions } from '$stores/workspace';
	import MenuToolbar from './MenuToolbar.svelte';
	import ProjectTabs from './ProjectTabs.svelte';
	import FileTreePanel from '$components/panels/FileTreePanel.svelte';
	import PanelGroup from './PanelGroup.svelte';
	import StatusBar from './StatusBar.svelte';
	import ConnectionScreen from '$components/connection/ConnectionScreen.svelte';
	import AddProjectModal from '$components/workspace/AddProjectModal.svelte';
	import FolderSelectEmbedded from '$components/workspace/FolderSelectEmbedded.svelte';
	import { workspaceStore } from '$stores/workspace';
	import { connectionStore } from '$stores/connection';
	import type { ConnectionProfile } from '$types';

	let resizing = $state(false);
	let menuCollapsed = $state(false);
	let addProjectOpen = $state(false);

	// Initial connection flow state
	let pendingConnectionId = $state<string | null>(null);
	let pendingProfile = $state<ConnectionProfile | null>(null);

	function handleAddProject() {
		addProjectOpen = true;
	}

	// Handle connection from ConnectionScreen when no sessions exist
	async function handleFirstConnect(profile: ConnectionProfile, password?: string, projectPath?: string) {
		try {
			const connectionId = await connectionStore.connect(profile, password);
			if (projectPath) {
				// Directly open the project
				await workspaceStore.createSession(connectionId, profile, projectPath);
				connectionStore.addRecentProject(profile.id, projectPath);
			} else {
				// Show folder select for this connection
				pendingConnectionId = connectionId;
				pendingProfile = profile;
			}
		} catch (error) {
			console.error('Connection failed:', error);
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
</script>

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
					class="w-1 cursor-col-resize bg-transparent hover:bg-accent/50 transition-colors flex-shrink-0"
					onmousedown={startResize}
				></div>
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
</div>
