import { writable } from 'svelte/store';
import { get } from 'svelte/store';
import { activeSession } from './workspace';

interface ConflictState {
	open: boolean;
	filePath: string | null;
	sessionId: string | null;
}

const initialState: ConflictState = {
	open: false,
	filePath: null,
	sessionId: null
};

function createConflictStore() {
	const { subscribe, set, update } = writable<ConflictState>(initialState);

	return {
		subscribe,

		open(filePath: string): void {
			const sessionId = get(activeSession)?.id ?? null;
			set({ open: true, filePath, sessionId });
		},

		close(): void {
			set(initialState);
		},

		reset(): void {
			set(initialState);
		}
	};
}

export const conflictStore = createConflictStore();
