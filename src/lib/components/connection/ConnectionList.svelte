<script lang="ts">
	import type { ConnectionProfile } from '$types';
	import Button from '$components/shared/Button.svelte';

	interface Props {
		profiles: ConnectionProfile[];
		onconnect: (profile: ConnectionProfile, password?: string, projectPath?: string) => void;
		onedit: (profile: ConnectionProfile) => void;
		ondelete: (id: string) => void;
		onnew: () => void;
	}

	let { profiles, onconnect, onedit, ondelete, onnew }: Props = $props();

	let passwordPrompt = $state<{
		profile: ConnectionProfile;
		password: string;
		projectPath?: string;
	} | null>(null);

	function handleConnect(profile: ConnectionProfile, projectPath?: string) {
		if (profile.authMethod === 'password') {
			passwordPrompt = { profile, password: '', projectPath };
		} else {
			onconnect(profile, undefined, projectPath);
		}
	}

	function handlePasswordSubmit(e: SubmitEvent) {
		e.preventDefault();
		if (passwordPrompt) {
			onconnect(passwordPrompt.profile, passwordPrompt.password, passwordPrompt.projectPath);
			passwordPrompt = null;
		}
	}

	function handleDeleteConfirm(profile: ConnectionProfile) {
		if (confirm(`Delete connection "${profile.name}"?`)) {
			ondelete(profile.id);
		}
	}

	function getProjectName(path: string): string {
		return path.split('/').pop() || path;
	}
</script>

<div class="space-y-4">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<h2 class="text-lg font-medium">Saved Connections</h2>
		<Button size="sm" onclick={onnew}>
			<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
			</svg>
			New
		</Button>
	</div>

	<!-- Connection List -->
	<div class="space-y-2">
		{#each profiles as profile (profile.id)}
			<div
				class="bg-panel-bg border border-panel-border rounded-lg p-4 hover:border-accent/50 transition-colors group"
			>
				<div class="flex items-center gap-4">
					<!-- Icon -->
					<div class="p-2 bg-editor-bg rounded-lg">
						<svg class="w-6 h-6 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="1.5"
								d="M5 12h14M12 5l7 7-7 7"
							/>
						</svg>
					</div>

					<!-- Info -->
					<div class="flex-1 min-w-0">
						<h3 class="font-medium text-editor-fg truncate">{profile.name}</h3>
						<p class="text-sm text-gray-400 truncate">
							{profile.username}@{profile.host}:{profile.port}
						</p>
					</div>

					<!-- Actions -->
					<div class="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
						<button
							class="p-2 rounded hover:bg-panel-active transition-colors"
							title="Edit"
							onclick={() => onedit(profile)}
						>
							<svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
								/>
							</svg>
						</button>
						<button
							class="p-2 rounded hover:bg-error/10 transition-colors"
							title="Delete"
							onclick={() => handleDeleteConfirm(profile)}
						>
							<svg class="w-4 h-4 text-error" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
								/>
							</svg>
						</button>
					</div>

					<!-- Connect Button -->
					<Button size="sm" onclick={() => handleConnect(profile)}>Connect</Button>
				</div>

				<!-- Recent Projects -->
				{#if profile.recentProjects?.length > 0}
					<div class="mt-3 pl-14 space-y-1">
						<p class="text-xs text-gray-500 uppercase tracking-wide">Recent Projects</p>
						<div class="flex flex-wrap gap-2">
							{#each profile.recentProjects.slice(0, 3) as projectPath}
								<button
									class="flex items-center gap-1.5 px-2 py-1 text-xs bg-editor-bg hover:bg-accent/20
										   border border-panel-border hover:border-accent/50 rounded transition-colors"
									onclick={() => handleConnect(profile, projectPath)}
									title="Connect and open {projectPath}"
								>
									<svg class="w-3 h-3 text-yellow-500" fill="currentColor" viewBox="0 0 20 20">
										<path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
									</svg>
									<span class="truncate max-w-24">{getProjectName(projectPath)}</span>
								</button>
							{/each}
						</div>
					</div>
				{/if}
			</div>
		{/each}
	</div>
</div>

<!-- Password Prompt Modal -->
{#if passwordPrompt}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
		<div class="bg-panel-bg border border-panel-border rounded-lg shadow-xl w-full max-w-sm mx-4">
			<div class="px-4 py-3 border-b border-panel-border">
				<h3 class="font-medium">Enter Password</h3>
				<p class="text-sm text-gray-400">{passwordPrompt.profile.name}</p>
			</div>
			<form class="p-4 space-y-4" onsubmit={handlePasswordSubmit}>
				<!-- svelte-ignore a11y_autofocus -->
				<input
					type="password"
					placeholder="Password"
					bind:value={passwordPrompt.password}
					class="input"
					autofocus
				/>
				<div class="flex justify-end gap-2">
					<Button variant="ghost" onclick={() => (passwordPrompt = null)}>Cancel</Button>
					<Button type="submit">Connect</Button>
				</div>
			</form>
		</div>
	</div>
{/if}
