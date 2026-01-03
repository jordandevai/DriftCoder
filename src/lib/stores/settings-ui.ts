import { writable } from 'svelte/store';

interface SettingsUiState {
	open: boolean;
}

const initialState: SettingsUiState = { open: false };

function createSettingsUiStore() {
	const { subscribe, set, update } = writable<SettingsUiState>(initialState);

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

export const settingsUiStore = createSettingsUiStore();

