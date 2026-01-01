import { get } from 'svelte/store';
import { activeSession, workspaceStore } from '$stores/workspace';
import { fileStore } from '$stores/files';
import { layoutStore, activePanel } from '$stores/layout';
import { terminalStore } from '$stores/terminal';
import { confirmStore } from '$stores/confirm';
import { notificationsStore } from '$stores/notifications';
import { conflictStore } from '$stores/conflict';

function isConflictError(error: unknown): boolean {
	return error instanceof Error && error.message === 'CONFLICT';
}

function notifySaveConflict(path: string) {
	conflictStore.open(path);
	notificationsStore.notifyOnce(
		`save-conflict:${path}`,
		{
			severity: 'warning',
			title: 'Save Conflict',
			message: `This file changed on the server: ${path}`,
			detail: `File: ${path}`,
			actions: [{ label: 'Resolve', run: () => conflictStore.open(path) }]
		},
		30_000
	);
}

function notifySaveFailed(path: string, error: unknown) {
	notificationsStore.notify({
		severity: 'error',
		title: 'Save Failed',
		message: `Failed to save ${path}.`,
		detail: error instanceof Error ? error.message : String(error)
	});
}

export async function saveActiveFile(): Promise<void> {
	const state = get(fileStore);
	const path = state.activeFilePath;
	if (!path) return;

	try {
		await fileStore.saveFile(path);
	} catch (error) {
		if (isConflictError(error)) {
			notifySaveConflict(path);
		} else {
			notifySaveFailed(path, error);
		}
	}
}

export async function saveAllDirtyFilesInActiveSession(): Promise<void> {
	const state = get(fileStore);
	const dirtyPaths = Array.from(state.openFiles.values())
		.filter((f) => f.dirty)
		.map((f) => f.path);

	for (const path of dirtyPaths) {
		try {
			await fileStore.saveFile(path);
		} catch (error) {
			if (isConflictError(error)) {
				notifySaveConflict(path);
			} else {
				notifySaveFailed(path, error);
			}
		}
	}
}

export function toggleFileTree(): void {
	layoutStore.toggleFileTree();
}

export async function newTerminal(): Promise<void> {
	const session = get(activeSession);
	if (!session) return;
	try {
		await terminalStore.createTerminal();
	} catch (error) {
		notificationsStore.notify({
			severity: 'error',
			title: 'Terminal Failed',
			message: 'Could not create a new terminal.',
			detail: error instanceof Error ? error.message : String(error)
		});
	}
}

export async function closeActivePanel(): Promise<void> {
	const session = get(activeSession);
	const panel = get(activePanel);
	if (!session || !panel) return;

	if (panel.type === 'editor' && panel.filePath) {
		const fileState = get(fileStore);
		const file = fileState.openFiles.get(panel.filePath);
		if (file?.dirty) {
			const confirmed = await confirmStore.confirm({
				title: 'Close Tab',
				message: `"${panel.title}" has unsaved changes. Close anyway?`,
				confirmText: 'Close',
				cancelText: 'Cancel',
				destructive: true
			});
			if (!confirmed) return;
		}

		fileStore.closeFile(panel.filePath);
		layoutStore.removePanelForSession(session.id, panel.id);
		return;
	}

	if (panel.type === 'terminal' && panel.terminalId) {
		try {
			await terminalStore.closeTerminal(panel.terminalId);
		} catch (error) {
			notificationsStore.notify({
				severity: 'error',
				title: 'Terminal Close Failed',
				message: 'Could not close the active terminal.',
				detail: error instanceof Error ? error.message : String(error)
			});
		} finally {
			layoutStore.removePanelForSession(session.id, panel.id);
		}
	}
}

export async function closeActiveTerminalPanel(): Promise<void> {
	const session = get(activeSession);
	const panel = get(activePanel);
	if (!session || !panel || panel.type !== 'terminal' || !panel.terminalId) return;

	try {
		await terminalStore.closeTerminal(panel.terminalId);
	} catch (error) {
		notificationsStore.notify({
			severity: 'error',
			title: 'Terminal Close Failed',
			message: 'Could not close the active terminal.',
			detail: error instanceof Error ? error.message : String(error)
		});
	} finally {
		layoutStore.removePanelForSession(session.id, panel.id);
	}
}

export async function closeActiveProject(): Promise<void> {
	const session = get(activeSession);
	if (!session) return;

	const hasUnsaved = Array.from(session.fileState.openFiles.values()).some((f) => f.dirty);
	if (hasUnsaved) {
		const confirmed = await confirmStore.confirm({
			title: 'Close Project',
			message: `"${session.displayName}" has unsaved changes. Close anyway?`,
			confirmText: 'Close',
			cancelText: 'Cancel',
			destructive: true
		});
		if (!confirmed) return;
	}

	await workspaceStore.closeSession(session.id);
}
