import { derived, get } from 'svelte/store';
import type { FileState, FileEntry, OpenFile, SessionFileState } from '$types';
import { invoke } from '$utils/tauri';
import { workspaceStore, activeSession } from './workspace';
import { detectLanguage } from '$utils/languages';

// Initial state for when no session is active
const emptyState: FileState = {
	projectRoot: '',
	tree: [],
	expandedPaths: new Set(),
	openFiles: new Map(),
	activeFilePath: null
};

// Sort entries: directories first, then alphabetically
function sortEntries(entries: FileEntry[]): FileEntry[] {
	return [...entries].sort((a, b) => {
		if (a.isDirectory && !b.isDirectory) return -1;
		if (!a.isDirectory && b.isDirectory) return 1;
		return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
	});
}

// Add children to a specific directory in the tree
function addChildrenToTree(tree: FileEntry[], parentPath: string, children: FileEntry[]): FileEntry[] {
	return tree.map((entry) => {
		if (entry.path === parentPath) {
			return { ...entry, children };
		} else if (entry.children) {
			return { ...entry, children: addChildrenToTree(entry.children, parentPath, children) };
		}
		return entry;
	});
}

// Get the connection ID for the active session
function getActiveConnectionId(): string {
	const session = get(activeSession);
	if (!session) {
		throw new Error('No active session');
	}
	return session.connectionId;
}

// Get the active session ID
function getActiveSessionId(): string {
	const ws = get(workspaceStore);
	if (!ws.activeSessionId) {
		throw new Error('No active session');
	}
	return ws.activeSessionId;
}

// Update the active session's file state
function updateFileState(updater: (state: SessionFileState) => SessionFileState): void {
	const sessionId = getActiveSessionId();
	workspaceStore.updateSessionFileState(sessionId, updater);
}

// Derived store that reflects the active session's file state
const fileStateStore = derived(activeSession, ($session) => {
	if (!$session) return emptyState;
	return {
		projectRoot: $session.projectRoot,
		tree: $session.fileState.tree,
		expandedPaths: $session.fileState.expandedPaths,
		openFiles: $session.fileState.openFiles,
		activeFilePath: $session.fileState.activeFilePath
	} as FileState;
});

