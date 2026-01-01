import { writable, derived, get } from 'svelte/store';
import type { ConnectionState, ConnectionProfile, ActiveConnection } from '$types';
import { invoke } from '$utils/tauri';
import { loadSavedConnections, saveConnections } from '$utils/storage';
import { notificationsStore } from './notifications';

const initialState: ConnectionState = {
	status: 'idle',
	activeConnections: new Map(),
	savedProfiles: [],
	error: null
};

function createConnectionStore() {
	const { subscribe, set, update } = writable<ConnectionState>(initialState);

	return {
		subscribe,

		/**
		 * Connect to a server. Returns the connection ID.
		 * Does not set any "active" connection - that's managed by workspaceStore sessions.
		 */
		async connect(profile: ConnectionProfile, password?: string): Promise<string> {
			update((s) => ({ ...s, status: 'connecting', error: null }));

			try {
				const connectionId = await invoke<string>('ssh_connect', {
					profile,
					password
				});

				const activeConn: ActiveConnection = {
					id: connectionId,
					profile,
					sessionCount: 0 // Will be incremented by workspaceStore when creating session
				};

				update((s) => {
					const newConnections = new Map(s.activeConnections);
					newConnections.set(connectionId, activeConn);
					return {
						...s,
						status: 'idle',
						activeConnections: newConnections,
						error: null
					};
				});

				return connectionId;
			} catch (error) {
				update((s) => ({
					...s,
					status: 'idle',
					error: error instanceof Error ? error.message : String(error)
				}));
				throw error;
			}
		},

		/**
		 * Disconnect a specific connection by ID
		 */
		async disconnectById(connectionId: string): Promise<void> {
			try {
				await invoke('ssh_disconnect', { connId: connectionId });
			} catch (error) {
				console.error('Failed to disconnect:', error);
				const profile = get({ subscribe }).activeConnections.get(connectionId)?.profile;
				notificationsStore.notify({
					severity: 'warning',
					title: 'Disconnect Failed',
					message: profile
						? `Could not disconnect from ${profile.username}@${profile.host}:${profile.port}.`
						: 'Could not disconnect.',
					detail: error instanceof Error ? error.message : String(error)
				});
			}

			update((s) => {
				const newConnections = new Map(s.activeConnections);
				newConnections.delete(connectionId);
				return { ...s, activeConnections: newConnections };
			});
		},

		/**
		 * Disconnect all active connections
		 */
		async disconnectAll(): Promise<void> {
			const state = get({ subscribe });
			for (const connectionId of state.activeConnections.keys()) {
				await this.disconnectById(connectionId);
			}
		},

		/**
		 * Test a connection without persisting
		 */
		async testConnection(profile: ConnectionProfile, password?: string): Promise<boolean> {
			try {
				return await invoke<boolean>('ssh_test_connection', { profile, password });
			} catch {
				return false;
			}
		},

		/**
		 * Get an active connection by ID
		 */
		getConnection(connectionId: string): ActiveConnection | undefined {
			return get({ subscribe }).activeConnections.get(connectionId);
		},

		/**
		 * Check if a connection is active
		 */
		isConnectionActive(connectionId: string): boolean {
			return get({ subscribe }).activeConnections.has(connectionId);
		},

		/**
		 * Update session count for a connection
		 */
		updateSessionCount(connectionId: string, delta: number): void {
			update((s) => {
				const conn = s.activeConnections.get(connectionId);
				if (!conn) return s;

				const newConnections = new Map(s.activeConnections);
				newConnections.set(connectionId, {
					...conn,
					sessionCount: Math.max(0, conn.sessionCount + delta)
				});
				return { ...s, activeConnections: newConnections };
			});
		},

		setError(error: string | null): void {
			update((s) => ({ ...s, error }));
		},

		addProfile(profile: ConnectionProfile): void {
			update((s) => {
				const newProfiles = [...s.savedProfiles.filter((p) => p.id !== profile.id), profile];
				saveConnections(newProfiles);
				return { ...s, savedProfiles: newProfiles };
			});
		},

		removeProfile(profileId: string): void {
			update((s) => {
				const newProfiles = s.savedProfiles.filter((p) => p.id !== profileId);
				saveConnections(newProfiles);
				return { ...s, savedProfiles: newProfiles };
			});
		},

		/**
		 * Add a project path to recent projects for a profile
		 */
		addRecentProject(profileId: string, path: string): void {
			update((s) => {
				const newProfiles = s.savedProfiles.map((p) => {
					if (p.id !== profileId) return p;
					const recent = [path, ...p.recentProjects.filter((r) => r !== path)].slice(0, 5);
					return { ...p, recentProjects: recent };
				});
				saveConnections(newProfiles);
				return { ...s, savedProfiles: newProfiles };
			});
		},

		/**
		 * Toggle bookmark for a path on a profile
		 */
		toggleBookmark(profileId: string, path: string): void {
			update((s) => {
				const newProfiles = s.savedProfiles.map((p) => {
					if (p.id !== profileId) return p;
					const hasBookmark = p.bookmarkedPaths.includes(path);
					const bookmarks = hasBookmark
						? p.bookmarkedPaths.filter((b) => b !== path)
						: [...p.bookmarkedPaths, path];
					return { ...p, bookmarkedPaths: bookmarks };
				});
				saveConnections(newProfiles);
				return { ...s, savedProfiles: newProfiles };
			});
		},

		/**
		 * Check if path is bookmarked for a profile
		 */
		isBookmarked(profileId: string, path: string): boolean {
			const state = get({ subscribe });
			const profile = state.savedProfiles.find((p) => p.id === profileId);
			return profile?.bookmarkedPaths.includes(path) || false;
		},

		/**
		 * Get a profile by ID
		 */
		getProfile(profileId: string): ConnectionProfile | undefined {
			return get({ subscribe }).savedProfiles.find((p) => p.id === profileId);
		},

		async init(): Promise<void> {
			const profiles = await loadSavedConnections();
			// Migrate old profiles that don't have new fields
			const migratedProfiles = profiles.map((p) => ({
				...p,
				recentProjects: p.recentProjects || [],
				bookmarkedPaths: p.bookmarkedPaths || []
			}));
			update((s) => ({ ...s, savedProfiles: migratedProfiles }));
		},

		reset(): void {
			set(initialState);
		}
	};
}

export const connectionStore = createConnectionStore();

// Derived stores for convenience
export const hasActiveConnections = derived(
	connectionStore,
	($store) => $store.activeConnections.size > 0
);

export const activeConnectionsList = derived(connectionStore, ($store) =>
	Array.from($store.activeConnections.values())
);

export const connectionError = derived(connectionStore, ($store) => $store.error);

export const isConnecting = derived(connectionStore, ($store) => $store.status === 'connecting');
