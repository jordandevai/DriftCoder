import { writable } from 'svelte/store';

export interface ConfirmOptions {
	title: string;
	message: string;
	detail?: string;
	confirmText?: string;
	cancelText?: string;
	destructive?: boolean;
}

export interface ConfirmRequest extends Required<Pick<ConfirmOptions, 'title' | 'message'>> {
	id: string;
	detail?: string;
	confirmText: string;
	cancelText: string;
	destructive: boolean;
}

interface ConfirmState {
	active: ConfirmRequest | null;
}

const initialState: ConfirmState = { active: null };

function createConfirmStore() {
	const { subscribe, set } = writable<ConfirmState>(initialState);

	type PendingConfirm = {
		request: ConfirmRequest;
		resolve: (confirmed: boolean) => void;
	};

	let activeResolve: ((confirmed: boolean) => void) | null = null;
	const queue: PendingConfirm[] = [];

	function activateNext() {
		if (activeResolve) return;
		const next = queue.shift();
		if (!next) {
			set(initialState);
			return;
		}
		activeResolve = next.resolve;
		set({ active: next.request });
	}

	return {
		subscribe,

		async confirm(options: ConfirmOptions): Promise<boolean> {
			const request: ConfirmRequest = {
				id: crypto.randomUUID(),
				title: options.title,
				message: options.message,
				detail: options.detail,
				confirmText: options.confirmText ?? 'Confirm',
				cancelText: options.cancelText ?? 'Cancel',
				destructive: options.destructive ?? false
			};

			return await new Promise<boolean>((resolve) => {
				queue.push({ request, resolve });
				activateNext();
			});
		},

		resolve(confirmed: boolean): void {
			if (activeResolve) {
				activeResolve(confirmed);
				activeResolve = null;
			}
			activateNext();
		},

		reset(): void {
			activeResolve = null;
			queue.length = 0;
			set(initialState);
		}
	};
}

export const confirmStore = createConfirmStore();
