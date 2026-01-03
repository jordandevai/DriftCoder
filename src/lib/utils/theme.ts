export type ThemeMode = 'dark' | 'light' | 'system';
export type ThemeName = 'dark' | 'light';

export type UiColorKey =
	| 'editor-bg'
	| 'editor-fg'
	| 'editor-line'
	| 'editor-selection'
	| 'editor-cursor'
	| 'panel-bg'
	| 'panel-border'
	| 'panel-active'
	| 'sidebar-bg'
	| 'sidebar-hover'
	| 'sidebar-active'
	| 'tab-bg'
	| 'tab-active'
	| 'tab-border'
	| 'status-bg'
	| 'status-fg'
	| 'accent'
	| 'accent-hover'
	| 'success'
	| 'warning'
	| 'error';

export type UiColors = Record<UiColorKey, string>; // hex strings

export const UI_COLOR_KEYS: UiColorKey[] = [
	'editor-bg',
	'editor-fg',
	'editor-line',
	'editor-selection',
	'editor-cursor',
	'panel-bg',
	'panel-border',
	'panel-active',
	'sidebar-bg',
	'sidebar-hover',
	'sidebar-active',
	'tab-bg',
	'tab-active',
	'tab-border',
	'status-bg',
	'status-fg',
	'accent',
	'accent-hover',
	'success',
	'warning',
	'error'
];

export type TerminalThemeKey =
	| 'background'
	| 'foreground'
	| 'cursor'
	| 'cursorAccent'
	| 'selectionBackground'
	| 'black'
	| 'red'
	| 'green'
	| 'yellow'
	| 'blue'
	| 'magenta'
	| 'cyan'
	| 'white'
	| 'brightBlack'
	| 'brightRed'
	| 'brightGreen'
	| 'brightYellow'
	| 'brightBlue'
	| 'brightMagenta'
	| 'brightCyan'
	| 'brightWhite';

export type TerminalTheme = Record<TerminalThemeKey, string>; // hex strings

export const TERMINAL_THEME_KEYS: TerminalThemeKey[] = [
	'background',
	'foreground',
	'cursor',
	'cursorAccent',
	'selectionBackground',
	'black',
	'red',
	'green',
	'yellow',
	'blue',
	'magenta',
	'cyan',
	'white',
	'brightBlack',
	'brightRed',
	'brightGreen',
	'brightYellow',
	'brightBlue',
	'brightMagenta',
	'brightCyan',
	'brightWhite'
];

export interface ThemeConfig {
	ui: UiColors;
	terminal: TerminalTheme;
	terminalMinimumContrastRatio: number;
}

export interface ThemeOverrides {
	ui?: Partial<UiColors>;
	terminal?: Partial<TerminalTheme>;
	terminalMinimumContrastRatio?: number;
}

export const DEFAULT_DARK_THEME: ThemeConfig = {
	ui: {
		'editor-bg': '#1e1e1e',
		'editor-fg': '#d4d4d4',
		'editor-line': '#2d2d2d',
		'editor-selection': '#264f78',
		'editor-cursor': '#aeafad',
		'panel-bg': '#252526',
		'panel-border': '#3c3c3c',
		'panel-active': '#37373d',
		'sidebar-bg': '#252526',
		'sidebar-hover': '#2a2d2e',
		'sidebar-active': '#37373d',
		'tab-bg': '#2d2d2d',
		'tab-active': '#1e1e1e',
		'tab-border': '#252526',
		'status-bg': '#007acc',
		'status-fg': '#ffffff',
		'accent': '#007acc',
		'accent-hover': '#0098ff',
		'success': '#4caf50',
		'warning': '#ff9800',
		'error': '#f44336'
	},
	terminal: {
		background: '#0f172a',
		foreground: '#e5e7eb',
		cursor: '#f8fafc',
		cursorAccent: '#0f172a',
		selectionBackground: '#334155',
		black: '#0f172a',
		red: '#ef4444',
		green: '#22c55e',
		yellow: '#eab308',
		blue: '#60a5fa',
		magenta: '#a78bfa',
		cyan: '#22d3ee',
		white: '#9ca3af',
		brightBlack: '#475569',
		brightRed: '#f87171',
		brightGreen: '#4ade80',
		brightYellow: '#fde047',
		brightBlue: '#93c5fd',
		brightMagenta: '#c4b5fd',
		brightCyan: '#67e8f9',
		brightWhite: '#e5e7eb'
	},
	terminalMinimumContrastRatio: 4.5
};

