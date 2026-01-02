import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';
import type { IpcError } from '$types';

function isIpcError(value: unknown): value is IpcError {
	return (
		!!value &&
		typeof value === 'object' &&
		typeof (value as { code?: unknown }).code === 'string' &&
		typeof (value as { message?: unknown }).message === 'string'
	);
}

function extractIpcError(error: unknown): IpcError | null {
	if (isIpcError(error)) return error;

	if (error && typeof error === 'object') {
		const maybeInner = (error as { error?: unknown }).error;
		if (isIpcError(maybeInner)) return maybeInner;

		const maybeMessage = (error as { message?: unknown }).message;
		if (typeof maybeMessage === 'string') {
			try {
				const parsed = JSON.parse(maybeMessage) as unknown;
				if (isIpcError(parsed)) return parsed;
			} catch {
				// ignore
			}
		}
	}

	if (typeof error === 'string') {
		try {
			const parsed = JSON.parse(error) as unknown;
			if (isIpcError(parsed)) return parsed;
		} catch {
			// ignore
		}
	}

	return null;
}

export class TauriCommandError extends Error {
	code: string;
	raw?: string;
	context?: unknown;
	cmd: string;

	constructor(cmd: string, ipc: IpcError) {
		const withCode = `[${ipc.code}] ${ipc.message}`;
		const full = ipc.raw ? `${withCode}\n${ipc.raw}` : withCode;
		super(full);
		this.name = 'TauriCommandError';
		this.code = ipc.code;
		this.raw = ipc.raw;
		this.context = ipc.context;
		this.cmd = cmd;
	}
}

function toUnknownError(error: unknown): Error {
	if (error instanceof Error) return error;
	if (typeof error === 'string') return new Error(error);
	try {
		return new Error(JSON.stringify(error));
	} catch {
		return new Error(String(error));
	}
}

/**
 * Type-safe wrapper around Tauri's invoke function
 */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
	try {
		return await tauriInvoke<T>(cmd, args);
	} catch (error) {
		console.error(`Tauri command failed: ${cmd}`, error);
		const ipc = extractIpcError(error);
		if (ipc) {
			throw new TauriCommandError(cmd, ipc);
		}
		throw toUnknownError(error);
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
