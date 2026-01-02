import { writable } from 'svelte/store';

interface DiagnosticsState {
	open: boolean;
}

const initialState: DiagnosticsState = { open: false };

function createDiagnosticsStore() {
	const { subscribe, set, update } = writable<DiagnosticsState>(initialState);

	return {
		subscribe,
		open(): void {
			set({ open: true });
		},
		close(): void {
			set({ open: false });
		},
		toggle(): void {
			update((s) => ({ open: !s.open }));
		}
	};
}

export const diagnosticsStore = createDiagnosticsStore();