export const DEFAULT_LIGHT_THEME: ThemeConfig = {
	ui: {
		'editor-bg': '#f8fafc',
		'editor-fg': '#111827',
		'editor-line': '#e5e7eb',
		'editor-selection': '#bfdbfe',
		'editor-cursor': '#111827',
		'panel-bg': '#ffffff',
		'panel-border': '#e5e7eb',
		'panel-active': '#f3f4f6',
		'sidebar-bg': '#ffffff',
		'sidebar-hover': '#f3f4f6',
		'sidebar-active': '#e5e7eb',
		'tab-bg': '#f3f4f6',
		'tab-active': '#ffffff',
		'tab-border': '#e5e7eb',
		'status-bg': '#2563eb',
		'status-fg': '#ffffff',
		'accent': '#2563eb',
		'accent-hover': '#1d4ed8',
		'success': '#16a34a',
		'warning': '#d97706',
		'error': '#dc2626'
	},
	terminal: {
		background: '#ffffff',
		foreground: '#111827',
		cursor: '#111827',
		cursorAccent: '#ffffff',
		selectionBackground: '#bfdbfe',
		black: '#0b1220',
		red: '#dc2626',
		green: '#16a34a',
		yellow: '#d97706',
		blue: '#2563eb',
		magenta: '#7c3aed',
		cyan: '#0891b2',
		// Keep "white" slightly gray so bg=white segments remain readable.
		white: '#d1d5db',
		brightBlack: '#6b7280',
		brightRed: '#ef4444',
		brightGreen: '#22c55e',
		brightYellow: '#f59e0b',
		brightBlue: '#3b82f6',
		brightMagenta: '#a78bfa',
		brightCyan: '#22d3ee',
		brightWhite: '#f3f4f6'
	},
	terminalMinimumContrastRatio: 4.5
};

export function themeNameFromMode(mode: ThemeMode, prefersDark: boolean): ThemeName {
	if (mode === 'system') return prefersDark ? 'dark' : 'light';
	return mode;
}

export function mergeThemeConfig(base: ThemeConfig, overrides?: ThemeOverrides | null): ThemeConfig {
	if (!overrides) return base;
	return {
		ui: { ...base.ui, ...(overrides.ui ?? {}) },
		terminal: { ...base.terminal, ...(overrides.terminal ?? {}) },
		terminalMinimumContrastRatio:
			typeof overrides.terminalMinimumContrastRatio === 'number'
				? overrides.terminalMinimumContrastRatio
				: base.terminalMinimumContrastRatio
	};
}

export function hexToRgbTriplet(hex: string): string | null {
	const normalized = hex.trim().replace(/^#/, '');
	if (![3, 6].includes(normalized.length)) return null;
	const full =
		normalized.length === 3
			? normalized
					.split('')
					.map((c) => c + c)
					.join('')
			: normalized;

	const r = parseInt(full.slice(0, 2), 16);
	const g = parseInt(full.slice(2, 4), 16);
	const b = parseInt(full.slice(4, 6), 16);
	if ([r, g, b].some((v) => Number.isNaN(v))) return null;
	return `${r} ${g} ${b}`;
}

export function setCssRgbVar(varName: string, hex: string): void {
	const triplet = hexToRgbTriplet(hex);
	if (!triplet) return;
	document.documentElement.style.setProperty(varName, triplet);
}

export function toKebabCase(input: string): string {
	return input.replace(/[A-Z]/g, (m) => `-${m.toLowerCase()}`);
}

export function terminalKeyToCssVar(key: TerminalThemeKey): string {
	return `--c-terminal-${toKebabCase(key)}`;
}

export function normalizeHexColor(raw: string): string | null {
	const v = raw.trim();
	if (!v) return null;
	const withHash = v.startsWith('#') ? v : `#${v}`;
	if (/^#[0-9a-fA-F]{3}$/.test(withHash)) return withHash;
	if (/^#[0-9a-fA-F]{6}$/.test(withHash)) return withHash;
	return null;
}
