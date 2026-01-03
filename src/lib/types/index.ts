// Connection types
export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';
export type AuthMethod = 'key' | 'password';

export interface ConnectionProfile {
	id: string;
	name: string;
	host: string;
	port: number;
	username: string;
	authMethod: AuthMethod;
	keyPath?: string;
	recentProjects: string[];
	bookmarkedPaths: string[];
}

// Legacy single-connection state (kept for backward compatibility during migration)
export interface ConnectionStateLegacy {
	status: ConnectionStatus;
	activeConnection: ConnectionProfile | null;
	connectionId: string | null;
	savedProfiles: ConnectionProfile[];
	error: string | null;
}

// New multi-connection state
export interface ConnectionState {
	status: 'idle' | 'connecting'; // Global status for connection operations
	activeConnections: Map<string, ActiveConnection>; // connectionId -> ActiveConnection
	savedProfiles: ConnectionProfile[];
	error: string | null;
}

// File system types
export interface FileEntry {
	name: string;
	path: string;
	isDirectory: boolean;
	size: number;
	mtime: number;
	permissions?: string;
	children?: FileEntry[];
}

export interface OpenFile {
	path: string;
	content: string;
	language: string;
	dirty: boolean;
	remoteMtime: number;
	remoteSize?: number;
	// Remote sync metadata (optional to keep compatibility)
	remoteLastCheckedAt?: number;
	remoteMtimeOnServer?: number;
	remoteSizeOnServer?: number;
	remoteChanged?: boolean;
	remoteUpdateAvailable?: boolean;
	remoteMissing?: boolean;
}

export interface FileState {
	projectRoot: string;
	tree: FileEntry[];
	expandedPaths: Set<string>;
	openFiles: Map<string, OpenFile>;
	activeFilePath: string | null;
}

// Layout types
export type PanelType = 'editor' | 'terminal';

export interface Panel {
	id: string;
	type: PanelType;
	title: string;
	filePath?: string;
	terminalId?: string;
}

export interface PanelGroup {
	id: string;
	panels: Panel[];
	activePanelId: string | null;
}

export type SplitDirection = 'horizontal' | 'vertical';

export type LayoutNode =
	| { type: 'leaf'; groupId: string }
	| { type: 'split'; direction: SplitDirection; children: LayoutNode[]; sizes: number[] };

export interface LayoutState {
	root: LayoutNode;
	groups: Map<string, PanelGroup>;
	activeGroupId: string | null;
	fileTreeWidth: number;
	fileTreeCollapsed: boolean;
}

// Settings types
export interface SettingsState {
	fontSize: number;
	tabSize: number;
	wordWrap: boolean;
	autosave: boolean;
	autosaveDelay: number;
	terminalScrollback: number;
	themeMode: 'dark' | 'light' | 'system';
	themeOverrides: {
		dark?: {
			ui?: Partial<Record<string, string>>;
			terminal?: Partial<Record<string, string>>;
			terminalMinimumContrastRatio?: number;
		};
		light?: {
			ui?: Partial<Record<string, string>>;
			terminal?: Partial<Record<string, string>>;
			terminalMinimumContrastRatio?: number;
		};
	};
}

// Terminal types
export interface TerminalSession {
	id: string;
	title: string;
	sessionId: string; // Links terminal to a workspace session
}

// Workspace/Session types (multi-project support)
export interface SessionFileState {
	tree: FileEntry[];
	expandedPaths: Set<string>;
	openFiles: Map<string, OpenFile>;
	activeFilePath: string | null;
}

export interface SessionLayoutState {
	root: LayoutNode;
	groups: Map<string, PanelGroup>;
	activeGroupId: string | null;
	fileTreeWidth: number;
	fileTreeCollapsed: boolean;
}

export interface Session {
	id: string;
	connectionId: string;
	connectionProfile: ConnectionProfile;
	connectionStatus?: 'connected' | 'disconnected';
	connectionDetail?: string | null;
	projectRoot: string;
	displayName: string; // e.g., "server:folder"
	fileState: SessionFileState;
	terminalIds: string[];
	layoutState: SessionLayoutState;
}

export interface WorkspaceState {
	sessions: Map<string, Session>;
	activeSessionId: string | null;
	sessionOrder: string[]; // Tab order
}

export interface ActiveConnection {
	id: string;
	profile: ConnectionProfile;
	sessionCount: number; // Number of sessions using this connection
	status?: 'connected' | 'disconnected' | 'reconnecting';
	lastDisconnectDetail?: string | null;
}

export interface IpcError {
	code: string;
	message: string;
	raw?: string;
	context?: unknown;
}
