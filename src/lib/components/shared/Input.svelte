<script lang="ts">
	interface Props {
		type?: 'text' | 'password' | 'number' | 'email';
		value?: string;
		placeholder?: string;
		label?: string;
		error?: string;
		disabled?: boolean;
		required?: boolean;
		id?: string;
		name?: string;
		oninput?: (e: Event) => void;
		onchange?: (e: Event) => void;
	}

	let {
		type = 'text',
		value = $bindable(''),
		placeholder = '',
		label,
		error,
		disabled = false,
		required = false,
		id,
		name,
		oninput,
		onchange
	}: Props = $props();

	const generatedId = `input-${Math.random().toString(36).slice(2)}`;
	const inputId = $derived(id || generatedId);
</script>

<div class="flex flex-col gap-1">
	{#if label}
		<label for={inputId} class="text-sm text-gray-400">
			{label}
			{#if required}
				<span class="text-error">*</span>
			{/if}
		</label>
	{/if}
	<input
		{type}
		id={inputId}
		{name}
		{placeholder}
		{disabled}
		{required}
		bind:value
		{oninput}
		{onchange}
		class="w-full px-3 py-2 bg-editor-bg border rounded text-editor-fg placeholder-gray-500 focus:outline-none focus:ring-1 transition-colors duration-150
			{error ? 'border-error focus:border-error focus:ring-error' : 'border-panel-border focus:border-accent focus:ring-accent'}
			{disabled ? 'opacity-50 cursor-not-allowed' : ''}"
	/>
	{#if error}
		<span class="text-xs text-error">{error}</span>
	{/if}
</div>
