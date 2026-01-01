import type { FileEntry } from '$types';

export function sortEntries(entries: FileEntry[]): FileEntry[] {
	return [...entries].sort((a, b) => {
		if (a.isDirectory && !b.isDirectory) return -1;
		if (!a.isDirectory && b.isDirectory) return 1;
		return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
	});
}

