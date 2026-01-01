import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';

/**
 * Type-safe wrapper around Tauri's invoke function
 */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
	try {
		return await tauriInvoke<T>(cmd, args);
	} catch (error) {
		console.error(`Tauri command failed: ${cmd}`, error);
		throw error;
	}
}

/**
 * Type-safe wrapper around Tauri's event listener
 */
export function listen<T>(event: string, callback: (payload: T) => void): Promise<UnlistenFn> {
	return tauriListen<T>(event, (e) => callback(e.payload));
}

/**
 * Check if we're running in Tauri environment
 */
export function isTauri(): boolean {
	return '__TAURI__' in window;
}
