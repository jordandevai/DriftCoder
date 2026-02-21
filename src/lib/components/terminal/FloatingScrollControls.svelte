<script lang="ts">
	interface Props {
		disabled?: boolean;
		onPageUp: () => void;
		onPageDown: () => void;
	}

	let { disabled = false, onPageUp, onPageDown }: Props = $props();

	let lastPointerActivationAt = 0;
	let lastActivationAt = 0;
	const CLICK_FALLBACK_MS = 300;
	const ACTIVATION_DEDUPE_MS = 80;

	function nowMs(): number {
		if (typeof performance !== 'undefined' && typeof performance.now === 'function') {
			return performance.now();
		}
		return Date.now();
	}

	function activate(action: () => void): void {
		const now = nowMs();
		if (now - lastActivationAt < ACTIVATION_DEDUPE_MS) return;
		lastActivationAt = now;
		if (disabled) return;
		action();
	}

	function handlePointerDown(e: PointerEvent, action: () => void): void {
		e.preventDefault();
		lastPointerActivationAt = nowMs();
		activate(action);
	}

	function handleTouchStart(e: TouchEvent, action: () => void): void {
		e.preventDefault();
		lastPointerActivationAt = nowMs();
		activate(action);
	}

	function handleClick(action: () => void): void {
		const now = nowMs();
		if (now - lastPointerActivationAt < CLICK_FALLBACK_MS) return;
		activate(action);
	}
</script>

<!-- Floating scroll controls - visible only on touch devices -->
<div
	class="absolute right-2 top-1/2 -translate-y-1/2 z-10 flex flex-col gap-2 hidden touch-device:flex pointer-events-auto"
	style="touch-action: manipulation;"
	role="toolbar"
	aria-label="Scroll controls"
>
	<button
		class="w-11 h-11 flex items-center justify-center rounded-lg bg-white/10 hover:bg-white/20 active:bg-white/30 transition-colors disabled:opacity-40 shadow-lg backdrop-blur-sm"
		disabled={disabled}
		title="Page Up"
		aria-label="Page Up - scroll terminal history"
		onpointerdown={(e) => handlePointerDown(e, onPageUp)}
		ontouchstart={(e) => handleTouchStart(e, onPageUp)}
		onclick={() => handleClick(onPageUp)}
	>
		<svg class="w-5 h-5 text-gray-100" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
		</svg>
	</button>
	<button
		class="w-11 h-11 flex items-center justify-center rounded-lg bg-white/10 hover:bg-white/20 active:bg-white/30 transition-colors disabled:opacity-40 shadow-lg backdrop-blur-sm"
		disabled={disabled}
		title="Page Down"
		aria-label="Page Down - scroll terminal history"
		onpointerdown={(e) => handlePointerDown(e, onPageDown)}
		ontouchstart={(e) => handleTouchStart(e, onPageDown)}
		onclick={() => handleClick(onPageDown)}
	>
		<svg class="w-5 h-5 text-gray-100" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
		</svg>
	</button>
</div>