// Create the file store with methods
function createFileStore() {
	return {
		// Subscribe to the derived state
		subscribe: fileStateStore.subscribe,

		async refreshDirectory(path: string): Promise<void> {
			const connId = getActiveConnectionId();
			const entries = await invoke<FileEntry[]>('sftp_list_dir', { connId, path });
			const sortedEntries = sortEntries(entries);
			const session = get(activeSession);

			updateFileState((s) => {
				if (path === session?.projectRoot) {
					return { ...s, tree: sortedEntries };
				}
				const newTree = addChildrenToTree(s.tree, path, sortedEntries);
				return { ...s, tree: newTree };
			});
		},

		async expandDirectory(path: string): Promise<void> {
			const connId = getActiveConnectionId();
			const entries = await invoke<FileEntry[]>('sftp_list_dir', { connId, path });
			const sortedEntries = sortEntries(entries);

			updateFileState((s) => {
				const newExpanded = new Set(s.expandedPaths);
				newExpanded.add(path);
				const newTree = addChildrenToTree(s.tree, path, sortedEntries);
				return {
					...s,
					expandedPaths: newExpanded,
					tree: newTree
				};
			});
		},

		collapseDirectory(path: string): void {
			updateFileState((s) => {
				const newExpanded = new Set(s.expandedPaths);
				newExpanded.delete(path);
				return { ...s, expandedPaths: newExpanded };
			});
		},

		toggleDirectory(path: string): void {
			const session = get(activeSession);
			if (!session) return;

			if (session.fileState.expandedPaths.has(path)) {
				this.collapseDirectory(path);
			} else {
				this.expandDirectory(path);
			}
		},

		async openFile(path: string): Promise<void> {
			const session = get(activeSession);
			if (!session) return;

			// Already open, just activate
			if (session.fileState.openFiles.has(path)) {
				updateFileState((s) => ({ ...s, activeFilePath: path }));
				return;
			}

			const connId = getActiveConnectionId();
			const content = await invoke<string>('sftp_read_file', { connId, path });
			const stat = await invoke<{ mtime: number }>('sftp_stat', { connId, path });

			const fileName = path.split('/').pop() || path;
			const language = detectLanguage(fileName);

			const openFile: OpenFile = {
				path,
				content,
				language,
				dirty: false,
				remoteMtime: stat.mtime
			};

			updateFileState((s) => {
				const newOpenFiles = new Map(s.openFiles);
				newOpenFiles.set(path, openFile);
				return { ...s, openFiles: newOpenFiles, activeFilePath: path };
			});
		},

		updateFileContent(path: string, content: string): void {
			updateFileState((s) => {
				const file = s.openFiles.get(path);
				if (!file) return s;

				const newOpenFiles = new Map(s.openFiles);
				newOpenFiles.set(path, { ...file, content, dirty: true });
				return { ...s, openFiles: newOpenFiles };
			});
		},

		async saveFile(path: string): Promise<void> {
			const session = get(activeSession);
			if (!session) return;

			const file = session.fileState.openFiles.get(path);
			if (!file) return;

			const connId = getActiveConnectionId();

			// Check for conflicts
			const remoteStat = await invoke<{ mtime: number }>('sftp_stat', { connId, path });
			if (remoteStat.mtime > file.remoteMtime) {
				throw new Error('CONFLICT');
			}

			const result = await invoke<{ mtime: number }>('sftp_write_file', {
				connId,
				path,
				content: file.content
			});

			updateFileState((s) => {
				const newOpenFiles = new Map(s.openFiles);
				const updatedFile = newOpenFiles.get(path);
				if (updatedFile) {
					newOpenFiles.set(path, {
						...updatedFile,
						dirty: false,
						remoteMtime: result.mtime
					});
				}
				return { ...s, openFiles: newOpenFiles };
			});
		},

		closeFile(path: string): void {
			updateFileState((s) => {
				const newOpenFiles = new Map(s.openFiles);
				newOpenFiles.delete(path);

				let newActivePath = s.activeFilePath;
				if (s.activeFilePath === path) {
					const paths = Array.from(newOpenFiles.keys());
					newActivePath = paths.length > 0 ? paths[paths.length - 1] : null;
				}

				return { ...s, openFiles: newOpenFiles, activeFilePath: newActivePath };
			});
		},

		setActiveFile(path: string | null): void {
			updateFileState((s) => ({ ...s, activeFilePath: path }));
		},

		async createFile(path: string): Promise<void> {
			const connId = getActiveConnectionId();
			await invoke('sftp_create_file', { connId, path });
			await this.refreshDirectory(path.substring(0, path.lastIndexOf('/')));
		},

		async createDirectory(path: string): Promise<void> {
			const connId = getActiveConnectionId();
			await invoke('sftp_create_dir', { connId, path });
			await this.refreshDirectory(path.substring(0, path.lastIndexOf('/')));
		},

		async deleteEntry(path: string): Promise<void> {
			const connId = getActiveConnectionId();
			await invoke('sftp_delete', { connId, path });
			await this.refreshDirectory(path.substring(0, path.lastIndexOf('/')));

			// Close file if open
			updateFileState((s) => {
				if (s.openFiles.has(path)) {
					const newOpenFiles = new Map(s.openFiles);
					newOpenFiles.delete(path);
					return { ...s, openFiles: newOpenFiles };
				}
				return s;
			});
		},

		async renameEntry(oldPath: string, newPath: string): Promise<void> {
			const connId = getActiveConnectionId();
			await invoke('sftp_rename', { connId, oldPath, newPath });
			await this.refreshDirectory(oldPath.substring(0, oldPath.lastIndexOf('/')));
		}
	};
}

export const fileStore = createFileStore();

// Derived stores
export const openFiles = derived(fileStore, ($store) => Array.from($store.openFiles.values()));
export const activeFile = derived(fileStore, ($store) =>
	$store.activeFilePath ? $store.openFiles.get($store.activeFilePath) : null
);
export const hasUnsavedFiles = derived(fileStore, ($store) =>
	Array.from($store.openFiles.values()).some((f) => f.dirty)
);
