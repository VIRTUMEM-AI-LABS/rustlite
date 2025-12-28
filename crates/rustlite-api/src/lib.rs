//! # RustLite
//!
//! A lightweight, high-performance embedded database written in Rust with ACID guarantees.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rustlite::Database;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a persistent database (data survives restarts)
//!     let db = Database::open("./my_database")?;
//!
//!     // Insert data
//!     db.put(b"user:1:name", b"Alice")?;
//!     db.put(b"user:1:email", b"alice@example.com")?;
//!
//!     // Retrieve data
//!     if let Some(name) = db.get(b"user:1:name")? {
//!         println!("Name: {}", String::from_utf8_lossy(&name));
//!     }
//!
//!     // Delete data
//!     db.delete(b"user:1:email")?;
//!
//!     // Data is automatically persisted to disk
//!     Ok(())
//! }
//! ```
//!
//! ## Database Modes
//!
//! ```rust,no_run
//! use rustlite::Database;
//!
//! // Persistent database (recommended for production)
//! let persistent_db = Database::open("./data")?;
//!
//! // In-memory database (fast, but data lost on exit)
//! let memory_db = Database::in_memory()?;
//! # Ok::<(), rustlite::Error>(())
//! ```
//!
//! ## Features
//!
//! - **v0.1.0**: In-memory key-value store with thread-safe concurrent access
//! - **v0.2.0**: Persistent storage with WAL, SSTable, and crash recovery
//! - **v0.3.0** (planned): Indexing and performance optimizations
//! - **v0.4.0** (planned): SQL-like query engine
//! - **v1.0.0** (planned): Production-ready with full ACID guarantees
//!
//! See [ROADMAP.md](https://github.com/VIRTUMEM-AI-LABS/rustlite/blob/main/ROADMAP.md) for details.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

// Re-export core types
pub use rustlite_core::{Error, Result};

// WAL components
pub use rustlite_wal::{
    RecoveryManager, RecoveryStats, SyncMode, WalConfig, WalManager, WalReader, WalRecord,
};

// Storage components
pub use rustlite_storage::{
    CompactionConfig, CompactionStats, CompactionWorker, Manifest, Memtable, MemtableEntry,
    SSTableEntry, SSTableMeta, SSTableReader, SSTableWriter, StorageConfig, StorageEngine,
    StorageStats,
};

// Snapshot components
pub use rustlite_snapshot::{
    SnapshotConfig, SnapshotFile, SnapshotManager, SnapshotMeta, SnapshotType,
};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Storage backend for the database
enum StorageBackend {
    /// In-memory storage using HashMap
    Memory(RwLock<HashMap<Vec<u8>, Vec<u8>>>),
    /// Persistent storage using LSM-tree
    Persistent(StorageEngine),
}

/// The main database handle.
///
/// Provides a unified interface for both in-memory and persistent storage.
/// Thread-safe and can be cloned to share across threads.
///
/// # Examples
///
/// ```rust,no_run
/// use rustlite::Database;
///
/// // Open a persistent database
/// let db = Database::open("./my_data")?;
/// db.put(b"key", b"value")?;
///
/// // Data persists across restarts
/// drop(db);
/// let db = Database::open("./my_data")?;
/// assert_eq!(db.get(b"key")?, Some(b"value".to_vec()));
/// # Ok::<(), rustlite::Error>(())
/// ```
#[derive(Clone)]
pub struct Database {
    inner: Arc<StorageBackend>,
}

