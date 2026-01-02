/**
 * Debug store for connection tracing.
 *
 * Enable tracing to see real-time connection diagnostics in the notification area.
 * Useful for debugging connection issues on mobile/LAN networks.
 */
import { writable, derived, get } from 'svelte/store';
import { invoke, isTauri, listen } from '$utils/tauri';
import { notificationsStore } from './notifications';

export interface TraceEvent {
	timestamp: number;
	category: string;
	step: string;
	correlationId?: string;
	message: string;
	detail?: string;
	data?: unknown;
	isError: boolean;
}

interface DebugState {
	traceEnabled: boolean;
	traces: TraceEvent[];
	maxTraces: number;
}

const initialState: DebugState = {
	traceEnabled: false,
	traces: [],
	maxTraces: 100 // Keep last 100 traces in memory
};

function createDebugStore() {
	const { subscribe, set, update } = writable<DebugState>(initialState);
	let unlistenTrace: (() => void) | null = null;

	return {
		subscribe,

		/**
		 * Enable connection tracing
		 */
		async enableTrace(): Promise<void> {
			try {
				await invoke<boolean>('debug_enable_trace');
				update((s) => ({ ...s, traceEnabled: true, traces: [] }));

				// Set up listener if not already
				if (isTauri() && !unlistenTrace) {
					unlistenTrace = await listen<TraceEvent>('connection_trace', (event) => {
						const trace = event;

						// Add to trace history
						update((s) => {
							const newTraces = [...s.traces, trace].slice(-s.maxTraces);
							return { ...s, traces: newTraces };
						});

						// Intentionally do not surface every trace as a notification.
					});
				}

				notificationsStore.notify({
					severity: 'info',
					title: 'Tracing Enabled',
					message: 'Connection trace events will appear here in real-time.'
				});
			} catch (error) {
				console.error('Failed to enable trace:', error);
				notificationsStore.notify({
					severity: 'error',
					title: 'Trace Error',
					message: 'Failed to enable connection tracing.',
					detail: error instanceof Error ? error.message : String(error)
				});
			}
		},

		/**
		 * Disable connection tracing
		 */
		async disableTrace(): Promise<void> {
			try {
				await invoke<boolean>('debug_disable_trace');
				update((s) => ({ ...s, traceEnabled: false }));

				notificationsStore.notify({
					severity: 'info',
					title: 'Tracing Disabled',
					message: 'Connection tracing has been disabled.'
				});
			} catch (error) {
				console.error('Failed to disable trace:', error);
			}
		},

		/**
		 * Toggle trace on/off
		 */
		async toggleTrace(): Promise<void> {
			const state = get({ subscribe });
			if (state.traceEnabled) {
				await this.disableTrace();
			} else {
				await this.enableTrace();
			}
		},

		/**
		 * Check if tracing is currently enabled
		 */
		async checkTraceEnabled(): Promise<boolean> {
			try {
				const enabled = await invoke<boolean>('debug_is_trace_enabled');
				update((s) => ({ ...s, traceEnabled: enabled }));
				return enabled;
			} catch {
				return false;
			}
		},

		/**
		 * Clear trace history
		 */
		clearTraces(): void {
			update((s) => ({ ...s, traces: [] }));
		},

		/**
		 * Initialize the debug store
		 */
		async init(): Promise<void> {
			// Check if tracing was already enabled (e.g., via env var)
			await this.checkTraceEnabled();

			// If tracing is enabled, set up the listener
			const state = get({ subscribe });
			if (state.traceEnabled && isTauri() && !unlistenTrace) {
				unlistenTrace = await listen<TraceEvent>('connection_trace', (event) => {
					const trace = event;

					update((s) => {
						const newTraces = [...s.traces, trace].slice(-s.maxTraces);
						return { ...s, traces: newTraces };
					});

					// Intentionally do not surface every trace as a notification.
				});
			}
		},

		/**
		 * Clean up listeners
		 */
		destroy(): void {
			if (unlistenTrace) {
				unlistenTrace();
				unlistenTrace = null;
			}
		},

		reset(): void {
			this.destroy();
			set(initialState);
		}
	};
}

export const debugStore = createDebugStore();

// Derived stores
export const isTraceEnabled = derived(debugStore, ($store) => $store.traceEnabled);
export const traceHistory = derived(debugStore, ($store) => $store.traces);
