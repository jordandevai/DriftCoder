<script lang="ts">
	import type { ConnectionProfile, AuthMethod } from '$types';
	import Button from '$components/shared/Button.svelte';
	import Input from '$components/shared/Input.svelte';

	interface Props {
		profile?: ConnectionProfile | null;
		onclose: () => void;
		onsave: (profile: ConnectionProfile) => void;
		onconnect: (profile: ConnectionProfile, password?: string) => void;
	}

	let { profile = null, onclose, onsave, onconnect }: Props = $props();

	// Form state - initialize from profile
	let name = $state('');
	let host = $state('');
	let port = $state('22');
	let username = $state('');
	let authMethod = $state<AuthMethod>('key');
	let keyPath = $state('~/.ssh/id_rsa');

	// Reset form when profile changes
	$effect(() => {
		name = profile?.name || '';
		host = profile?.host || '';
		port = profile?.port?.toString() || '22';
		username = profile?.username || '';
		authMethod = profile?.authMethod || 'key';
		keyPath = profile?.keyPath || '~/.ssh/id_rsa';
	});
	let password = $state('');
	let saveConnection = $state(true);
	let testing = $state(false);
	let testResult = $state<'success' | 'failed' | null>(null);
	let testError = $state<string | null>(null);

	const isEditing = $derived(!!profile);

	function generateId(): string {
		return crypto.randomUUID();
	}

	function buildProfile(): ConnectionProfile {
		return {
			id: profile?.id || generateId(),
			name: name || `${username}@${host}`,
			host,
			port: parseInt(port) || 22,
			username,
			authMethod,
			keyPath: authMethod === 'key' ? keyPath : undefined,
			recentProjects: profile?.recentProjects || [],
			bookmarkedPaths: profile?.bookmarkedPaths || []
		};
	}

	async function handleTest() {
		testing = true;
		testResult = null;
		testError = null;

		try {
			const { invoke } = await import('$utils/tauri');
			const connectionProfile = buildProfile();
			const success = await invoke<boolean>('ssh_test_connection', {
				profile: connectionProfile,
				password: authMethod === 'password' ? password : undefined
			});
			testResult = success ? 'success' : 'failed';
		} catch (e) {
			testResult = 'failed';
			testError = e instanceof Error ? e.message : String(e);
		} finally {
			testing = false;
		}
	}

	function handleSave() {
		const connectionProfile = buildProfile();
		onsave(connectionProfile);
	}

	function handleConnect() {
		const connectionProfile = buildProfile();
		if (saveConnection) {
			onsave(connectionProfile);
		}
		onconnect(connectionProfile, authMethod === 'password' ? password : undefined);
	}

	function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		handleConnect();
	}
</script>

<div class="bg-panel-bg border border-panel-border rounded-lg overflow-hidden">
	<div class="flex items-center justify-between px-4 py-3 border-b border-panel-border">
		<h2 class="text-lg font-medium">
			{isEditing ? 'Edit Connection' : 'New Connection'}
		</h2>
		<button class="p-1 rounded hover:bg-panel-active transition-colors" onclick={onclose} aria-label="Close">
			<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M6 18L18 6M6 6l12 12"
				/>
			</svg>
		</button>
	</div>

	<form class="p-4 space-y-4" onsubmit={handleSubmit}>
		<div class="flex items-center justify-between gap-3">
			<label class="flex items-center gap-2 cursor-pointer select-none">
				<input type="checkbox" bind:checked={saveConnection} class="text-accent rounded" />
				<span class="text-sm text-gray-300">Save connection</span>
			</label>
			<div class="text-xs text-gray-500">{saveConnection ? 'Saved' : 'Quick connect'}</div>
		</div>

		{#if saveConnection}
			<Input label="Connection Name" placeholder="My Server" bind:value={name} />
		{/if}

		<div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
			<div class="sm:col-span-2">
				<Input label="Server" placeholder="192.168.1.100" bind:value={host} required />
			</div>
			<Input label="Port" type="number" bind:value={port} required />
		</div>

		<Input label="Username" placeholder="user" bind:value={username} required />

		<!-- Auth Method -->
		<fieldset class="space-y-2">
			<legend class="text-sm text-gray-400">Authentication</legend>
			<div class="flex gap-4">
				<label class="flex items-center gap-2 cursor-pointer">
					<input
						type="radio"
						name="authMethod"
						value="key"
						bind:group={authMethod}
						class="text-accent"
					/>
					<span>SSH Key</span>
				</label>
				<label class="flex items-center gap-2 cursor-pointer">
					<input
						type="radio"
						name="authMethod"
						value="password"
						bind:group={authMethod}
						class="text-accent"
					/>
					<span>Password</span>
				</label>
			</div>
		</fieldset>

		{#if authMethod === 'key'}
			<details class="bg-editor-bg border border-panel-border rounded-lg p-3">
				<summary class="cursor-pointer text-sm text-gray-300 select-none">Advanced</summary>
				<div class="pt-3 space-y-3">
					<Input label="Key Path" placeholder="~/.ssh/id_rsa" bind:value={keyPath} />
				</div>
			</details>
		{:else}
			<Input label="Password" type="password" bind:value={password} required />
		{/if}

		<div class="flex items-center gap-3">
			<Button type="button" variant="secondary" onclick={handleTest} loading={testing}>Test</Button>
			{#if testResult}
				<div
					class="flex-1 p-3 rounded text-sm {testResult === 'success'
						? 'bg-success/10 text-success border border-success'
						: 'bg-error/10 text-error border border-error'}"
				>
					{#if testResult === 'success'}
						Connection successful!
					{:else}
						Connection failed{testError ? `: ${testError}` : ''}
					{/if}
				</div>
			{/if}
		</div>

		<!-- Actions -->
		<div class="flex gap-3 pt-2">
			<div class="flex-1"></div>
			<Button variant="ghost" onclick={onclose}>Cancel</Button>
			<Button type="submit">Connect</Button>
		</div>
	</form>
</div>