impl Database {
    /// Opens a persistent database at the specified path.
    ///
    /// Creates the directory if it doesn't exist. Data is persisted to disk
    /// using a Write-Ahead Log (WAL) and SSTable files.
    ///
    /// # Arguments
    ///
    /// * `path` - Directory path where database files will be stored
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rustlite::Database;
    ///
    /// let db = Database::open("./my_database")?;
    /// db.put(b"hello", b"world")?;
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let engine = StorageEngine::open(path)?;
        Ok(Database {
            inner: Arc::new(StorageBackend::Persistent(engine)),
        })
    }

    /// Opens a persistent database with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `path` - Directory path where database files will be stored
    /// * `config` - Storage configuration options
    pub fn open_with_config<P: AsRef<Path>>(path: P, config: StorageConfig) -> Result<Self> {
        let engine = StorageEngine::open_with_config(path, config)?;
        Ok(Database {
            inner: Arc::new(StorageBackend::Persistent(engine)),
        })
    }

    /// Creates an in-memory database.
    ///
    /// Data is stored only in memory and will be lost when the database
    /// is dropped. Useful for testing or temporary data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rustlite::Database;
    ///
    /// let db = Database::in_memory()?;
    /// db.put(b"temp", b"data")?;
    /// // Data is lost when db goes out of scope
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn in_memory() -> Result<Self> {
        Ok(Database {
            inner: Arc::new(StorageBackend::Memory(RwLock::new(HashMap::new()))),
        })
    }

    /// Creates a new in-memory database (alias for `in_memory()`).
    ///
    /// For backward compatibility with v0.1.0.
    #[deprecated(since = "0.2.0", note = "Use `Database::open()` for persistent storage or `Database::in_memory()` for temporary storage")]
    pub fn new() -> Result<Self> {
        Self::in_memory()
    }

    /// Inserts or updates a key-value pair.
    ///
    /// If the key already exists, its value will be updated.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to insert
    /// * `value` - The value to associate with the key
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rustlite::Database;
    ///
    /// let db = Database::open("./data")?;
    /// db.put(b"name", b"Alice")?;
    /// db.put(b"name", b"Bob")?; // Updates the value
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        match self.inner.as_ref() {
            StorageBackend::Memory(store) => {
                let mut store = store.write().map_err(|_| Error::LockPoisoned)?;
                store.insert(key.to_vec(), value.to_vec());
                Ok(())
            }
            StorageBackend::Persistent(engine) => engine.put(key, value),
        }
    }

    /// Retrieves a value by key.
    ///
    /// Returns `None` if the key doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rustlite::Database;
    ///
    /// let db = Database::open("./data")?;
    /// db.put(b"greeting", b"Hello!")?;
    ///
    /// match db.get(b"greeting")? {
    ///     Some(value) => println!("Found: {}", String::from_utf8_lossy(&value)),
    ///     None => println!("Key not found"),
    /// }
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        match self.inner.as_ref() {
            StorageBackend::Memory(store) => {
                let store = store.read().map_err(|_| Error::LockPoisoned)?;
                Ok(store.get(key).cloned())
            }
            StorageBackend::Persistent(engine) => engine.get(key),
        }
    }

    /// Deletes a key-value pair.
    ///
    /// Returns `true` if the key existed and was deleted, `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to delete
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rustlite::Database;
    ///
    /// let db = Database::open("./data")?;
    /// db.put(b"temp", b"value")?;
    /// db.delete(b"temp")?;
    /// assert_eq!(db.get(b"temp")?, None);
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn delete(&self, key: &[u8]) -> Result<bool> {
        match self.inner.as_ref() {
            StorageBackend::Memory(store) => {
                let mut store = store.write().map_err(|_| Error::LockPoisoned)?;
                Ok(store.remove(key).is_some())
            }
            StorageBackend::Persistent(engine) => {
                // Check if key exists before deleting
                let existed = engine.get(key)?.is_some();
                if existed {
                    engine.delete(key)?;
                }
                Ok(existed)
            }
        }
    }

    /// Forces all pending writes to disk.
    ///
    /// For persistent databases, this flushes the memtable to SSTable
    /// and syncs the WAL. For in-memory databases, this is a no-op.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rustlite::Database;
    ///
    /// let db = Database::open("./data")?;
    /// db.put(b"important", b"data")?;
    /// db.sync()?; // Ensure data is on disk
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn sync(&self) -> Result<()> {
        match self.inner.as_ref() {
            StorageBackend::Memory(_) => Ok(()),
            StorageBackend::Persistent(engine) => engine.sync(),
        }
    }

    /// Returns whether this is a persistent database.
    pub fn is_persistent(&self) -> bool {
        matches!(self.inner.as_ref(), StorageBackend::Persistent(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.2.0");
    }

    #[test]
    fn test_in_memory_database() {
        let db = Database::in_memory().unwrap();
        db.put(b"key", b"value").unwrap();
        assert_eq!(db.get(b"key").unwrap(), Some(b"value".to_vec()));
        assert!(!db.is_persistent());
    }

    #[test]
    fn test_persistent_database() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path()).unwrap();
        
        db.put(b"persist", b"data").unwrap();
        assert_eq!(db.get(b"persist").unwrap(), Some(b"data".to_vec()));
        assert!(db.is_persistent());
    }

    #[test]
    fn test_persistence_across_reopens() {
        let dir = tempdir().unwrap();
        
        // Write data
        {
            let db = Database::open(dir.path()).unwrap();
            db.put(b"key1", b"value1").unwrap();
            db.put(b"key2", b"value2").unwrap();
            db.sync().unwrap();
        }
        
        // Reopen and verify
        {
            let db = Database::open(dir.path()).unwrap();
            assert_eq!(db.get(b"key1").unwrap(), Some(b"value1".to_vec()));
            assert_eq!(db.get(b"key2").unwrap(), Some(b"value2".to_vec()));
        }
    }

    #[test]
    fn test_delete() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path()).unwrap();
        
        db.put(b"key", b"value").unwrap();
        assert!(db.delete(b"key").unwrap());
        assert_eq!(db.get(b"key").unwrap(), None);
        assert!(!db.delete(b"key").unwrap()); // Already deleted
    }

    #[test]
    fn test_update() {
        let db = Database::in_memory().unwrap();
        
        db.put(b"counter", b"1").unwrap();
        assert_eq!(db.get(b"counter").unwrap(), Some(b"1".to_vec()));
        
        db.put(b"counter", b"2").unwrap();
        assert_eq!(db.get(b"counter").unwrap(), Some(b"2".to_vec()));
    }

    #[test]
    #[allow(deprecated)]
    fn test_backward_compatibility() {
        // Database::new() still works but is deprecated
        let db = Database::new().unwrap();
        db.put(b"key", b"value").unwrap();
        assert_eq!(db.get(b"key").unwrap(), Some(b"value".to_vec()));
    }
}
