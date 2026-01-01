#![allow(dead_code)]
use crate::ssh::client::SshConnection;
use crate::ssh::pty::PtySession;
use std::collections::HashMap;

/// Application state holding active connections and sessions
pub struct AppState {
    /// Active SSH connections keyed by connection ID
    pub connections: HashMap<String, SshConnection>,
    /// Active PTY sessions keyed by terminal ID
    pub terminals: HashMap<String, PtySession>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            terminals: HashMap::new(),
        }
    }

    pub fn add_connection(&mut self, id: String, connection: SshConnection) {
        self.connections.insert(id, connection);
    }

    #[allow(dead_code)]
    pub fn get_connection(&self, id: &str) -> Option<&SshConnection> {
        self.connections.get(id)
    }

    pub fn get_connection_mut(&mut self, id: &str) -> Option<&mut SshConnection> {
        self.connections.get_mut(id)
    }

    pub fn remove_connection(&mut self, id: &str) -> Option<SshConnection> {
        // Also remove any terminals associated with this connection
        self.terminals.retain(|_, term| term.connection_id != id);
        self.connections.remove(id)
    }

    pub fn add_terminal(&mut self, id: String, terminal: PtySession) {
        self.terminals.insert(id, terminal);
    }

    #[allow(dead_code)]
    pub fn get_terminal(&self, id: &str) -> Option<&PtySession> {
        self.terminals.get(id)
    }

    pub fn get_terminal_mut(&mut self, id: &str) -> Option<&mut PtySession> {
        self.terminals.get_mut(id)
    }

    pub fn remove_terminal(&mut self, id: &str) -> Option<PtySession> {
        self.terminals.remove(id)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
