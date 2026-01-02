import { writable } from 'svelte/store';

export interface PromptRequest {
	title: string;
	message?: string;
	detail?: string;
	placeholder?: string;
	initialValue?: string;
	confirmText?: string;
	cancelText?: string;
}

interface PromptState {
	active: PromptRequest | null;
	value: string;
}

const initialState: PromptState = { active: null, value: '' };

function createPromptStore() {
	const { subscribe, set, update } = writable<PromptState>(initialState);
	let resolver: ((value: string | null) => void) | null = null;

	return {
		subscribe,

		async prompt(request: PromptRequest): Promise<string | null> {
			if (resolver) {
				// Only allow one prompt at a time.
				resolver(null);
				resolver = null;
			}

			const initialValue = request.initialValue ?? '';
			set({ active: request, value: initialValue });

			return await new Promise<string | null>((resolve) => {
				resolver = resolve;
			});
		},

		setValue(value: string): void {
			update((s) => ({ ...s, value }));
		},

		resolve(value: string | null): void {
			if (resolver) resolver(value);
			resolver = null;
			set(initialState);
		},

		close(): void {
			this.resolve(null);
		}
	};
}

export const promptStore = createPromptStore();

