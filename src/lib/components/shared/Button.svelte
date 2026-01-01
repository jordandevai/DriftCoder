<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
		size?: 'sm' | 'md' | 'lg';
		disabled?: boolean;
		loading?: boolean;
		type?: 'button' | 'submit' | 'reset';
		onclick?: (e: MouseEvent) => void;
		children: Snippet;
	}

	let {
		variant = 'primary',
		size = 'md',
		disabled = false,
		loading = false,
		type = 'button',
		onclick,
		children
	}: Props = $props();

	const variantClasses = {
		primary: 'bg-accent text-white hover:bg-accent-hover',
		secondary: 'bg-panel-bg text-editor-fg border border-panel-border hover:bg-panel-active',
		ghost: 'bg-transparent text-editor-fg hover:bg-panel-active',
		danger: 'bg-error text-white hover:bg-red-600'
	};

	const sizeClasses = {
		sm: 'px-2 py-1 text-xs',
		md: 'px-4 py-2 text-sm',
		lg: 'px-6 py-3 text-base'
	};
</script>

<button
	{type}
	class="inline-flex items-center justify-center gap-2 rounded font-medium transition-colors duration-150 focus:outline-none focus:ring-2 focus:ring-accent focus:ring-offset-2 focus:ring-offset-editor-bg disabled:opacity-50 disabled:cursor-not-allowed {variantClasses[
		variant
	]} {sizeClasses[size]}"
	disabled={disabled || loading}
	{onclick}
>
	{#if loading}
		<svg class="animate-spin h-4 w-4" viewBox="0 0 24 24">
			<circle
				class="opacity-25"
				cx="12"
				cy="12"
				r="10"
				stroke="currentColor"
				stroke-width="4"
				fill="none"
			/>
			<path
				class="opacity-75"
				fill="currentColor"
				d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
			/>
		</svg>
	{/if}
	{@render children()}
</button>
