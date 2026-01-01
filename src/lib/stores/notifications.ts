import { derived, get, writable } from 'svelte/store';

export type NotificationSeverity = 'info' | 'warning' | 'error';

export interface NotificationAction {
	label: string;
	run: () => void | Promise<void>;
}

export interface Notification {
	id: string;
	createdAt: number;
	severity: NotificationSeverity;
	title: string;
	message: string;
	detail?: string;
	sessionId?: string;
	readAt: number | null;
	dismissedAt: number | null;
	actions: NotificationAction[];
}

export interface NotifyInput {
	severity: NotificationSeverity;
	title: string;
	message: string;
	detail?: string;
	sessionId?: string;
	actions?: NotificationAction[];
}

interface NotificationState {
	centerOpen: boolean;
	selectedId: string | null;
	notifications: Notification[];
}

const initialState: NotificationState = {
	centerOpen: false,
	selectedId: null,
	notifications: []
};

function createNotificationsStore() {
	const { subscribe, set, update } = writable<NotificationState>(initialState);
	const lastByKey = new Map<string, { id: string; createdAt: number }>();

	return {
		subscribe,

		notify(input: NotifyInput): string {
			const id = crypto.randomUUID();
			const now = Date.now();

			const notification: Notification = {
				id,
				createdAt: now,
				severity: input.severity,
				title: input.title,
				message: input.message,
				detail: input.detail,
				sessionId: input.sessionId,
				readAt: null,
				dismissedAt: null,
				actions: input.actions ?? []
			};

			update((s) => ({
				...s,
				notifications: [notification, ...s.notifications],
				selectedId: s.selectedId ?? id
			}));

			return id;
		},

		notifyOnce(key: string, input: NotifyInput, windowMs = 10_000): string {
			const now = Date.now();
			const existing = lastByKey.get(key);
			if (existing && now - existing.createdAt < windowMs) {
				const state = get({ subscribe });
				const found = state.notifications.find((n) => n.id === existing.id);
				if (found && !found.dismissedAt) return existing.id;
			}

			const id = this.notify(input);
			lastByKey.set(key, { id, createdAt: now });
			return id;
		},

		markRead(id: string): void {
			update((s) => ({
				...s,
				notifications: s.notifications.map((n) =>
					n.id === id && !n.readAt ? { ...n, readAt: Date.now() } : n
				)
			}));
		},

		dismiss(id: string): void {
			update((s) => ({
				...s,
				notifications: s.notifications.map((n) =>
					n.id === id && !n.dismissedAt ? { ...n, dismissedAt: Date.now() } : n
				)
			}));
		},

		select(id: string | null): void {
			update((s) => ({ ...s, selectedId: id }));
			if (id) this.markRead(id);
		},

		openCenter(): void {
			update((s) => ({ ...s, centerOpen: true }));
		},

		closeCenter(): void {
			update((s) => ({ ...s, centerOpen: false }));
		},

		toggleCenter(): void {
			update((s) => ({ ...s, centerOpen: !s.centerOpen }));
		},

		clearAll(): void {
			update((s) => ({ ...s, notifications: [], selectedId: null }));
		},

		reset(): void {
			lastByKey.clear();
			set(initialState);
		}
	};
}

export const notificationsStore = createNotificationsStore();

export const unreadCount = derived(notificationsStore, ($s) =>
	$s.notifications.reduce((count, n) => count + (n.readAt ? 0 : 1), 0)
);

export const activePopups = derived(notificationsStore, ($s) =>
	$s.notifications.filter((n) => !n.dismissedAt && !n.readAt).slice(0, 3)
);

export const selectedNotification = derived(notificationsStore, ($s) =>
	$s.selectedId ? $s.notifications.find((n) => n.id === $s.selectedId) ?? null : null
);
