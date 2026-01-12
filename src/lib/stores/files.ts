import { derived, get } from 'svelte/store';
import type { FileState, FileEntry, OpenFile, SessionFileState } from '$types';
import { invoke } from '$utils/tauri';
import { workspaceStore, activeSession } from './workspace';
import { detectLanguage } from '$utils/languages';
import { sortEntries } from '$utils/file-tree';

const DEFAULT_REMOTE_CHECK_STALE_MS = 5_000;
const REMOTE_POLL_TICK_MS = 2_000;

type FileMeta = { path: string; size: number; mtime: number };

function rewritePath(path: string, oldBase: string, newBase: string): string | null {
	if (path === oldBase) return newBase;
	const prefix = oldBase.endsWith('/') ? oldBase : `${oldBase}/`;
	if (path.startsWith(prefix)) return newBase + path.slice(oldBase.length);
	return null;
}

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
	let remotePollTimer: number | null = null;
	const lastTreeEnsureAtBySessionId = new Map<string, number>();
	const TREE_ENSURE_THROTTLE_MS = 15_000;

	async function refreshDirectoryForSession(
		sessionId: string,
		connId: string,
		projectRoot: string,
		path: string
	): Promise<void> {
		const entries = await invoke<FileEntry[]>('sftp_list_dir', { connId, path });
		const sortedEntries = sortEntries(entries);

		updateFileState(sessionId, (s) => {
			if (path === projectRoot) {
				return { ...s, tree: sortedEntries };
			}
			const newTree = addChildrenToTree(s.tree, path, sortedEntries);
			return { ...s, tree: newTree };
		});
	}

	async function ensureProjectTreeLoadedForActiveSession(): Promise<void> {
		const session = get(activeSession);
		if (!session) return;
		if (session.connectionStatus === 'disconnected') return;
		if (!session.projectRoot) return;
		if (session.fileState.tree.length > 0) return;

		const last = lastTreeEnsureAtBySessionId.get(session.id) ?? 0;
		const now = Date.now();
		if (now - last < TREE_ENSURE_THROTTLE_MS) return;
		lastTreeEnsureAtBySessionId.set(session.id, now);

		try {
			console.debug('[fileStore] auto-loading file tree', {
				sessionId: session.id,
				projectRoot: session.projectRoot
			});
			await refreshDirectoryForSession(session.id, session.connectionId, session.projectRoot, session.projectRoot);
		} catch (error) {
			console.warn('Failed to auto-load project file tree:', error);
		}
	}

	async function fetchRemoteFileInternal(path: string): Promise<{ content: string; mtime: number; size: number }> {
		const session = requireActiveSession();
		const connId = session.connectionId;

		const result = await invoke<{ content: string; mtime: number; size: number }>('sftp_read_file_with_stat', {
			connId,
			path
		});

		return { content: result.content, mtime: result.mtime, size: result.size };
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
				remoteSize: remote.size,
				remoteLastCheckedAt: Date.now(),
				remoteChanged: false,
				remoteUpdateAvailable: false,
				remoteMtimeOnServer: undefined,
				remoteSizeOnServer: undefined
			});
			return { ...s, openFiles: newOpenFiles };
		});
	}

	async function checkRemoteForOpenFile(
		path: string,
		opts?: { onlyIfStaleMs?: number; trigger?: 'focus' | 'activate' | 'poll' | 'manual' }
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

		let meta: FileMeta;
		try {
			meta = await invoke<FileMeta>('sftp_stat', { connId, path });
		} catch (error) {
			const msg = error instanceof Error ? error.message.toLowerCase() : String(error).toLowerCase();
			const missing =
				msg.includes('no such file') ||
				msg.includes('not found') ||
				msg.includes('does not exist') ||
				msg.includes('status code 2');
			if (missing) {
				updateFileState(sessionId, (s) => {
					const file = s.openFiles.get(path);
					if (!file) return s;
					const newOpenFiles = new Map(s.openFiles);
					newOpenFiles.set(path, {
						...file,
						remoteLastCheckedAt: now,
						remoteMissing: true
					});
					return { ...s, openFiles: newOpenFiles };
				});
			}
			return;
		}

		const latestSession = get(activeSession);
		if (!latestSession || latestSession.id !== sessionId) return;
		const latest = latestSession.fileState.openFiles.get(path);
		if (!latest) return;

		updateFileState(sessionId, (s) => {
			const file = s.openFiles.get(path);
			if (!file) return s;

			const remoteNewer =
				meta.mtime > file.remoteMtime ||
				(file.remoteSize !== undefined && meta.size !== file.remoteSize);

			const updated: OpenFile = {
				...file,
				remoteLastCheckedAt: now,
				remoteMissing: false,
				remoteMtimeOnServer: meta.mtime,
				remoteSizeOnServer: meta.size,
				remoteChanged: remoteNewer && file.dirty ? true : false,
				remoteUpdateAvailable: remoteNewer && !file.dirty ? true : false
			};

			if (!remoteNewer) {
				updated.remoteChanged = false;
				updated.remoteMtimeOnServer = undefined;
				updated.remoteSizeOnServer = undefined;
				updated.remoteUpdateAvailable = false;
			}

			const newOpenFiles = new Map(s.openFiles);
			newOpenFiles.set(path, updated);
			return { ...s, openFiles: newOpenFiles };
		});

		const afterUpdateSession = get(activeSession);
		if (!afterUpdateSession || afterUpdateSession.id !== sessionId) return;
		const afterUpdateFile = afterUpdateSession.fileState.openFiles.get(path);
		if (!afterUpdateFile) return;
		const remoteNewer =
			meta.mtime > afterUpdateFile.remoteMtime ||
			(afterUpdateFile.remoteSize !== undefined && meta.size !== afterUpdateFile.remoteSize);
		if (!afterUpdateFile.dirty && remoteNewer) {
			await reloadFileFromRemoteInternal(path);
		}
	}

	return {
		subscribe: fileStateStore.subscribe,

		initRemoteSync(): void {
			if (typeof window === 'undefined') return;
			if (remoteSyncCleanup) return;

			const onFocus = () => {
				const session = get(activeSession);
				if (!session || session.connectionStatus === 'disconnected') return;
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

			remotePollTimer = window.setInterval(() => {
				if (document.hidden) return;
				const session = get(activeSession);
				if (!session || session.connectionStatus === 'disconnected') return;
				const path = session?.fileState.activeFilePath;
				if (!path) return;
				void checkRemoteForOpenFile(path, { trigger: 'poll' });
			}, REMOTE_POLL_TICK_MS);

			remoteSyncCleanup = () => {
				window.removeEventListener('focus', onFocus);
				document.removeEventListener('visibilitychange', onVisibility);
				if (remotePollTimer !== null) {
					window.clearInterval(remotePollTimer);
					remotePollTimer = null;
				}
				remoteSyncCleanup = null;
			};
		},

		destroyRemoteSync(): void {
			remoteSyncCleanup?.();
		},

		async checkRemoteNow(path: string): Promise<void> {
			await checkRemoteForOpenFile(path, { onlyIfStaleMs: 0, trigger: 'manual' });
		},

		async fetchRemoteFile(path: string): Promise<{ content: string; mtime: number; size: number }> {
			return await fetchRemoteFileInternal(path);
		},

		async refreshDirectory(path: string): Promise<void> {
			const session = requireActiveSession();
			const sessionId = session.id;
			const connId = session.connectionId;
			const projectRoot = session.projectRoot;
			await refreshDirectoryForSession(sessionId, connId, projectRoot, path);
		},

		async ensureProjectTreeLoaded(): Promise<void> {
			await ensureProjectTreeLoadedForActiveSession();
		},

		async expandDirectory(path: string): Promise<void> {
			const session = requireActiveSession();
			const sessionId = session.id;
			const connId = session.connectionId;

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

		async expandAll(maxDepth = 5): Promise<void> {
			const session = requireActiveSession();
			const sessionId = session.id;
			const connId = session.connectionId;

			const loadDir = async (path: string, depth: number): Promise<void> => {
				if (depth > maxDepth) return;

				const currentSession = get(activeSession);
				if (!currentSession || currentSession.id !== sessionId) return;

				const existing = findEntry(currentSession.fileState.tree, path);
				let children = existing?.children;

				if (!children) {
					const entries = await invoke<FileEntry[]>('sftp_list_dir', { connId, path });
					children = sortEntries(entries);
					updateFileState(sessionId, (s) => {
						const newExpanded = new Set(s.expandedPaths);
						newExpanded.add(path);
						const newTree = addChildrenToTree(s.tree, path, children!);
						return { ...s, expandedPaths: newExpanded, tree: newTree };
					});
				} else {
					updateFileState(sessionId, (s) => {
						const newExpanded = new Set(s.expandedPaths);
						newExpanded.add(path);
						return { ...s, expandedPaths: newExpanded };
					});
				}

				const subDirs = children.filter((e) => e.isDirectory);
				await Promise.all(subDirs.map((d) => loadDir(d.path, depth + 1)));
			};

			await loadDir(session.projectRoot, 0);
		},

		async openFile(path: string): Promise<void> {
			const session = requireActiveSession();
			const sessionId = session.id;
			const connId = session.connectionId;

			if (session.fileState.openFiles.has(path)) {
				this.setActiveFile(path);
				return;
			}

			const result = await invoke<{ content: string; mtime: number; size: number }>('sftp_read_file_with_stat', {
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
				remoteMtime: result.mtime,
				remoteSize: result.size,
				remoteLastCheckedAt: Date.now(),
				remoteChanged: false,
				remoteUpdateAvailable: false
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

			let remoteStat: FileMeta;
			try {
				remoteStat = await invoke<FileMeta>('sftp_stat', { connId, path });
			} catch (error) {
				const msg = error instanceof Error ? error.message.toLowerCase() : String(error).toLowerCase();
				const missing =
					msg.includes('no such file') ||
					msg.includes('not found') ||
					msg.includes('does not exist') ||
					msg.includes('status code 2');
				if (missing) throw new Error('MISSING');
				throw error;
			}
			const remoteNewer =
				remoteStat.mtime > file.remoteMtime ||
				(file.remoteSize !== undefined && remoteStat.size !== file.remoteSize);
			if (remoteNewer) {
				throw new Error('CONFLICT');
			}

			const result = await invoke<FileMeta>('sftp_write_file', {
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
						remoteMtime: result.mtime,
						remoteSize: result.size,
						remoteLastCheckedAt: Date.now(),
						remoteChanged: false,
						remoteUpdateAvailable: false,
						remoteMtimeOnServer: undefined,
						remoteSizeOnServer: undefined
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

			const result = await invoke<FileMeta>('sftp_write_file', {
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
					remoteMtime: result.mtime,
					remoteSize: result.size,
					remoteLastCheckedAt: Date.now(),
					remoteChanged: false,
					remoteUpdateAvailable: false,
					remoteMtimeOnServer: undefined,
					remoteSizeOnServer: undefined
				});
				return { ...s, openFiles: newOpenFiles };
			});
		},

		async saveFileAs(oldPath: string, newPath: string): Promise<void> {
			const session = requireActiveSession();
			const sessionId = session.id;
			const connId = session.connectionId;

			const existing = session.fileState.openFiles.get(oldPath);
			if (!existing) return;

			const result = await invoke<FileMeta>('sftp_write_file', {
				connId,
				path: newPath,
				content: existing.content
			});

			updateFileState(sessionId, (s) => {
				const file = s.openFiles.get(oldPath);
				if (!file) return s;

				const newOpenFiles = new Map(s.openFiles);
				newOpenFiles.delete(oldPath);
				newOpenFiles.set(newPath, {
					...file,
					path: newPath,
					dirty: false,
					remoteMtime: result.mtime,
					remoteSize: result.size,
					remoteLastCheckedAt: Date.now(),
					remoteChanged: false,
					remoteUpdateAvailable: false,
					remoteMissing: false,
					remoteMtimeOnServer: undefined,
					remoteSizeOnServer: undefined
				});

				const activeFilePath = s.activeFilePath === oldPath ? newPath : s.activeFilePath;
				return { ...s, openFiles: newOpenFiles, activeFilePath };
			});

			workspaceStore.updateSessionLayoutState(sessionId, (layout) => {
				const newGroups = new Map(layout.groups);
				for (const [groupId, group] of newGroups) {
					const panels = group.panels.map((p) => {
						if (p.type !== 'editor' || !p.filePath) return p;
						if (p.filePath !== oldPath) return p;
						const title = newPath.split('/').pop() || newPath;
						return { ...p, filePath: newPath, title };
					});
					newGroups.set(groupId, { ...group, panels });
				}
				return { ...layout, groups: newGroups };
			});

			const parent = newPath.substring(0, newPath.lastIndexOf('/'));
			if (parent) {
				await this.refreshDirectory(parent);
			}
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

			// Check if already set to avoid infinite reactivity loops in effects
			if (session.fileState.activeFilePath === path) return;

			updateFileState(session.id, (s) => ({ ...s, activeFilePath: path }));
			if (path) {
				void checkRemoteForOpenFile(path, { trigger: 'activate' });
			}
		},

		setScrollPosition(path: string, scrollTop: number): void {
			const session = get(activeSession);
			if (!session) return;

			updateFileState(session.id, (s) => {
				const scrollPositions = new Map(s.scrollPositions ?? []);
				scrollPositions.set(path, scrollTop);
				return { ...s, scrollPositions };
			});
		},

		getScrollPosition(path: string): number {
			const session = get(activeSession);
			if (!session) return 0;
			return session.fileState.scrollPositions?.get(path) ?? 0;
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
			const sessionId = session.id;
			const connId = session.connectionId;

			await invoke('sftp_rename', { connId, oldPath, newPath });

			updateFileState(sessionId, (s) => {
				const newOpenFiles = new Map<string, OpenFile>();
				for (const [path, file] of s.openFiles) {
					const rewritten = rewritePath(path, oldPath, newPath);
					if (!rewritten) {
						newOpenFiles.set(path, file);
						continue;
					}
					newOpenFiles.set(rewritten, { ...file, path: rewritten });
				}

				const nextExpanded = new Set<string>();
				for (const p of s.expandedPaths) {
					const rewritten = rewritePath(p, oldPath, newPath);
					nextExpanded.add(rewritten ?? p);
				}

				const activeFilePath = s.activeFilePath ? rewritePath(s.activeFilePath, oldPath, newPath) ?? s.activeFilePath : null;
				return { ...s, openFiles: newOpenFiles, expandedPaths: nextExpanded, activeFilePath };
			});

			workspaceStore.updateSessionLayoutState(sessionId, (layout) => {
				const newGroups = new Map(layout.groups);
				for (const [groupId, group] of newGroups) {
					const panels = group.panels.map((p) => {
						if (p.type !== 'editor' || !p.filePath) return p;
						const rewritten = rewritePath(p.filePath, oldPath, newPath);
						if (!rewritten) return p;
						const title = rewritten.split('/').pop() || rewritten;
						return { ...p, filePath: rewritten, title };
					});
					newGroups.set(groupId, { ...group, panels });
				}
				return { ...layout, groups: newGroups };
			});

			const oldParent = oldPath.substring(0, oldPath.lastIndexOf('/'));
			const newParent = newPath.substring(0, newPath.lastIndexOf('/'));
			if (oldParent) await this.refreshDirectory(oldParent);
			if (newParent && newParent !== oldParent) await this.refreshDirectory(newParent);
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