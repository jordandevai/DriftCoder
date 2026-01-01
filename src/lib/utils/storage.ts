import { load, Store } from '@tauri-apps/plugin-store';
import type { ConnectionProfile } from '$types';

let store: Store | null = null;

async function getStore(): Promise<Store> {
	if (!store) {
		store = await load('settings.json');
	}
	return store;
}

export async function loadSavedConnections(): Promise<ConnectionProfile[]> {
	try {
		const s = await getStore();
		const profiles = await s.get<ConnectionProfile[]>('connections');
		return profiles || [];
	} catch (error) {
		console.error('Failed to load saved connections:', error);
		return [];
	}
}

export async function saveConnections(profiles: ConnectionProfile[]): Promise<void> {
	try {
		const s = await getStore();
		await s.set('connections', profiles);
		await s.save();
	} catch (error) {
		console.error('Failed to save connections:', error);
	}
}
