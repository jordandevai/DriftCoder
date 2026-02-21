import { writable } from 'svelte/store';

interface SettingsUiState {
	open: boolean;
	terminalFullscreen: boolean;
}

const initialState: SettingsUiState = { open: false, terminalFullscreen: false };

function createSettingsUiStore() {
	const { subscribe, set, update } = writable<SettingsUiState>(initialState);

	return {
		subscribe,
		open(): void {
			update((s) => ({ ...s, open: true }));
		},
		close(): void {
			update((s) => ({ ...s, open: false }));
		},
		toggle(): void {
			update((s) => ({ ...s, open: !s.open }));
		},
		setTerminalFullscreen(fullscreen: boolean): void {
			update((s) => ({ ...s, terminalFullscreen: fullscreen }));
		},
		toggleTerminalFullscreen(): void {
			update((s) => ({ ...s, terminalFullscreen: !s.terminalFullscreen }));
		}
	};
}

export const settingsUiStore = createSettingsUiStore();
