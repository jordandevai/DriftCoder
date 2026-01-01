<script lang="ts">
	// DEPRECATED: This component is no longer used. Use FolderSelectEmbedded instead.
	// Keeping for backwards compatibility during migration.

	interface Props {
		onselect: (path: string) => void;
		oncancel?: () => void;
	}

	let { onselect, oncancel }: Props = $props();
</script>

<div class="h-full flex flex-col items-center justify-center p-8">
	<div class="text-center">
		<p class="text-gray-400 mb-4">This component is deprecated. Please use FolderSelectEmbedded instead.</p>
		{#if oncancel}
			<button class="text-accent hover:underline" onclick={oncancel}>Go Back</button>
		{/if}
	</div>
</div>
