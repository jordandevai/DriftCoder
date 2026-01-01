<script lang="ts">
	import Modal from '$components/shared/Modal.svelte';
	import Button from '$components/shared/Button.svelte';
	import { confirmStore } from '$stores/confirm';

	const request = $derived($confirmStore.active);
</script>

<Modal
	open={!!request}
	title={request?.title}
	size="md"
	onclose={() => confirmStore.resolve(false)}
>
	{#if request}
		<div class="space-y-3">
			<div class="text-sm text-gray-200 whitespace-pre-wrap break-words">{request.message}</div>
			{#if request.detail}
				<pre class="text-xs bg-editor-bg border border-panel-border rounded p-3 overflow-auto max-h-48 whitespace-pre-wrap break-words">{request.detail}</pre>
			{/if}

			<div class="flex justify-end gap-2 pt-2">
				<Button variant="ghost" onclick={() => confirmStore.resolve(false)}>{request.cancelText}</Button>
				<Button
					onclick={() => confirmStore.resolve(true)}
					variant={request.destructive ? 'danger' : 'primary'}
				>
					{request.confirmText}
				</Button>
			</div>
		</div>
	{/if}
</Modal>

