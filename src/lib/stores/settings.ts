import { writable } from 'svelte/store';
import type { SettingsState } from '$types';

const defaultSettings: SettingsState = {
	fontSize: 14,
	tabSize: 4,
	wordWrap: false,
	autosave: false,
	autosaveDelay: 1000
};

function createSettingsStore() {
	const { subscribe, set, update } = writable<SettingsState>(defaultSettings);

	return {
		subscribe,

		setFontSize(size: number): void {
			update((s) => ({ ...s, fontSize: Math.max(10, Math.min(24, size)) }));
		},

		setTabSize(size: number): void {
			update((s) => ({ ...s, tabSize: Math.max(2, Math.min(8, size)) }));
		},

		toggleWordWrap(): void {
			update((s) => ({ ...s, wordWrap: !s.wordWrap }));
		},

		toggleAutosave(): void {
			update((s) => ({ ...s, autosave: !s.autosave }));
		},

		setAutosaveDelay(delay: number): void {
			update((s) => ({ ...s, autosaveDelay: Math.max(500, Math.min(5000, delay)) }));
		},

		loadSettings(settings: Partial<SettingsState>): void {
			update((s) => ({ ...s, ...settings }));
		},

		reset(): void {
			set(defaultSettings);
		}
	};
}

export const settingsStore = createSettingsStore();
