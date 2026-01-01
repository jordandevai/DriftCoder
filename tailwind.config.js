/** @type {import('tailwindcss').Config} */
export default {
	content: ['./src/**/*.{html,js,svelte,ts}'],
	theme: {
		extend: {
			colors: {
				// Editor theme colors
				'editor-bg': '#1e1e1e',
				'editor-fg': '#d4d4d4',
				'editor-line': '#2d2d2d',
				'editor-selection': '#264f78',
				'editor-cursor': '#aeafad',
				// Panel colors
				'panel-bg': '#252526',
				'panel-border': '#3c3c3c',
				'panel-active': '#37373d',
				// Sidebar colors
				'sidebar-bg': '#252526',
				'sidebar-hover': '#2a2d2e',
				'sidebar-active': '#37373d',
				// Tab colors
				'tab-bg': '#2d2d2d',
				'tab-active': '#1e1e1e',
				'tab-border': '#252526',
				// Status bar
				'status-bg': '#007acc',
				'status-fg': '#ffffff',
				// Accent colors
				'accent': '#007acc',
				'accent-hover': '#0098ff',
				'success': '#4caf50',
				'warning': '#ff9800',
				'error': '#f44336'
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
	plugins: []
};
