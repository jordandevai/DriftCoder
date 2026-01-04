<script lang="ts">
	import Modal from '$components/shared/Modal.svelte';
	import Button from '$components/shared/Button.svelte';
	import { settingsStore } from '$stores/settings';
	import { settingsUiStore } from '$stores/settings-ui';
	import {
		DEFAULT_DARK_THEME,
		DEFAULT_LIGHT_THEME,
		mergeThemeConfig,
		normalizeHexColor,
		UI_COLOR_KEYS,
		TERMINAL_THEME_KEYS,
		type ThemeMode,
		type ThemeName,
		type UiColorKey,
		type TerminalThemeKey
	} from '$utils/theme';

	const open = $derived($settingsUiStore.open);
	const scrollback = $derived($settingsStore.terminalScrollback);
	const terminalPersistence = $derived($settingsStore.terminalSessionPersistence);
	const tmuxPrefix = $derived($settingsStore.terminalTmuxSessionPrefix);
	const wordWrap = $derived($settingsStore.wordWrap);
	const themeMode = $derived($settingsStore.themeMode);
	const themeOverrides = $derived($settingsStore.themeOverrides);

	let editTheme = $state<ThemeName>('dark');
	let uiFilter = $state('');
	let terminalFilter = $state('');

	const effectiveTheme = $derived.by(() => {
		themeMode;
		return settingsStore.getEffectiveThemeName();
	});

	$effect(() => {
		if (!open) return;
		editTheme = effectiveTheme;
	});

	const editConfig = $derived.by(() => {
		themeOverrides;
		const base = editTheme === 'dark' ? DEFAULT_DARK_THEME : DEFAULT_LIGHT_THEME;
		const overrides = themeOverrides?.[editTheme] ?? null;
		return mergeThemeConfig(base, overrides);
	});

	const filteredUiKeys = $derived.by(() => {
		const f = uiFilter.trim().toLowerCase();
		if (!f) return UI_COLOR_KEYS;
		return UI_COLOR_KEYS.filter((k) => k.toLowerCase().includes(f));
	});

	const filteredTerminalKeys = $derived.by(() => {
		const f = terminalFilter.trim().toLowerCase();
		if (!f) return TERMINAL_THEME_KEYS;
		return TERMINAL_THEME_KEYS.filter((k) => k.toLowerCase().includes(f));
	});

	function setUiColor(key: UiColorKey, raw: string) {
		const normalized = normalizeHexColor(raw);
		if (!normalized) return;
		settingsStore.setThemeUiColor(editTheme, key, normalized);
	}

	function setTerminalColor(key: TerminalThemeKey, raw: string) {
		const normalized = normalizeHexColor(raw);
		if (!normalized) return;
		settingsStore.setThemeTerminalColor(editTheme, key, normalized);
	}
</script>

