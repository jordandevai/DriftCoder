import { derived, get } from 'svelte/store';
import type { FileState, FileEntry, OpenFile, SessionFileState } from '$types';
import { invoke } from '$utils/tauri';
import { workspaceStore, activeSession } from './workspace';
import { detectLanguage } from '$utils/languages';
import { sortEntries } from '$utils/file-tree';

const DEFAULT_REMOTE_CHECK_STALE_MS = 10_000;

// Initial state for when no session is active
const emptyState: FileState = {
	projectRoot: '',
	tree: [],
	expandedPaths: new Set(),
	openFiles: new Map(),
	activeFilePath: null
};

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

function findEntry(tree: FileEntry[], path: string): FileEntry | null {
	for (const entry of tree) {
		if (entry.path === path) return entry;
		if (entry.children) {
			const found = findEntry(entry.children, path);
			if (found) return found;
		}
	}
	return null;
}

function collectDirectoryPaths(tree: FileEntry[], out: Set<string>): void {
	for (const entry of tree) {
		if (entry.isDirectory) {
			out.add(entry.path);
			if (entry.children) collectDirectoryPaths(entry.children, out);
		}
	}
}

function requireActiveSession() {
	const session = get(activeSession);
	if (!session) {
		throw new Error('No active session');
	}
	return session;
}

