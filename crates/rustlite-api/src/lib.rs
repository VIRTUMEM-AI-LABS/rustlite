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
//! ## Indexing (v0.3.0+)
//!
//! ```rust
//! use rustlite::{Database, IndexType};
//!
//! let db = Database::in_memory()?;
//!
//! // Create indexes
//! db.create_index("users_by_name", IndexType::BTree)?;
//! db.create_index("sessions", IndexType::Hash)?;
//!
//! // Use indexes for fast lookups
//! db.index_insert("users_by_name", b"alice", 100)?;
//! db.index_insert("users_by_name", b"bob", 101)?;
//!
//! let results = db.index_find("users_by_name", b"alice")?;
//! # Ok::<(), rustlite::Error>(())
//! ```
//!
//! ## Features
//!
//! - **v0.1.0**: In-memory key-value store with thread-safe concurrent access
//! - **v0.2.0**: Persistent storage with WAL, SSTable, and crash recovery
//! - **v0.3.0**: B-Tree and Hash indexing for fast lookups
//! - **v0.4.0** (planned): SQL-like query engine
//! - **v1.0.0** (planned): Production-ready with full ACID guarantees
//!
//! See [ROADMAP.md](https://github.com/VIRTUMEM-AI-LABS/rustlite/blob/main/docs/ROADMAP.md) for details.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

// Re-export core types
pub use rustlite_core::{Error, Result};
pub use rustlite_core::index::{BTreeIndex, HashIndex, Index, IndexInfo, IndexManager, IndexType};

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

