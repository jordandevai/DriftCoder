import { writable, get } from 'svelte/store';
import type { SettingsState } from '$types';
import { isTauri } from '$utils/tauri';
import { loadSavedSettings, saveSettings } from '$utils/storage';
import {
	DEFAULT_DARK_THEME,
	DEFAULT_LIGHT_THEME,
	mergeThemeConfig,
	terminalKeyToCssVar,
	setCssRgbVar,
	themeNameFromMode,
	type ThemeMode,
	type ThemeName,
	type UiColorKey,
	type TerminalThemeKey
} from '$utils/theme';

const defaultSettings: SettingsState = {
	fontSize: 14,
	tabSize: 4,
	wordWrap: false,
	autosave: false,
	autosaveDelay: 1000,
	terminalScrollback: 50_000,
	themeMode: 'dark',
	themeOverrides: {}
};

function createSettingsStore() {
	const { subscribe, set, update } = writable<SettingsState>(defaultSettings);
	let savingTimer: number | null = null;
	let initialized = false;
	let unsubscribePersist: (() => void) | null = null;
	let prefersDark = true;
	let unsubscribeApply: (() => void) | null = null;
	let current = defaultSettings;

	function applyTheme(settings: SettingsState) {
		if (typeof window === 'undefined') return;

		const mode = settings.themeMode as ThemeMode;
		const themeName: ThemeName = themeNameFromMode(mode, prefersDark);

		const base = themeName === 'dark' ? DEFAULT_DARK_THEME : DEFAULT_LIGHT_THEME;
		const overrides = settings.themeOverrides?.[themeName] ?? null;
		const theme = mergeThemeConfig(base, overrides);

		document.documentElement.dataset.theme = themeName;

		for (const [key, hex] of Object.entries(theme.ui)) {
			setCssRgbVar(`--c-${key}`, hex);
		}

		// Expose terminal theme as CSS vars for components that want them
		for (const [key, hex] of Object.entries(theme.terminal)) {
			setCssRgbVar(terminalKeyToCssVar(key as TerminalThemeKey), hex);
		}
		document.documentElement.style.setProperty(
			'--terminal-min-contrast',
			String(theme.terminalMinimumContrastRatio)
		);
	}

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

			// Track system theme if needed.
			if (typeof window !== 'undefined') {
				const media = window.matchMedia?.('(prefers-color-scheme: dark)');
				prefersDark = media?.matches ?? true;
				media?.addEventListener?.('change', (e) => {
					prefersDark = e.matches;
					applyTheme(current);
				});
			}

			// Apply theme immediately and on changes.
			unsubscribeApply = subscribe((s) => {
				current = s;
				applyTheme(s);
			});
			current = get({ subscribe });
			applyTheme(current);
		},

		getEffectiveThemeName(): ThemeName {
			const s = current;
			return themeNameFromMode(s.themeMode as ThemeMode, prefersDark);
		},

		setThemeMode(mode: ThemeMode): void {
			update((s) => ({ ...s, themeMode: mode }));
		},

		setThemeUiColor(theme: ThemeName, key: UiColorKey, hex: string): void {
			update((s) => {
				const existing = s.themeOverrides?.[theme] ?? {};
				const ui = { ...(existing.ui ?? {}), [key]: hex };
				return {
					...s,
					themeOverrides: { ...s.themeOverrides, [theme]: { ...existing, ui } }
				};
			});
		},

		setThemeTerminalColor(theme: ThemeName, key: TerminalThemeKey, hex: string): void {
			update((s) => {
				const existing = s.themeOverrides?.[theme] ?? {};
				const terminal = { ...(existing.terminal ?? {}), [key]: hex };
				return {
					...s,
					themeOverrides: { ...s.themeOverrides, [theme]: { ...existing, terminal } }
				};
			});
		},

		setThemeTerminalContrast(theme: ThemeName, ratio: number): void {
			update((s) => {
				const existing = s.themeOverrides?.[theme] ?? {};
				return {
					...s,
					themeOverrides: {
						...s.themeOverrides,
						[theme]: { ...existing, terminalMinimumContrastRatio: Math.max(1, Math.min(21, ratio)) }
					}
				};
			});
		},

		resetTheme(theme: ThemeName): void {
			update((s) => {
				const next = { ...s.themeOverrides };
				delete next[theme];
				return { ...s, themeOverrides: next };
			});
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
			if (unsubscribeApply) {
				unsubscribeApply();
				unsubscribeApply = null;
			}
			initialized = false;
		}
	};
}

export const settingsStore = createSettingsStore();
