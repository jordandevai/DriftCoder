<script lang="ts">
	import Modal from '$components/shared/Modal.svelte';
	import Button from '$components/shared/Button.svelte';
	import {
		notificationsStore,
		selectedNotification,
		unreadCount,
		type Notification
	} from '$stores/notifications';

	const open = $derived($notificationsStore.centerOpen);
	const notifications = $derived($notificationsStore.notifications);
	const selected = $derived($selectedNotification);

	function formatTime(ts: number): string {
		return new Date(ts).toLocaleString();
	}

	function severityDot(severity: Notification['severity']): string {
		switch (severity) {
			case 'error':
				return 'bg-error';
			case 'warning':
				return 'bg-warning';
			default:
				return 'bg-gray-500';
		}
	}

	function close() {
		notificationsStore.closeCenter();
	}

	async function copySelected() {
		if (!selected) return;
		const payload = {
			title: selected.title,
			message: selected.message,
			detail: selected.detail ?? null,
			createdAt: selected.createdAt
		};
		const text = JSON.stringify(payload, null, 2);
		try {
			await navigator.clipboard.writeText(text);
		} catch (error) {
			console.error('Failed to copy notification:', error);
		}
	}
</script>

<Modal open={open} title="Notifications" size="xl" onclose={close}>
	<div class="flex flex-col gap-3">
		<div class="flex items-center justify-between gap-3">
			<div class="text-sm text-gray-400">
				{$unreadCount} unread • {notifications.length} total
			</div>
			<div class="flex items-center gap-2">
				<Button
					size="sm"
					variant="ghost"
					onclick={() => {
						notificationsStore.clearAll();
						close();
					}}
					disabled={notifications.length === 0}
				>
					Clear All
				</Button>
				<Button size="sm" onclick={close}>Close</Button>
			</div>
		</div>

		<div class="grid grid-cols-1 lg:grid-cols-2 gap-3 min-h-[360px]">
			<div class="border border-panel-border rounded-lg overflow-hidden">
				{#if notifications.length === 0}
					<div class="p-4 text-gray-400 text-sm">No notifications.</div>
				{:else}
					<div class="max-h-[60vh] overflow-y-auto">
						{#each notifications as n (n.id)}
							<button
								class="w-full text-left px-3 py-2 border-b border-panel-border hover:bg-panel-active transition-colors"
								onclick={() => notificationsStore.select(n.id)}
							>
								<div class="flex items-start gap-2">
									<span class="w-2 h-2 rounded-full mt-1.5 {severityDot(n.severity)}"></span>
									<div class="min-w-0 flex-1">
										<div class="flex items-center justify-between gap-2">
											<div class="text-sm font-medium text-editor-fg truncate">
												{n.title}
											</div>
											{#if !n.readAt}
												<span class="text-[10px] text-accent uppercase">Unread</span>
											{/if}
										</div>
										<div class="text-xs text-gray-400 truncate">{n.message}</div>
										<div class="text-[10px] text-gray-500 mt-0.5">{formatTime(n.createdAt)}</div>
									</div>
								</div>
							</button>
						{/each}
					</div>
				{/if}
			</div>

			<div class="border border-panel-border rounded-lg overflow-hidden">
				{#if !selected}
					<div class="p-4 text-gray-400 text-sm">Select a notification to view details.</div>
				{:else}
					<div class="p-4 space-y-3">
						<div class="flex items-start justify-between gap-3">
							<div class="min-w-0">
								<div class="text-sm font-medium text-editor-fg">{selected.title}</div>
								<div class="text-xs text-gray-400 mt-0.5">{formatTime(selected.createdAt)}</div>
							</div>
							<div class="flex items-center gap-2">
								<Button
									size="sm"
									variant="ghost"
									onclick={copySelected}
									disabled={!selected.message && !selected.detail}
								>
									Copy
								</Button>
								<Button size="sm" variant="ghost" onclick={() => notificationsStore.dismiss(selected.id)}>
									Dismiss Popup
								</Button>
								<Button size="sm" variant="ghost" onclick={() => notificationsStore.markRead(selected.id)}>
									Mark Read
								</Button>
							</div>
						</div>

						<div class="text-sm text-gray-200 whitespace-pre-wrap break-words">{selected.message}</div>

						{#if selected.detail}
							<pre class="text-xs bg-editor-bg border border-panel-border rounded p-3 overflow-auto max-h-56 whitespace-pre-wrap break-words">{selected.detail}</pre>
						{/if}

						{#if selected.actions.length > 0}
							<div class="flex flex-wrap gap-2 pt-2">
								{#each selected.actions as a (a.label)}
									<Button
										size="sm"
										onclick={async () => {
											try {
												await a.run();
											} catch (error) {
												console.error('Notification action failed:', error);
											}
										}}
									>
										{a.label}
									</Button>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	</div>
</Modal>