/// Inner database state
struct DatabaseInner {
    /// Storage backend
    storage: StorageBackend,
    /// Index manager for secondary indexes
    indexes: RwLock<IndexManager>,
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
    inner: Arc<DatabaseInner>,
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
            inner: Arc::new(DatabaseInner {
                storage: StorageBackend::Persistent(engine),
                indexes: RwLock::new(IndexManager::new()),
            }),
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
            inner: Arc::new(DatabaseInner {
                storage: StorageBackend::Persistent(engine),
                indexes: RwLock::new(IndexManager::new()),
            }),
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
            inner: Arc::new(DatabaseInner {
                storage: StorageBackend::Memory(RwLock::new(HashMap::new())),
                indexes: RwLock::new(IndexManager::new()),
            }),
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
        match &self.inner.storage {
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
        match &self.inner.storage {
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
        match &self.inner.storage {
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
        match &self.inner.storage {
            StorageBackend::Memory(_) => Ok(()),
            StorageBackend::Persistent(engine) => engine.sync(),
        }
    }

    /// Returns whether this is a persistent database.
    pub fn is_persistent(&self) -> bool {
        matches!(&self.inner.storage, StorageBackend::Persistent(_))
    }

    // =========================================================================
    // Index Operations (v0.3.0+)
    // =========================================================================

    /// Creates a new index with the specified name and type.
    ///
    /// # Arguments
    ///
    /// * `name` - Unique name for the index
    /// * `index_type` - Type of index (BTree for range queries, Hash for fast lookups)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rustlite::{Database, IndexType};
    ///
    /// let db = Database::in_memory()?;
    /// db.create_index("users_by_name", IndexType::BTree)?;
    /// db.create_index("sessions", IndexType::Hash)?;
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn create_index(&self, name: &str, index_type: IndexType) -> Result<()> {
        let mut indexes = self.inner.indexes.write().map_err(|_| Error::LockPoisoned)?;
        indexes.create_index(name, index_type)
    }

    /// Drops an index by name.
    ///
    /// Returns `true` if the index existed and was dropped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rustlite::{Database, IndexType};
    ///
    /// let db = Database::in_memory()?;
    /// db.create_index("temp_index", IndexType::Hash)?;
    /// assert!(db.drop_index("temp_index")?);
    /// assert!(!db.drop_index("temp_index")?); // Already dropped
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn drop_index(&self, name: &str) -> Result<bool> {
        let mut indexes = self.inner.indexes.write().map_err(|_| Error::LockPoisoned)?;
        indexes.drop_index(name)
    }

    /// Inserts a key-value pair into a named index.
    ///
    /// The value is typically a record ID or offset pointing to the actual data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rustlite::{Database, IndexType};
    ///
    /// let db = Database::in_memory()?;
    /// db.create_index("names", IndexType::BTree)?;
    ///
    /// // Index "alice" pointing to record ID 100
    /// db.index_insert("names", b"alice", 100)?;
    /// db.index_insert("names", b"bob", 101)?;
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn index_insert(&self, name: &str, key: &[u8], value: u64) -> Result<()> {
        let mut indexes = self.inner.indexes.write().map_err(|_| Error::LockPoisoned)?;
        indexes.insert(name, key, value)
    }

    /// Finds all values matching a key in a named index.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rustlite::{Database, IndexType};
    ///
    /// let db = Database::in_memory()?;
    /// db.create_index("names", IndexType::Hash)?;
    /// db.index_insert("names", b"alice", 100)?;
    ///
    /// let results = db.index_find("names", b"alice")?;
    /// assert_eq!(results, vec![100]);
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn index_find(&self, name: &str, key: &[u8]) -> Result<Vec<u64>> {
        let indexes = self.inner.indexes.read().map_err(|_| Error::LockPoisoned)?;
        indexes.find(name, key)
    }

    /// Removes a key from a named index.
    ///
    /// Returns `true` if the key existed and was removed.
    pub fn index_remove(&self, name: &str, key: &[u8]) -> Result<bool> {
        let mut indexes = self.inner.indexes.write().map_err(|_| Error::LockPoisoned)?;
        indexes.remove(name, key)
    }

    /// Lists all index names in the database.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rustlite::{Database, IndexType};
    ///
    /// let db = Database::in_memory()?;
    /// db.create_index("idx1", IndexType::BTree)?;
    /// db.create_index("idx2", IndexType::Hash)?;
    ///
    /// let names = db.list_indexes()?;
    /// assert_eq!(names.len(), 2);
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn list_indexes(&self) -> Result<Vec<String>> {
        let indexes = self.inner.indexes.read().map_err(|_| Error::LockPoisoned)?;
        Ok(indexes.list_indexes().iter().map(|s| s.to_string()).collect())
    }

    /// Gets information about all indexes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rustlite::{Database, IndexType};
    ///
    /// let db = Database::in_memory()?;
    /// db.create_index("users", IndexType::BTree)?;
    /// db.index_insert("users", b"alice", 1)?;
    ///
    /// for info in db.index_info()? {
    ///     println!("Index: {}, Type: {}, Entries: {}", 
    ///              info.name, info.index_type, info.entry_count);
    /// }
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn index_info(&self) -> Result<Vec<IndexInfo>> {
        let indexes = self.inner.indexes.read().map_err(|_| Error::LockPoisoned)?;
        Ok(indexes.index_info())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.3.0");
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

    // Index tests
    #[test]
    fn test_create_and_drop_index() {
        let db = Database::in_memory().unwrap();
        
        db.create_index("test_idx", IndexType::BTree).unwrap();
        assert_eq!(db.list_indexes().unwrap().len(), 1);
        
        assert!(db.drop_index("test_idx").unwrap());
        assert_eq!(db.list_indexes().unwrap().len(), 0);
    }

    #[test]
    fn test_btree_index_operations() {
        let db = Database::in_memory().unwrap();
        db.create_index("names", IndexType::BTree).unwrap();
        
        db.index_insert("names", b"alice", 100).unwrap();
        db.index_insert("names", b"bob", 101).unwrap();
        db.index_insert("names", b"charlie", 102).unwrap();
        
        assert_eq!(db.index_find("names", b"bob").unwrap(), vec![101]);
        
        assert!(db.index_remove("names", b"bob").unwrap());
        assert!(db.index_find("names", b"bob").unwrap().is_empty());
    }

    #[test]
    fn test_hash_index_operations() {
        let db = Database::in_memory().unwrap();
        db.create_index("sessions", IndexType::Hash).unwrap();
        
        db.index_insert("sessions", b"sess:abc", 500).unwrap();
        db.index_insert("sessions", b"sess:def", 501).unwrap();
        
        assert_eq!(db.index_find("sessions", b"sess:abc").unwrap(), vec![500]);
        assert!(db.index_find("sessions", b"nonexistent").unwrap().is_empty());
    }

    #[test]
    fn test_index_info() {
        let db = Database::in_memory().unwrap();
        db.create_index("idx1", IndexType::BTree).unwrap();
        db.create_index("idx2", IndexType::Hash).unwrap();
        
        db.index_insert("idx1", b"key1", 1).unwrap();
        db.index_insert("idx1", b"key2", 2).unwrap();
        db.index_insert("idx2", b"key3", 3).unwrap();
        
        let info = db.index_info().unwrap();
        assert_eq!(info.len(), 2);
    }
}
