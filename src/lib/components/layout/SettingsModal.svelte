<script lang="ts">
	import Modal from '$components/shared/Modal.svelte';
	import Button from '$components/shared/Button.svelte';
	import { settingsStore } from '$stores/settings';
	import { settingsUiStore } from '$stores/settings-ui';

	const open = $derived($settingsUiStore.open);
	const scrollback = $derived($settingsStore.terminalScrollback);
</script>

<Modal open={open} title="Settings" size="md" onclose={() => settingsUiStore.close()}>
	<div class="space-y-5">
		<div>
			<div class="text-sm font-medium text-gray-200">Terminal scrollback (lines)</div>
			<div class="text-xs text-gray-400 mt-1">
				Higher values keep more history but use more memory. Applies immediately; existing history can’t be restored.
			</div>

			<div class="mt-3 flex items-center gap-3">
				<input
					class="input w-32"
					type="number"
					min="1000"
					max="200000"
					step="1000"
					value={scrollback}
					oninput={(e) => settingsStore.setTerminalScrollback(Number((e.currentTarget as HTMLInputElement).value))}
				/>
				<div class="text-xs text-gray-400">Default: 50000</div>
			</div>
		</div>

		<div class="flex justify-end gap-2 pt-2">
			<Button variant="ghost" onclick={() => settingsUiStore.close()}>Close</Button>
		</div>
	</div>
</Modal>