<Modal open={open} title="Settings" size="lg" onclose={() => settingsUiStore.close()}>
	<div class="space-y-6">
		<div>
			<div class="text-sm font-medium text-editor-fg">Appearance</div>
			<div class="text-xs text-editor-fg/70 mt-1">Applies immediately across the app.</div>

			<div class="mt-3 flex flex-wrap items-center gap-3">
				<label class="text-sm text-editor-fg/80" for="settings-theme-mode">Theme</label>
				<select
					id="settings-theme-mode"
					class="input w-44"
					value={themeMode}
					onchange={(e) =>
						settingsStore.setThemeMode((e.currentTarget as HTMLSelectElement).value as ThemeMode)}
				>
					<option value="system">System</option>
					<option value="dark">Dark</option>
					<option value="light">Light</option>
				</select>
				<div class="text-xs text-editor-fg/60">
					Effective: <span class="font-medium">{effectiveTheme}</span>
				</div>
			</div>
		</div>

		<div>
			<div class="text-sm font-medium text-editor-fg">Editor</div>
			<div class="text-xs text-editor-fg/70 mt-1">Code editor behavior settings.</div>

			<div class="mt-3 flex items-center gap-3">
				<label class="flex items-center gap-2 cursor-pointer">
					<input
						type="checkbox"
						class="w-4 h-4 rounded border-panel-border accent-accent"
						checked={wordWrap}
						onchange={() => settingsStore.toggleWordWrap()}
					/>
					<span class="text-sm text-editor-fg/80">Word Wrap</span>
				</label>
				<div class="text-xs text-editor-fg/60">Wrap long lines instead of horizontal scrolling</div>
			</div>
		</div>

		<div>
			<div class="text-sm font-medium text-editor-fg">Terminal</div>
			<div class="text-xs text-editor-fg/70 mt-1">
				Higher values keep more history but use more memory. Applies immediately; existing history can't be restored.
			</div>

			<div class="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2">
				<div class="flex items-center gap-3">
					<label class="text-sm text-editor-fg/80" for="settings-terminal-persistence">Session persistence</label>
					<select
						id="settings-terminal-persistence"
						class="input w-32"
						value={terminalPersistence}
						onchange={(e) =>
							settingsStore.setTerminalSessionPersistence(
								(e.currentTarget as HTMLSelectElement).value as 'none' | 'tmux'
							)}
					>
						<option value="none">None</option>
						<option value="tmux">tmux</option>
					</select>
				</div>
				<div class="flex items-center gap-3">
					<label class="text-sm text-editor-fg/80" for="settings-terminal-tmux-prefix">tmux prefix</label>
					<input
						id="settings-terminal-tmux-prefix"
						class="input w-32"
						value={tmuxPrefix}
						disabled={terminalPersistence !== 'tmux'}
						oninput={(e) =>
							settingsStore.setTerminalTmuxSessionPrefix((e.currentTarget as HTMLInputElement).value)}
					/>
				</div>
			</div>

			<div class="mt-2 text-xs text-editor-fg/60">
				If enabled, terminals attach to a stable remote tmux session so they survive reconnects/backgrounding. Requires tmux on the server.
			</div>

			<div class="mt-3 flex items-center gap-3">
				<label class="text-sm text-editor-fg/80" for="settings-terminal-scrollback">Scrollback</label>
				<input
					id="settings-terminal-scrollback"
					class="input w-32"
					type="number"
					min="1000"
					max="200000"
					step="1000"
					value={scrollback}
					oninput={(e) =>
						settingsStore.setTerminalScrollback(Number((e.currentTarget as HTMLInputElement).value))}
				/>
				<div class="text-xs text-editor-fg/60">Default: 50000</div>
			</div>
		</div>

		<details class="panel p-4">
			<summary class="cursor-pointer select-none text-sm text-editor-fg">
				Theme editor <span class="text-xs text-editor-fg/60">(advanced)</span>
			</summary>

			<div class="mt-4 space-y-6">
				<div class="flex flex-wrap items-center justify-between gap-3">
					<div class="flex items-center gap-2">
						<label class="text-sm text-editor-fg/80" for="settings-theme-edit">Edit</label>
						<select
							id="settings-theme-edit"
							class="input w-32"
							value={editTheme}
							onchange={(e) => (editTheme = (e.currentTarget as HTMLSelectElement).value as ThemeName)}
						>
							<option value="dark">Dark</option>
							<option value="light">Light</option>
						</select>
					</div>
					<div class="flex items-center gap-2">
						<Button size="sm" variant="ghost" onclick={() => settingsStore.resetTheme(editTheme)}>Reset {editTheme}</Button>
					</div>
				</div>

				<div class="grid grid-cols-1 gap-3">
					<div class="text-sm font-medium text-editor-fg">UI colors</div>
					<input class="input" placeholder="Filter (e.g. editor, sidebar, accent)" value={uiFilter} oninput={(e) => (uiFilter = (e.currentTarget as HTMLInputElement).value)} />

					<div class="grid grid-cols-1 gap-2 max-h-[34vh] overflow-auto pr-1">
						{#each filteredUiKeys as key (key)}
							<div class="flex items-center gap-3 rounded border border-panel-border bg-panel-bg px-3 py-2">
								<div class="w-48 text-xs font-mono text-editor-fg/80">{key}</div>
								<input
									type="color"
									class="h-8 w-10 rounded border border-panel-border bg-transparent"
									value={editConfig.ui[key]}
									oninput={(e) => setUiColor(key as UiColorKey, (e.currentTarget as HTMLInputElement).value)}
								/>
								<input
									class="input font-mono text-xs flex-1"
									value={editConfig.ui[key]}
									onchange={(e) => setUiColor(key as UiColorKey, (e.currentTarget as HTMLInputElement).value)}
								/>
							</div>
						{/each}
					</div>
				</div>

				<div class="grid grid-cols-1 gap-3">
					<div class="text-sm font-medium text-editor-fg">Terminal colors</div>
					<input class="input" placeholder="Filter (e.g. background, cursor, bright)" value={terminalFilter} oninput={(e) => (terminalFilter = (e.currentTarget as HTMLInputElement).value)} />

					<div class="flex items-center gap-3 rounded border border-panel-border bg-panel-bg px-3 py-2">
						<div class="w-48 text-xs font-mono text-editor-fg/80">minimumContrastRatio</div>
						<input
							class="input w-32 font-mono text-xs"
							type="number"
							min="1"
							max="21"
							step="0.1"
							value={editConfig.terminalMinimumContrastRatio}
							onchange={(e) =>
								settingsStore.setThemeTerminalContrast(
									editTheme,
									Number((e.currentTarget as HTMLInputElement).value || 0)
								)}
						/>
						<div class="text-xs text-editor-fg/60">Try 4.5–7.0 for small text</div>
					</div>

					<div class="grid grid-cols-1 gap-2 max-h-[34vh] overflow-auto pr-1">
						{#each filteredTerminalKeys as key (key)}
							<div class="flex items-center gap-3 rounded border border-panel-border bg-panel-bg px-3 py-2">
								<div class="w-48 text-xs font-mono text-editor-fg/80">{key}</div>
								<input
									type="color"
									class="h-8 w-10 rounded border border-panel-border bg-transparent"
									value={editConfig.terminal[key]}
									oninput={(e) =>
										setTerminalColor(key as TerminalThemeKey, (e.currentTarget as HTMLInputElement).value)}
								/>
								<input
									class="input font-mono text-xs flex-1"
									value={editConfig.terminal[key]}
									onchange={(e) =>
										setTerminalColor(key as TerminalThemeKey, (e.currentTarget as HTMLInputElement).value)}
								/>
							</div>
						{/each}
					</div>
				</div>
			</div>
		</details>

		<div class="flex justify-end gap-2 pt-2">
			<Button variant="ghost" onclick={() => settingsUiStore.close()}>Close</Button>
		</div>
	</div>
</Modal>
