<script lang="ts">
	import Modal from '$components/shared/Modal.svelte';
	import Button from '$components/shared/Button.svelte';
	import { diagnosticsStore } from '$stores/diagnostics';
	import { debugStore, traceHistory, isTraceEnabled } from '$stores/debug';
	import { invoke, isTauri } from '$utils/tauri';

	type KnownHostEntry = {
		host: string;
		port: number;
		keyType: string;
		fingerprintSha256: string;
		publicKeyOpenssh: string;
		trustedAt: number;
	};

	const open = $derived($diagnosticsStore.open);
	const traces = $derived($traceHistory);

	let filter = $state('');
	let hostKeys = $state<KnownHostEntry[] | null>(null);
	let hostKeysLoading = $state(false);
	let hostKeysError = $state<string | null>(null);

	const filtered = $derived.by(() => {
		const f = filter.trim().toLowerCase();
		if (!f) return traces;
		return traces.filter((t) => {
			const hay = `${t.category}:${t.step} ${t.message} ${t.detail ?? ''}`.toLowerCase();
			return hay.includes(f);
		});
	});

	function close() {
		diagnosticsStore.close();
	}

	function formatTime(ts: number): string {
		return new Date(ts).toLocaleTimeString();
	}

	async function loadTrustedHostKeys() {
		if (!isTauri()) return;
		hostKeysLoading = true;
		hostKeysError = null;
		try {
			hostKeys = await invoke<KnownHostEntry[]>('ssh_list_trusted_host_keys');
		} catch (error) {
			hostKeysError = error instanceof Error ? error.message : String(error);
		} finally {
			hostKeysLoading = false;
		}
	}

	async function forgetTrustedHostKey(entry: KnownHostEntry) {
		if (!isTauri()) return;
		await invoke('ssh_forget_host_key', { host: entry.host, port: entry.port });
		await loadTrustedHostKeys();
	}

	async function copyDiagnosticsReport() {
		const exportedAt = Date.now();
		const report: Record<string, unknown> = {
			exportedAt,
			frontend: {
				traceHistory: traces
			}
		};

		if (isTauri()) {
			try {
				report.backend = await invoke<unknown>('debug_export_diagnostics');
			} catch (error) {
				report.backend = { error: error instanceof Error ? error.message : String(error) };
			}
		}

		await navigator.clipboard.writeText(JSON.stringify(report, null, 2));
	}
</script>

<Modal open={open} title="Diagnostics" size="xl" onclose={close}>
	<div class="flex flex-col gap-3">
		<div class="flex flex-wrap items-center justify-between gap-2">
			<div class="flex items-center gap-2">
				<input
					class="input w-64 max-w-full"
					placeholder="Filter traces…"
					bind:value={filter}
				/>
				<div class="text-xs text-gray-500">{filtered.length} events</div>
			</div>

			<div class="flex items-center gap-2">
				<Button size="sm" variant="ghost" onclick={() => debugStore.toggleTrace()}>
					{$isTraceEnabled ? 'Disable Tracing' : 'Enable Tracing'}
				</Button>
				<Button size="sm" variant="ghost" onclick={() => debugStore.clearTraces()} disabled={traces.length === 0}>
					Clear
				</Button>
				<Button size="sm" variant="ghost" onclick={copyDiagnosticsReport}>
					Copy Report
				</Button>
				<Button size="sm" onclick={close}>Close</Button>
			</div>
		</div>

		<div class="border border-panel-border rounded-lg overflow-auto">
			<div class="max-h-[70vh] overflow-auto">
				<table class="w-full text-xs">
					<thead class="sticky top-0 bg-panel-bg border-b border-panel-border">
						<tr class="text-gray-400">
							<th class="text-left font-medium px-2 py-2 w-24">Time</th>
							<th class="text-left font-medium px-2 py-2 w-40">Category</th>
							<th class="text-left font-medium px-2 py-2">Message</th>
						</tr>
					</thead>
					<tbody>
						{#if filtered.length === 0}
							<tr>
								<td class="px-2 py-3 text-gray-500" colspan="3">No trace events.</td>
							</tr>
						{:else}
							{#each filtered as t (t.timestamp + ':' + (t.correlationId ?? '') + ':' + t.category + ':' + t.step)}
								<tr class="border-b border-panel-border/60 {t.isError ? 'bg-error/5' : ''}">
									<td class="px-2 py-2 text-gray-500 font-mono">{formatTime(t.timestamp)}</td>
									<td class="px-2 py-2">
										<div class="font-mono text-gray-300">
											{t.category}:{t.step}
										</div>
										{#if t.correlationId}
											<div class="text-[10px] text-gray-500 font-mono truncate max-w-[160px]">
												{t.correlationId}
											</div>
										{/if}
									</td>
									<td class="px-2 py-2">
										<div class="text-gray-200 whitespace-pre-wrap break-words">{t.message}</div>
										{#if t.detail}
											<div class="text-gray-400 whitespace-pre-wrap break-words mt-1">{t.detail}</div>
										{/if}
									</td>
								</tr>
							{/each}
						{/if}
					</tbody>
				</table>
			</div>
		</div>

		<details class="border border-panel-border rounded-lg overflow-auto">
			<summary class="px-3 py-2 text-xs text-gray-400 cursor-pointer select-none bg-panel-bg border-b border-panel-border">
				Trusted Host Keys {hostKeys ? `(${hostKeys.length})` : ''}
			</summary>
			<div class="p-3 flex flex-col gap-2">
				<div class="flex items-center justify-between gap-2">
					<div class="text-xs text-gray-500">
						Manage trusted SSH host keys stored on this device.
					</div>
					<Button size="sm" variant="ghost" onclick={loadTrustedHostKeys} disabled={!isTauri() || hostKeysLoading}>
						{hostKeysLoading ? 'Loading…' : hostKeys ? 'Refresh' : 'Load'}
					</Button>
				</div>

				{#if !isTauri()}
					<div class="text-xs text-gray-500">Not available in web preview.</div>
				{:else if hostKeysError}
					<div class="text-xs text-error whitespace-pre-wrap break-words">{hostKeysError}</div>
				{:else if hostKeys && hostKeys.length === 0}
					<div class="text-xs text-gray-500">No trusted host keys yet.</div>
				{:else if hostKeys}
					<div class="overflow-auto">
						<table class="w-full text-xs">
							<thead class="text-gray-400">
								<tr>
									<th class="text-left font-medium py-1 pr-2">Host</th>
									<th class="text-left font-medium py-1 pr-2">Fingerprint</th>
									<th class="text-left font-medium py-1"></th>
								</tr>
							</thead>
							<tbody>
								{#each hostKeys as hk (hk.host + ':' + hk.port)}
									<tr class="border-t border-panel-border/60">
										<td class="py-2 pr-2 font-mono text-gray-200">{hk.host}:{hk.port}</td>
										<td class="py-2 pr-2 font-mono text-gray-300 truncate max-w-[420px]">{hk.keyType} {hk.fingerprintSha256}</td>
										<td class="py-2 text-right">
											<Button size="sm" variant="ghost" onclick={() => forgetTrustedHostKey(hk)}>
												Forget
											</Button>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</div>
		</details>
	</div>
</Modal>
