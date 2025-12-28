//! Write-Ahead Logging (WAL) module.
//!
//! This module implements the write-ahead log for durability and crash recovery.
//! Planned for v0.2+.

use crate::Result;

/// Write-ahead log entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalEntryType {
    /// Insert or update operation
    Put,
    /// Delete operation
    Delete,
    /// Transaction commit
    Commit,
    /// Transaction rollback
    Rollback,
}

/// Write-ahead log entry (placeholder)
#[allow(dead_code)]
pub struct WalEntry {
    /// Type of the WAL entry
    pub entry_type: WalEntryType,
    /// Optional key for put/delete operations
    pub key: Option<Vec<u8>>,
    /// Optional value for put operations
    pub value: Option<Vec<u8>>,
    /// Optional transaction id
    pub tx_id: Option<u64>,
}

/// Write-ahead log (placeholder)
#[allow(dead_code)]
pub struct Wal {
    // In-memory log for placeholder behavior
    entries: Vec<WalEntry>,
}

impl Wal {
    /// Create a new WAL instance
    #[allow(dead_code)]
    pub fn new(_path: &std::path::Path) -> Result<Self> {
        // Placeholder: ignore path and keep an in-memory log
        Ok(Wal { entries: Vec::new() })
    }
    
    /// Append an entry to the log
    #[allow(dead_code)]
    pub fn append(&mut self, _entry: WalEntry) -> Result<u64> {
        // Append to in-memory vector and return its index as log offset
        self.entries.push(_entry);
        Ok((self.entries.len() - 1) as u64)
    }
    
    /// Sync the log to disk
    #[allow(dead_code)]
    pub fn sync(&mut self) -> Result<()> {
        // No-op for placeholder in-memory WAL
        Ok(())
    }
}
