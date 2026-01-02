<script lang="ts">
	import Modal from '$components/shared/Modal.svelte';
	import Button from '$components/shared/Button.svelte';
	import { promptStore } from '$stores/prompt';

	const request = $derived($promptStore.active);
	const value = $derived($promptStore.value);
</script>

<Modal open={!!request} title={request?.title} size="md" onclose={() => promptStore.close()}>
	{#if request}
		<div class="space-y-3">
			{#if request.message}
				<div class="text-sm text-gray-200 whitespace-pre-wrap break-words">{request.message}</div>
			{/if}
			{#if request.detail}
				<pre class="text-xs bg-editor-bg border border-panel-border rounded p-3 overflow-auto max-h-48 whitespace-pre-wrap break-words">{request.detail}</pre>
			{/if}

			<!-- svelte-ignore a11y_autofocus -->
			<input
				class="input w-full"
				placeholder={request.placeholder ?? ''}
				value={value}
				oninput={(e) => promptStore.setValue((e.currentTarget as HTMLInputElement).value)}
				onkeydown={(e) => {
					if (e.key === 'Enter') promptStore.resolve(value.trim() ? value.trim() : null);
					if (e.key === 'Escape') promptStore.close();
				}}
				autofocus
			/>

			<div class="flex justify-end gap-2 pt-2">
				<Button variant="ghost" onclick={() => promptStore.close()}>
					{request.cancelText ?? 'Cancel'}
				</Button>
				<Button onclick={() => promptStore.resolve(value.trim() ? value.trim() : null)}>
					{request.confirmText ?? 'OK'}
				</Button>
			</div>
		</div>
	{/if}
</Modal>

