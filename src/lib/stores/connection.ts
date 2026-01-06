import { writable, derived, get } from 'svelte/store';
import type { ConnectionState, ConnectionProfile, ActiveConnection } from '$types';
import { invoke, isTauri, listen, TauriCommandError } from '$utils/tauri';
import { loadSavedConnections, saveConnections } from '$utils/storage';
import { notificationsStore } from './notifications';
import { confirmStore } from './confirm';
import { parseHostKeyContext } from '$utils/ssh-hostkey';
import { promptStore } from './prompt';

const initialState: ConnectionState = {
	status: 'idle',
	activeConnections: new Map(),
	savedProfiles: [],
	error: null
};

function createConnectionStore() {
	const { subscribe, set, update } = writable<ConnectionState>(initialState);
	let unlistenConnectionStatus: (() => void) | null = null;
	const connectionSecrets = new Map<string, { password?: string }>();
	const disconnectingIds = new Set<string>();
	const autoReconnectControllers = new Map<string, { cancelled: boolean }>();

	const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

	const shouldAutoReconnect = (connectionId: string): boolean => {
		const state = get({ subscribe });
		const active = state.activeConnections.get(connectionId);
		if (!active) return false;
		// Only auto-reconnect when there are active workspaces using this connection.
		if ((active.sessionCount ?? 0) <= 0) return false;
		if (disconnectingIds.has(connectionId)) return false;
		return true;
	};

	const startAutoReconnect = (connectionId: string) => {
		if (!shouldAutoReconnect(connectionId)) return;
		if (autoReconnectControllers.has(connectionId)) return;

		const controller = { cancelled: false };
		autoReconnectControllers.set(connectionId, controller);

		const run = async () => {
			let attempt = 0;
			while (!controller.cancelled) {
				const state = get({ subscribe });
				const active = state.activeConnections.get(connectionId);
				if (!active) break;
				if ((active.sessionCount ?? 0) <= 0) break;
				if ((active.status ?? 'connected') === 'connected') break;

				const secret = connectionSecrets.get(connectionId);
				if (active.profile.authMethod === 'password' && !secret?.password) {
					// Can't silently auto-reconnect without a password; user can hit Reconnect to prompt.
					break;
				}

				attempt += 1;
				try {
					await connectionStore.reconnect(connectionId, secret?.password);
					break;
				} catch {
					// Keep retrying with backoff until user closes the workspace or reconnect succeeds.
				}

				const backoffMs = Math.min(30_000, 300 * 2 ** Math.min(8, attempt - 1));
				await delay(backoffMs);
			}
		};

		void run().finally(() => {
			autoReconnectControllers.delete(connectionId);
		});
	};

	return {
		subscribe,

		async getTrustedHostKeyFingerprint(
			profile: Pick<ConnectionProfile, 'host' | 'port'>
		): Promise<string | null> {
			try {
				const entry = await invoke<{ fingerprintSha256: string } | null>('ssh_get_trusted_host_key', {
					host: profile.host,
					port: profile.port
				});
				return entry?.fingerprintSha256 ? entry.fingerprintSha256 : null;
			} catch {
				return null;
			}
		},

		async applyHostKeyFingerprint(profile: ConnectionProfile): Promise<ConnectionProfile> {
			const fingerprintSha256 = await this.getTrustedHostKeyFingerprint(profile);
			if (!fingerprintSha256) return profile;
			if (profile.hostKeyFingerprintSha256 === fingerprintSha256) return profile;

			const updatedProfile: ConnectionProfile = { ...profile, hostKeyFingerprintSha256: fingerprintSha256 };

			update((s) => {
				let changed = false;
				const nextActive = new Map(s.activeConnections);

				for (const [id, conn] of nextActive.entries()) {
					if (conn.profile.id !== profile.id) continue;
					nextActive.set(id, {
						...conn,
						profile: { ...conn.profile, hostKeyFingerprintSha256: fingerprintSha256 }
					});
					changed = true;
				}

				const hasSavedProfile = s.savedProfiles.some((p) => p.id === profile.id);
				const nextSaved = hasSavedProfile
					? s.savedProfiles.map((p) =>
							p.id === profile.id ? { ...p, hostKeyFingerprintSha256: fingerprintSha256 } : p
						)
					: s.savedProfiles;

				if (hasSavedProfile) {
					saveConnections(nextSaved);
					changed = true;
				}

				return changed ? { ...s, activeConnections: nextActive, savedProfiles: nextSaved } : s;
			});

			return updatedProfile;
		},

		/**
		 * Register a connection ID as known to the UI without connecting.
		 * Used when restoring a workspace after the app was backgrounded/killed.
		 */
		registerDisconnected(connectionId: string, profile: ConnectionProfile, sessionCount = 0): void {
			update((s) => {
				if (s.activeConnections.has(connectionId)) return s;
				const next = new Map(s.activeConnections);
				next.set(connectionId, {
					id: connectionId,
					profile,
					sessionCount,
					status: 'disconnected',
					lastDisconnectDetail: null
				});
				return { ...s, activeConnections: next };
			});
		},

		async reconnect(connectionId: string, password?: string): Promise<void> {
			const state = get({ subscribe });
			const active = state.activeConnections.get(connectionId);
			if (!active) {
				throw new Error('Connection not found');
			}

			update((s) => {
				const next = new Map(s.activeConnections);
				const conn = next.get(connectionId);
				if (conn) next.set(connectionId, { ...conn, status: 'reconnecting' });
				return { ...s, status: 'connecting', activeConnections: next, error: null };
			});

			const reconnectOnce = async (overridePassword?: string): Promise<void> => {
				const usedPassword = overridePassword ?? password;
				await invoke<void>('ssh_reconnect', {
					connId: connectionId,
					profile: active.profile,
					password: usedPassword
				});
				if (active.profile.authMethod === 'password' && usedPassword) {
					connectionSecrets.set(connectionId, { password: usedPassword });
				}
			};

			try {
				try {
					await reconnectOnce();
					await this.applyHostKeyFingerprint(active.profile);
				} catch (error) {
					if (error instanceof TauriCommandError && error.code === 'ssh_hostkey_untrusted') {
						const ctx = parseHostKeyContext(error.context);
						if (ctx && 'fingerprintSha256' in ctx) {
							const confirmed = await confirmStore.confirm({
								title: 'Trust Host Key?',
								message:
									`The SSH server ${ctx.host}:${ctx.port} is presenting an untrusted host key.\n\n` +
									`Only trust this key if you are sure you’re connecting to the right machine.`,
								detail: `${ctx.keyType} ${ctx.fingerprintSha256}\n\n${ctx.publicKeyOpenssh}`,
								confirmText: 'Trust & Reconnect',
								cancelText: 'Cancel'
							});
							if (confirmed) {
								await invoke('ssh_trust_host_key', {
									request: {
										host: ctx.host,
										port: ctx.port,
										keyType: ctx.keyType,
										fingerprintSha256: ctx.fingerprintSha256,
										publicKeyOpenssh: ctx.publicKeyOpenssh
									}
								});
								await reconnectOnce();
								await this.applyHostKeyFingerprint(active.profile);
							} else {
								throw error;
							}
						} else {
							throw error;
						}
					} else if (error instanceof TauriCommandError && error.code === 'ssh_hostkey_mismatch') {
						const ctx = parseHostKeyContext(error.context);
						if (ctx && 'expectedFingerprintSha256' in ctx) {
							const confirmed = await confirmStore.confirm({
								title: 'Host Key Changed',
								message:
									`WARNING: The host key for ${ctx.host}:${ctx.port} has changed.\n\n` +
									`This can indicate a man-in-the-middle attack, or that the server was reinstalled.\n\n` +
									`Replace the saved key only if you’re sure this is expected.`,
								detail:
									`Expected: ${ctx.expectedFingerprintSha256}\n` +
									`Actual:   ${ctx.actualFingerprintSha256}\n\n` +
									`New key:\n${ctx.actualPublicKeyOpenssh}`,
								confirmText: 'Replace Key & Reconnect',
								cancelText: 'Cancel',
								destructive: true
							});
							if (confirmed) {
								await invoke('ssh_forget_host_key', { host: ctx.host, port: ctx.port });
								await invoke('ssh_trust_host_key', {
									request: {
										host: ctx.host,
										port: ctx.port,
										keyType: ctx.keyType,
										fingerprintSha256: ctx.actualFingerprintSha256,
										publicKeyOpenssh: ctx.actualPublicKeyOpenssh
									}
								});
								await reconnectOnce();
								await this.applyHostKeyFingerprint(active.profile);
							} else {
								throw error;
							}
						} else {
							throw error;
						}
					} else if (
						error instanceof TauriCommandError &&
						(error.code === 'missing_password' || error.code === 'ssh_auth_failed')
						) {
							if (active.profile.authMethod === 'password') {
								const entered = await promptStore.prompt({
									title: 'Reconnect',
									message: `Enter password for ${active.profile.username}@${active.profile.host}:${active.profile.port}`,
									placeholder: 'Password',
									confirmText: 'Reconnect',
								cancelText: 'Cancel',
								inputType: 'password',
								trim: false
							});
							if (entered !== null) await reconnectOnce(entered);
							else {
								throw error;
							}
						} else {
							throw error;
						}
					} else {
						throw error;
					}
				}

				// Re-open existing terminals for this connection so the UI can continue without "close/open new terminal".
				try {
					const { terminalStore } = await import('./terminal');
					await terminalStore.reopenTerminalsForConnection(connectionId);
				} catch (e) {
					console.error('Failed to reopen terminals after reconnect:', e);
				}

				update((s) => {
					const next = new Map(s.activeConnections);
					const conn = next.get(connectionId);
					if (conn) {
						next.set(connectionId, {
							...conn,
							status: 'connected',
							lastDisconnectDetail: null
						});
					}
					return { ...s, status: 'idle', activeConnections: next, error: null };
				});

				try {
					const { workspaceStore } = await import('./workspace');
					workspaceStore.markConnectionConnected(connectionId);
				} catch (e) {
					console.error('Failed to mark sessions connected:', e);
				}
			} catch (error) {
				update((s) => {
					const next = new Map(s.activeConnections);
					const conn = next.get(connectionId);
					if (conn) {
						next.set(connectionId, {
							...conn,
							status: 'disconnected',
							lastDisconnectDetail: error instanceof Error ? error.message : String(error)
						});
					}
					return { ...s, status: 'idle', activeConnections: next, error: null };
				});
				throw error;
			}
		},

		/**
		 * Connect to a server. Returns the connection ID and (possibly enriched) profile.
		 * Does not set any "active" connection - that's managed by workspaceStore sessions.
		 */
		async connect(
			profile: ConnectionProfile,
			password?: string
		): Promise<{ connectionId: string; profile: ConnectionProfile }> {
			update((s) => ({ ...s, status: 'connecting', error: null }));

			const connectOnce = async (): Promise<string> =>
				await invoke<string>('ssh_connect', {
					profile,
					password
				});

			try {
				let connectionId: string;
				try {
					connectionId = await connectOnce();
				} catch (error) {
					if (error instanceof TauriCommandError && error.code === 'ssh_hostkey_untrusted') {
						const ctx = parseHostKeyContext(error.context);
						if (ctx && 'fingerprintSha256' in ctx) {
							const confirmed = await confirmStore.confirm({
								title: 'Trust Host Key?',
								message:
									`The SSH server ${ctx.host}:${ctx.port} is presenting an untrusted host key.\n\n` +
									`Only trust this key if you are sure you’re connecting to the right machine.`,
								detail: `${ctx.keyType} ${ctx.fingerprintSha256}\n\n${ctx.publicKeyOpenssh}`,
								confirmText: 'Trust & Connect',
								cancelText: 'Cancel'
							});
							if (confirmed) {
								await invoke('ssh_trust_host_key', {
									request: {
										host: ctx.host,
										port: ctx.port,
										keyType: ctx.keyType,
										fingerprintSha256: ctx.fingerprintSha256,
										publicKeyOpenssh: ctx.publicKeyOpenssh
									}
								});
								connectionId = await connectOnce();
							} else {
								throw error;
							}
						} else {
							throw error;
						}
					} else if (error instanceof TauriCommandError && error.code === 'ssh_hostkey_mismatch') {
						const ctx = parseHostKeyContext(error.context);
						if (ctx && 'expectedFingerprintSha256' in ctx) {
							const confirmed = await confirmStore.confirm({
								title: 'Host Key Changed',
								message:
									`WARNING: The host key for ${ctx.host}:${ctx.port} has changed.\n\n` +
									`This can indicate a man-in-the-middle attack, or that the server was reinstalled.\n\n` +
									`Replace the saved key only if you’re sure this is expected.`,
								detail:
									`Expected: ${ctx.expectedFingerprintSha256}\n` +
									`Actual:   ${ctx.actualFingerprintSha256}\n\n` +
									`New key:\n${ctx.actualPublicKeyOpenssh}`,
								confirmText: 'Replace Key & Connect',
								cancelText: 'Cancel',
								destructive: true
							});
							if (confirmed) {
								await invoke('ssh_forget_host_key', { host: ctx.host, port: ctx.port });
								await invoke('ssh_trust_host_key', {
									request: {
										host: ctx.host,
										port: ctx.port,
										keyType: ctx.keyType,
										fingerprintSha256: ctx.actualFingerprintSha256,
										publicKeyOpenssh: ctx.actualPublicKeyOpenssh
									}
								});
								connectionId = await connectOnce();
							} else {
								throw error;
							}
						} else {
							throw error;
						}
					} else {
						throw error;
					}
				}

				const enrichedProfile = await this.applyHostKeyFingerprint(profile);
				const activeConn: ActiveConnection = {
					id: connectionId,
					profile: enrichedProfile,
					sessionCount: 0, // Will be incremented by workspaceStore when creating session
					status: 'connected',
					lastDisconnectDetail: null
				};

				if (enrichedProfile.authMethod === 'password' && password) {
					connectionSecrets.set(connectionId, { password });
				}

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

				return { connectionId, profile: enrichedProfile };
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
			disconnectingIds.add(connectionId);
			const controller = autoReconnectControllers.get(connectionId);
			if (controller) controller.cancelled = true;
			autoReconnectControllers.delete(connectionId);
			connectionSecrets.delete(connectionId);
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
			disconnectingIds.delete(connectionId);
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
			const conn = get({ subscribe }).activeConnections.get(connectionId);
			return !!conn && (conn.status ?? 'connected') === 'connected';
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

			if (isTauri() && !unlistenConnectionStatus) {
				unlistenConnectionStatus = await listen<{
					connectionId: string;
					status: 'connected' | 'disconnected';
					detail?: string | null;
				}>('connection_status_changed', async (payload) => {
					if (payload.status === 'connected') {
						update((s) => {
							const next = new Map(s.activeConnections);
							const active = next.get(payload.connectionId);
							if (!active) return s;
							next.set(payload.connectionId, {
								...active,
								status: 'connected',
								lastDisconnectDetail: null
							});
							return { ...s, activeConnections: next };
						});

						try {
							const { workspaceStore } = await import('./workspace');
							workspaceStore.markConnectionConnected(payload.connectionId);
						} catch (e) {
							console.error('Failed to mark sessions connected:', e);
						}
						return;
					}

					if (payload.status !== 'disconnected') return;

					const state = get({ subscribe });
					const active = state.activeConnections.get(payload.connectionId);
					if (!active) return;

					update((s) => {
						const newConnections = new Map(s.activeConnections);
						newConnections.set(payload.connectionId, {
							...active,
							status: 'disconnected',
							lastDisconnectDetail: payload.detail ?? null
						});
						return { ...s, activeConnections: newConnections };
					});

					notificationsStore.notifyOnce(`connection_lost:${payload.connectionId}`, {
						severity: 'warning',
						title: 'Connection Lost',
						message: `Disconnected from ${active.profile.username}@${active.profile.host}:${active.profile.port}.`,
						detail: payload.detail || 'Disconnected'
					});

					try {
						const { workspaceStore } = await import('./workspace');
						workspaceStore.markConnectionDisconnected(payload.connectionId, payload.detail ?? null);
					} catch (e) {
						console.error('Failed to reconcile sessions after disconnect:', e);
					}

					// Immediately begin background auto-reconnect for active workspaces.
					startAutoReconnect(payload.connectionId);
				});
			}
		},

		reset(): void {
			if (unlistenConnectionStatus) {
				unlistenConnectionStatus();
				unlistenConnectionStatus = null;
			}
			autoReconnectControllers.forEach((c) => (c.cancelled = true));
			autoReconnectControllers.clear();
			connectionSecrets.clear();
			disconnectingIds.clear();
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
