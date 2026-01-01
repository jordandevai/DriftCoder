<script lang="ts">
	import { activePopups, notificationsStore, type Notification } from '$stores/notifications';
	import Button from '$components/shared/Button.svelte';

	function getSeverityClasses(severity: Notification['severity']): string {
		switch (severity) {
			case 'error':
				return 'border-error/40 bg-error/10';
			case 'warning':
				return 'border-warning/40 bg-warning/10';
			default:
				return 'border-panel-border bg-panel-bg';
		}
	}

	function getSeverityLabel(severity: Notification['severity']): string {
		switch (severity) {
			case 'error':
				return 'Error';
			case 'warning':
				return 'Warning';
			default:
				return 'Info';
		}
	}

	async function runAction(notification: Notification, idx: number) {
		const action = notification.actions[idx];
		if (!action) return;
		try {
			await action.run();
			notificationsStore.markRead(notification.id);
		} catch (error) {
			console.error('Notification action failed:', error);
		}
	}
</script>

{#if $activePopups.length > 0}
	<div class="fixed top-3 right-3 z-[60] flex flex-col gap-2 w-[380px] max-w-[calc(100vw-24px)]">
		{#each $activePopups as n (n.id)}
			<div class="border rounded-lg shadow-xl overflow-hidden {getSeverityClasses(n.severity)}">
				<div class="px-3 py-2 flex items-start justify-between gap-2">
					<div class="min-w-0">
						<div class="text-[11px] uppercase tracking-wide text-gray-400">
							{getSeverityLabel(n.severity)}
						</div>
						<div class="text-sm font-medium text-editor-fg truncate">{n.title}</div>
						<div class="text-sm text-gray-300 mt-0.5 break-words">{n.message}</div>
					</div>
					<button
						class="p-1 rounded hover:bg-panel-active transition-colors flex-shrink-0"
						aria-label="Dismiss notification"
						onclick={() => notificationsStore.dismiss(n.id)}
					>
						<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
						</svg>
					</button>
				</div>

				{#if n.actions.length > 0 || n.detail}
					<div class="px-3 pb-3 flex items-center justify-between gap-2">
						<div class="flex flex-wrap gap-2">
							{#each n.actions as a, idx (a.label)}
								<Button size="sm" onclick={() => runAction(n, idx)}>{a.label}</Button>
							{/each}
						</div>
						{#if n.detail}
							<Button
								size="sm"
								variant="ghost"
								onclick={() => {
									notificationsStore.select(n.id);
									notificationsStore.openCenter();
								}}
							>
								Details
							</Button>
						{/if}
					</div>
				{/if}
			</div>
		{/each}
	</div>
{/if}

