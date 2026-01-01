<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		open: boolean;
		title?: string;
		size?: 'sm' | 'md' | 'lg' | 'xl';
		onclose?: () => void;
		children: Snippet;
		footer?: Snippet;
	}

	let { open = $bindable(false), title, size = 'md', onclose, children, footer }: Props = $props();

	const sizeClasses = {
		sm: 'max-w-sm',
		md: 'max-w-md',
		lg: 'max-w-2xl',
		xl: 'max-w-4xl'
	};

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			open = false;
			onclose?.();
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			open = false;
			onclose?.();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4"
		onclick={handleBackdropClick}
	>
		<div
			class="bg-panel-bg border border-panel-border rounded-lg shadow-xl w-full {sizeClasses[size]} max-h-[90vh] overflow-hidden flex flex-col"
		>
			{#if title}
				<div class="flex items-center justify-between px-4 py-3 border-b border-panel-border flex-shrink-0">
					<h2 class="text-lg font-medium text-editor-fg">{title}</h2>
					<button
						class="p-1 rounded hover:bg-panel-active transition-colors"
						aria-label="Close modal"
						onclick={() => {
							open = false;
							onclose?.();
						}}
					>
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
			{/if}

			<div class="p-4 overflow-y-auto flex-1 min-h-0">
				{@render children()}
			</div>

			{#if footer}
				<div class="px-4 py-3 border-t border-panel-border bg-editor-bg/50 flex-shrink-0">
					{@render footer()}
				</div>
			{/if}
		</div>
	</div>
{/if}
