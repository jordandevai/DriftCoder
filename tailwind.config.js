/** @type {import('tailwindcss').Config} */
export default {
	content: ['./src/**/*.{html,js,svelte,ts}'],
	theme: {
		extend: {
			colors: {
				// Editor theme colors
				'editor-bg': 'rgb(var(--c-editor-bg) / <alpha-value>)',
				'editor-fg': 'rgb(var(--c-editor-fg) / <alpha-value>)',
				'editor-line': 'rgb(var(--c-editor-line) / <alpha-value>)',
				'editor-selection': 'rgb(var(--c-editor-selection) / <alpha-value>)',
				'editor-cursor': 'rgb(var(--c-editor-cursor) / <alpha-value>)',
				// Panel colors
				'panel-bg': 'rgb(var(--c-panel-bg) / <alpha-value>)',
				'panel-border': 'rgb(var(--c-panel-border) / <alpha-value>)',
				'panel-active': 'rgb(var(--c-panel-active) / <alpha-value>)',
				// Sidebar colors
				'sidebar-bg': 'rgb(var(--c-sidebar-bg) / <alpha-value>)',
				'sidebar-hover': 'rgb(var(--c-sidebar-hover) / <alpha-value>)',
				'sidebar-active': 'rgb(var(--c-sidebar-active) / <alpha-value>)',
				// Tab colors
				'tab-bg': 'rgb(var(--c-tab-bg) / <alpha-value>)',
				'tab-active': 'rgb(var(--c-tab-active) / <alpha-value>)',
				'tab-border': 'rgb(var(--c-tab-border) / <alpha-value>)',
				// Status bar
				'status-bg': 'rgb(var(--c-status-bg) / <alpha-value>)',
				'status-fg': 'rgb(var(--c-status-fg) / <alpha-value>)',
				// Accent colors
				'accent': 'rgb(var(--c-accent) / <alpha-value>)',
				'accent-hover': 'rgb(var(--c-accent-hover) / <alpha-value>)',
				'success': 'rgb(var(--c-success) / <alpha-value>)',
				'warning': 'rgb(var(--c-warning) / <alpha-value>)',
				'error': 'rgb(var(--c-error) / <alpha-value>)'
			},
			fontFamily: {
				mono: ['JetBrains Mono', 'Fira Code', 'Consolas', 'Monaco', 'monospace'],
				sans: ['Inter', 'system-ui', 'sans-serif']
			},
			fontSize: {
				'2xs': '0.625rem'
			}
		}
	},
	plugins: [
		function({ addVariant }) {
			addVariant('touch-device', '@media (pointer: coarse)');
			addVariant('mouse-device', '@media (pointer: fine)');
		}
	]
};