// Update a specific session's file state (prevents async actions from mutating the wrong session after a tab switch)
function updateFileState(sessionId: string, updater: (state: SessionFileState) => SessionFileState): void {
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
	let remoteSyncCleanup: (() => void) | null = null;

	async function fetchRemoteFileInternal(path: string): Promise<{ content: string; mtime: number }> {
		const session = requireActiveSession();
		const connId = session.connectionId;

		const result = await invoke<{ content: string; mtime: number }>('sftp_read_file_with_stat', {
			connId,
			path
		});

		return { content: result.content, mtime: result.mtime };
	}

	async function reloadFileFromRemoteInternal(path: string): Promise<void> {
		const session = requireActiveSession();
		const sessionId = session.id;

		const remote = await fetchRemoteFileInternal(path);
		updateFileState(sessionId, (s) => {
			const file = s.openFiles.get(path);
			if (!file) return s;

			const newOpenFiles = new Map(s.openFiles);
			newOpenFiles.set(path, {
				...file,
				content: remote.content,
				dirty: false,
				remoteMtime: remote.mtime,
				remoteLastCheckedAt: Date.now(),
				remoteChanged: false,
				remoteMtimeOnServer: undefined
			});
			return { ...s, openFiles: newOpenFiles };
		});
	}

	async function checkRemoteForOpenFile(
		path: string,
		opts?: { onlyIfStaleMs?: number; trigger?: 'focus' | 'activate' | 'manual' }
	): Promise<void> {
		const session = get(activeSession);
		if (!session) return;

		const sessionId = session.id;
		const connId = session.connectionId;

		const current = session.fileState.openFiles.get(path);
		if (!current) return;

		const now = Date.now();
		const onlyIfStaleMs = opts?.onlyIfStaleMs ?? DEFAULT_REMOTE_CHECK_STALE_MS;
		if (onlyIfStaleMs > 0 && current.remoteLastCheckedAt && now - current.remoteLastCheckedAt < onlyIfStaleMs) {
			return;
		}

		let remoteMtime: number;
		try {
			const remoteStat = await invoke<{ mtime: number }>('sftp_stat', { connId, path });
			remoteMtime = remoteStat.mtime;
		} catch {
			// If the connection is down or the stat fails transiently, avoid spamming UI.
			return;
		}

		// Use latest state to avoid racing against user edits or session switches.
		const latestSession = get(activeSession);
		if (!latestSession || latestSession.id !== sessionId) return;
		const latest = latestSession.fileState.openFiles.get(path);
		if (!latest) return;

		updateFileState(sessionId, (s) => {
			const file = s.openFiles.get(path);
			if (!file) return s;

			const updated: OpenFile = {
				...file,
				remoteLastCheckedAt: now,
				remoteMtimeOnServer: remoteMtime,
				// Only surface a "remote changed" indicator when the user has local edits.
				remoteChanged: remoteMtime > file.remoteMtime && file.dirty ? true : false
			};

			// If remote isn't newer, clear any prior indicator.
			if (remoteMtime <= file.remoteMtime) {
				updated.remoteChanged = false;
				updated.remoteMtimeOnServer = undefined;
			}

			const newOpenFiles = new Map(s.openFiles);
			newOpenFiles.set(path, updated);
			return { ...s, openFiles: newOpenFiles };
		});

		// If the file is clean and the remote changed, refresh the local view automatically.
		const afterUpdateSession = get(activeSession);
		if (!afterUpdateSession || afterUpdateSession.id !== sessionId) return;
		const afterUpdateFile = afterUpdateSession.fileState.openFiles.get(path);
		if (!afterUpdateFile) return;
		if (!afterUpdateFile.dirty && remoteMtime > afterUpdateFile.remoteMtime) {
			await reloadFileFromRemoteInternal(path);
		}
	}

	return {
		// Subscribe to the derived state
		subscribe: fileStateStore.subscribe,

		initRemoteSync(): void {
			if (typeof window === 'undefined') return;
			if (remoteSyncCleanup) return;

			const onFocus = () => {
				const session = get(activeSession);
				const path = session?.fileState.activeFilePath;
				if (!path) return;
				void checkRemoteForOpenFile(path, { trigger: 'focus' });
			};

			const onVisibility = () => {
				if (document.hidden) return;
				onFocus();
			};

			window.addEventListener('focus', onFocus);
			document.addEventListener('visibilitychange', onVisibility);

			remoteSyncCleanup = () => {
				window.removeEventListener('focus', onFocus);
				document.removeEventListener('visibilitychange', onVisibility);
				remoteSyncCleanup = null;
			};
		},

		destroyRemoteSync(): void {
			remoteSyncCleanup?.();
		},

		async checkRemoteNow(path: string): Promise<void> {
			await checkRemoteForOpenFile(path, { onlyIfStaleMs: 0, trigger: 'manual' });
		},

		async fetchRemoteFile(path: string): Promise<{ content: string; mtime: number }> {
			return await fetchRemoteFileInternal(path);
		},

		async refreshDirectory(path: string): Promise<void> {
			const session = requireActiveSession();
			const sessionId = session.id;
			const connId = session.connectionId;
			const projectRoot = session.projectRoot;

			const entries = await invoke<FileEntry[]>('sftp_list_dir', { connId, path });
			const sortedEntries = sortEntries(entries);

			updateFileState(sessionId, (s) => {
				if (path === projectRoot) {
					return { ...s, tree: sortedEntries };
				}
				const newTree = addChildrenToTree(s.tree, path, sortedEntries);
				return { ...s, tree: newTree };
			});
		},

		async expandDirectory(path: string): Promise<void> {
			const session = requireActiveSession();
			const sessionId = session.id;
			const connId = session.connectionId;

			// If we've already loaded children for this directory, expanding should not refetch.
			const existing = findEntry(session.fileState.tree, path);
			if (existing?.children) {
				updateFileState(sessionId, (s) => {
					const newExpanded = new Set(s.expandedPaths);
					newExpanded.add(path);
					return { ...s, expandedPaths: newExpanded };
				});
				return;
			}

			const entries = await invoke<FileEntry[]>('sftp_list_dir', { connId, path });
			const sortedEntries = sortEntries(entries);

			updateFileState(sessionId, (s) => {
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
			const session = get(activeSession);
			if (!session) return;

			updateFileState(session.id, (s) => {
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

		collapseAll(): void {
			const session = get(activeSession);
			if (!session) return;
			updateFileState(session.id, (s) => ({ ...s, expandedPaths: new Set([session.projectRoot]) }));
		},

		expandAllLoaded(): void {
			const session = get(activeSession);
			if (!session) return;
			updateFileState(session.id, (s) => {
				const expanded = new Set(s.expandedPaths);
				expanded.add(session.projectRoot);
				collectDirectoryPaths(s.tree, expanded);
				return { ...s, expandedPaths: expanded };
			});
		},

		async openFile(path: string): Promise<void> {
			const session = requireActiveSession();
			const sessionId = session.id;
			const connId = session.connectionId;

			// Already open, just activate
			if (session.fileState.openFiles.has(path)) {
				updateFileState(sessionId, (s) => ({ ...s, activeFilePath: path }));
				return;
			}

			const result = await invoke<{ content: string; mtime: number }>('sftp_read_file_with_stat', {
				connId,
				path
			});

			const fileName = path.split('/').pop() || path;
			const language = detectLanguage(fileName);

			const openFile: OpenFile = {
				path,
				content: result.content,
				language,
				dirty: false,
				remoteMtime: result.mtime
			};

			updateFileState(sessionId, (s) => {
				const newOpenFiles = new Map(s.openFiles);
				newOpenFiles.set(path, openFile);
				return { ...s, openFiles: newOpenFiles, activeFilePath: path };
			});
		},

		updateFileContent(path: string, content: string): void {
			const session = get(activeSession);
			if (!session) return;

			updateFileState(session.id, (s) => {
				const file = s.openFiles.get(path);
				if (!file) return s;

				const newOpenFiles = new Map(s.openFiles);
				newOpenFiles.set(path, { ...file, content, dirty: true });
				return { ...s, openFiles: newOpenFiles };
			});
		},

		setRemoteMtime(path: string, remoteMtime: number): void {
			const session = get(activeSession);
			if (!session) return;

			updateFileState(session.id, (s) => {
				const file = s.openFiles.get(path);
				if (!file) return s;

				const newOpenFiles = new Map(s.openFiles);
				newOpenFiles.set(path, { ...file, remoteMtime });
				return { ...s, openFiles: newOpenFiles };
			});
		},

		async reloadFileFromRemote(path: string): Promise<void> {
			await reloadFileFromRemoteInternal(path);
		},

		async saveFile(path: string): Promise<void> {
			const session = requireActiveSession();
			const sessionId = session.id;
			const connId = session.connectionId;

			const file = session.fileState.openFiles.get(path);
			if (!file) return;

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

			updateFileState(sessionId, (s) => {
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

		async forceSaveFile(path: string, contentOverride?: string): Promise<void> {
			const session = requireActiveSession();
			const sessionId = session.id;
			const connId = session.connectionId;

			const sessionState = get(activeSession);
			const file = sessionState?.fileState.openFiles.get(path);
			if (!file && contentOverride === undefined) return;

			const content = contentOverride ?? file!.content;

			const result = await invoke<{ mtime: number }>('sftp_write_file', {
				connId,
				path,
				content
			});

			updateFileState(sessionId, (s) => {
				const existing = s.openFiles.get(path);
				if (!existing) return s;

				const newOpenFiles = new Map(s.openFiles);
				newOpenFiles.set(path, {
					...existing,
					content,
					dirty: false,
					remoteMtime: result.mtime
				});
				return { ...s, openFiles: newOpenFiles };
			});
		},

		closeFile(path: string): void {
			const session = get(activeSession);
			if (!session) return;

			updateFileState(session.id, (s) => {
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
			const session = get(activeSession);
			if (!session) return;

			updateFileState(session.id, (s) => ({ ...s, activeFilePath: path }));
			if (path) {
				void checkRemoteForOpenFile(path, { trigger: 'activate' });
			}
		},

		async createFile(path: string): Promise<void> {
			const session = requireActiveSession();
			const connId = session.connectionId;

			await invoke('sftp_create_file', { connId, path });
			await this.refreshDirectory(path.substring(0, path.lastIndexOf('/')));
		},

		async createDirectory(path: string): Promise<void> {
			const session = requireActiveSession();
			const connId = session.connectionId;

			await invoke('sftp_create_dir', { connId, path });
			await this.refreshDirectory(path.substring(0, path.lastIndexOf('/')));
		},

		async deleteEntry(path: string): Promise<void> {
			const session = requireActiveSession();
			const sessionId = session.id;
			const connId = session.connectionId;

			await invoke('sftp_delete', { connId, path });
			await this.refreshDirectory(path.substring(0, path.lastIndexOf('/')));

			// Close file if open
			updateFileState(sessionId, (s) => {
				if (s.openFiles.has(path)) {
					const newOpenFiles = new Map(s.openFiles);
					newOpenFiles.delete(path);
					return { ...s, openFiles: newOpenFiles };
				}
				return s;
			});
		},

		async renameEntry(oldPath: string, newPath: string): Promise<void> {
			const session = requireActiveSession();
			const connId = session.connectionId;

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
