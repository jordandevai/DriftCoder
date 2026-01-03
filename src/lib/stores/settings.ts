import { writable } from 'svelte/store';
import type { SettingsState } from '$types';
import { isTauri } from '$utils/tauri';
import { loadSavedSettings, saveSettings } from '$utils/storage';

const defaultSettings: SettingsState = {
	fontSize: 14,
	tabSize: 4,
	wordWrap: false,
	autosave: false,
	autosaveDelay: 1000,
	terminalScrollback: 50_000
};

function createSettingsStore() {
	const { subscribe, set, update } = writable<SettingsState>(defaultSettings);
	let savingTimer: number | null = null;
	let initialized = false;
	let unsubscribePersist: (() => void) | null = null;

	function schedulePersist(next: SettingsState) {
		if (!isTauri()) return;
		if (!initialized) return;
		if (typeof window === 'undefined') return;

		if (savingTimer !== null) window.clearTimeout(savingTimer);
		savingTimer = window.setTimeout(() => {
			void saveSettings(next);
		}, 300);
	}

	return {
		subscribe,

		async init(): Promise<void> {
			if (initialized) return;
			initialized = true;

			if (isTauri()) {
				const loaded = await loadSavedSettings();
				if (loaded) {
					set({ ...defaultSettings, ...loaded });
				}

				// Persist changes (debounced) after init completes.
				unsubscribePersist = subscribe((s) => schedulePersist(s));
			}
		},

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

		setTerminalScrollback(lines: number): void {
			update((s) => ({
				...s,
				terminalScrollback: Math.max(1_000, Math.min(200_000, Math.floor(lines || 0)))
			}));
		},

		loadSettings(settings: Partial<SettingsState>): void {
			update((s) => ({ ...s, ...settings }));
		},

		reset(): void {
			set(defaultSettings);
			if (unsubscribePersist) {
				unsubscribePersist();
				unsubscribePersist = null;
			}
			initialized = false;
		}
	};
}

export const settingsStore = createSettingsStore();
