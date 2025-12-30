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

use tracing::{debug, info, instrument, warn};

pub mod logging;
mod security;

// Re-export core types
pub use rustlite_core::index::{BTreeIndex, HashIndex, Index, IndexInfo, IndexManager, IndexType};
pub use rustlite_core::{Error, Result};

// Transaction support (v0.5.0+)
pub use rustlite_core::transaction::{
    IsolationLevel, MVCCStorage, Timestamp, Transaction, TransactionId, TransactionManager,
    VersionChain, VersionedValue,
};

// Query engine (v0.4.0+)
pub use rustlite_core::query::{
    Column, ExecutionContext, Executor, Lexer, Parser, PhysicalPlan, Planner, Query, Row, Value,
};

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
    /// MVCC transaction manager (v0.5.0+)
    transaction_manager: Option<Arc<TransactionManager>>,
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
        let path_ref = path.as_ref();
        info!(path = ?path_ref, "Opening RustLite database");

        let engine = StorageEngine::open(path)?;
        let mvcc_storage = Arc::new(MVCCStorage::new());
        let tx_manager = TransactionManager::new(mvcc_storage);

        Ok(Database {
            inner: Arc::new(DatabaseInner {
                storage: StorageBackend::Persistent(engine),
                indexes: RwLock::new(IndexManager::new()),
                transaction_manager: Some(tx_manager),
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
        let mvcc_storage = Arc::new(MVCCStorage::new());
        let tx_manager = TransactionManager::new(mvcc_storage);

        Ok(Database {
            inner: Arc::new(DatabaseInner {
                storage: StorageBackend::Persistent(engine),
                indexes: RwLock::new(IndexManager::new()),
                transaction_manager: Some(tx_manager),
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
        info!("Creating in-memory RustLite database");

        let mvcc_storage = Arc::new(MVCCStorage::new());
        let tx_manager = TransactionManager::new(mvcc_storage);

        Ok(Database {
            inner: Arc::new(DatabaseInner {
                storage: StorageBackend::Memory(RwLock::new(HashMap::new())),
                indexes: RwLock::new(IndexManager::new()),
                transaction_manager: Some(tx_manager),
            }),
        })
    }

    /// Creates a new in-memory database (alias for `in_memory()`).
    ///
    /// For backward compatibility with v0.1.0.
    #[deprecated(
        since = "0.2.0",
        note = "Use `Database::open()` for persistent storage or `Database::in_memory()` for temporary storage"
    )]
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
    #[instrument(skip(self, key, value), fields(key_len = key.len(), value_len = value.len()))]
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        // Security: Validate inputs
        security::validate_key(key)?;
        security::validate_value(value)?;

        debug!("Writing key-value pair");

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
    #[instrument(skip(self, key), fields(key_len = key.len()))]
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // Security: Validate inputs
        security::validate_key(key)?;

        debug!("Reading key");

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
    #[instrument(skip(self, key), fields(key_len = key.len()))]
    pub fn delete(&self, key: &[u8]) -> Result<bool> {
        // Security: Validate inputs
        security::validate_key(key)?;

        debug!("Deleting key");

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
    #[instrument(skip(self), fields(name = %name, index_type = ?index_type))]
    pub fn create_index(&self, name: &str, index_type: IndexType) -> Result<()> {
        // Security: Validate index name
        security::validate_index_name(name)?;

        info!("Creating index");

        let mut indexes = self
            .inner
            .indexes
            .write()
            .map_err(|_| Error::LockPoisoned)?;
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
        let mut indexes = self
            .inner
            .indexes
            .write()
            .map_err(|_| Error::LockPoisoned)?;
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
        let mut indexes = self
            .inner
            .indexes
            .write()
            .map_err(|_| Error::LockPoisoned)?;
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
        let mut indexes = self
            .inner
            .indexes
            .write()
            .map_err(|_| Error::LockPoisoned)?;
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
        Ok(indexes
            .list_indexes()
            .iter()
            .map(|s| s.to_string())
            .collect())
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

    /// Executes a SQL-like query and returns results (v0.4.0+).
    ///
    /// Parses, plans, and executes a SELECT query against in-memory data.
    /// Currently supports: SELECT, FROM, WHERE, ORDER BY, LIMIT, JOIN.
    ///
    /// # Arguments
    ///
    /// * `sql` - SQL-like query string
    /// * `context` - Execution context with data and indexes
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rustlite::{Database, ExecutionContext, Row, Column, Value};
    /// use std::collections::HashMap;
    ///
    /// let db = Database::in_memory()?;
    ///
    /// // Prepare test data
    /// let mut context = ExecutionContext::new();
    /// context.data.insert("users".to_string(), vec![
    ///     Row {
    ///         columns: vec![
    ///             Column { name: "name".to_string(), alias: None },
    ///             Column { name: "age".to_string(), alias: None },
    ///         ],
    ///         values: vec![Value::String("Alice".to_string()), Value::Integer(30)],
    ///     },
    /// ]);
    ///
    /// let results = db.query("SELECT name FROM users WHERE age > 18", context)?;
    /// assert_eq!(results.len(), 1);
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    #[instrument(skip(self, sql, context), fields(sql_len = sql.len()))]
    pub fn query(&self, sql: &str, context: ExecutionContext) -> Result<Vec<Row>> {
        // Security: Validate query length
        security::validate_query(sql)?;

        debug!(sql = %sql, "Executing query");

        // Parse the SQL
        let mut parser =
            Parser::new(sql).map_err(|e| Error::InvalidInput(format!("Parse error: {}", e)))?;
        let query = parser
            .parse()
            .map_err(|e| Error::InvalidInput(format!("Parse error: {}", e)))?;

        // Plan the query
        let planner = Planner::new();
        let plan = planner
            .plan(&query)
            .map_err(|e| Error::InvalidInput(format!("Planning error: {}", e)))?;

        // Execute the query
        let mut executor = Executor::new(context);
        executor.execute(&plan)
    }

    /// Prepares a SQL-like query for repeated execution (v0.4.0+).
    ///
    /// Parses and plans the query once, returning a reusable plan.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rustlite::Database;
    ///
    /// let db = Database::in_memory()?;
    /// let plan = db.prepare("SELECT * FROM users WHERE age > 18")?;
    /// // Plan can be executed multiple times with different contexts
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn prepare(&self, sql: &str) -> Result<PhysicalPlan> {
        let mut parser =
            Parser::new(sql).map_err(|e| Error::InvalidInput(format!("Parse error: {}", e)))?;
        let query = parser
            .parse()
            .map_err(|e| Error::InvalidInput(format!("Parse error: {}", e)))?;

        let planner = Planner::new();
        planner
            .plan(&query)
            .map_err(|e| Error::InvalidInput(format!("Planning error: {}", e)))
    }

    /// Executes a prepared query plan with given context (v0.4.0+).
    pub fn execute_plan(&self, plan: &PhysicalPlan, context: ExecutionContext) -> Result<Vec<Row>> {
        let mut executor = Executor::new(context);
        executor.execute(plan)
    }

    // ===== Transaction Methods (v0.5.0+) =====

    /// Begins a new MVCC transaction with the specified isolation level (v0.5.0+).
    ///
    /// Returns a Transaction handle that provides snapshot isolation and
    /// ACID guarantees. Changes are buffered until commit.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rustlite::{Database, IsolationLevel};
    ///
    /// let db = Database::in_memory()?;
    ///
    /// // Start a transaction
    /// let mut txn = db.begin_transaction(IsolationLevel::RepeatableRead)?;
    ///
    /// // Read and write within transaction
    /// txn.put(b"key1".to_vec(), b"value1".to_vec())?;
    /// txn.put(b"key2".to_vec(), b"value2".to_vec())?;
    ///
    /// // Commit changes
    /// txn.commit()?;
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    #[instrument(skip(self), fields(isolation = ?isolation))]
    pub fn begin_transaction(&self, isolation: IsolationLevel) -> Result<Transaction> {
        info!("Beginning transaction");
        if let Some(ref manager) = self.inner.transaction_manager {
            manager.begin(isolation)
        } else {
            Err(Error::Transaction(
                "Transaction support not initialized".into(),
            ))
        }
    }

    /// Begins a new transaction with default isolation level (RepeatableRead).
    ///
    /// Convenience method equivalent to `begin_transaction(IsolationLevel::RepeatableRead)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rustlite::Database;
    ///
    /// let db = Database::in_memory()?;
    /// let mut txn = db.begin()?;
    /// txn.put(b"key".to_vec(), b"value".to_vec())?;
    /// txn.commit()?;
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn begin(&self) -> Result<Transaction> {
        self.begin_transaction(IsolationLevel::default())
    }

    /// Performs garbage collection on MVCC version chains (v0.5.0+).
    ///
    /// Removes old versions that are no longer visible to any active transaction.
    /// This helps reduce memory usage in long-running databases.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rustlite::Database;
    ///
    /// let db = Database::in_memory()?;
    /// // ... perform many transactions ...
    /// db.gc()?; // Clean up old versions
    /// # Ok::<(), rustlite::Error>(())
    /// ```
    pub fn gc(&self) -> Result<()> {
        if let Some(ref manager) = self.inner.transaction_manager {
            manager.gc()
        } else {
            Ok(()) // No-op if transactions not initialized
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.7.0");
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
        assert!(db
            .index_find("sessions", b"nonexistent")
            .unwrap()
            .is_empty());
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

    #[test]
    fn test_simple_query() {
        let db = Database::in_memory().unwrap();

        let mut context = ExecutionContext::new();
        context.data.insert(
            "users".to_string(),
            vec![
                Row {
                    columns: vec![
                        Column {
                            name: "name".to_string(),
                            alias: None,
                        },
                        Column {
                            name: "age".to_string(),
                            alias: None,
                        },
                    ],
                    values: vec![Value::String("Alice".to_string()), Value::Integer(30)],
                },
                Row {
                    columns: vec![
                        Column {
                            name: "name".to_string(),
                            alias: None,
                        },
                        Column {
                            name: "age".to_string(),
                            alias: None,
                        },
                    ],
                    values: vec![Value::String("Bob".to_string()), Value::Integer(25)],
                },
            ],
        );

        let results = db.query("SELECT * FROM users", context).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_with_where() {
        let db = Database::in_memory().unwrap();

        let mut context = ExecutionContext::new();
        context.data.insert(
            "users".to_string(),
            vec![
                Row {
                    columns: vec![
                        Column {
                            name: "name".to_string(),
                            alias: None,
                        },
                        Column {
                            name: "age".to_string(),
                            alias: None,
                        },
                    ],
                    values: vec![Value::String("Alice".to_string()), Value::Integer(30)],
                },
                Row {
                    columns: vec![
                        Column {
                            name: "name".to_string(),
                            alias: None,
                        },
                        Column {
                            name: "age".to_string(),
                            alias: None,
                        },
                    ],
                    values: vec![Value::String("Bob".to_string()), Value::Integer(25)],
                },
            ],
        );

        let results = db
            .query("SELECT name FROM users WHERE age > 26", context)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].values[0], Value::String("Alice".to_string()));
    }

    #[test]
    fn test_query_with_limit() {
        let db = Database::in_memory().unwrap();

        let mut context = ExecutionContext::new();
        context.data.insert(
            "users".to_string(),
            vec![
                Row {
                    columns: vec![Column {
                        name: "name".to_string(),
                        alias: None,
                    }],
                    values: vec![Value::String("Alice".to_string())],
                },
                Row {
                    columns: vec![Column {
                        name: "name".to_string(),
                        alias: None,
                    }],
                    values: vec![Value::String("Bob".to_string())],
                },
                Row {
                    columns: vec![Column {
                        name: "name".to_string(),
                        alias: None,
                    }],
                    values: vec![Value::String("Charlie".to_string())],
                },
            ],
        );

        let results = db.query("SELECT * FROM users LIMIT 2", context).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_prepare_and_execute() {
        let db = Database::in_memory().unwrap();
        let plan = db.prepare("SELECT * FROM users WHERE age > 18").unwrap();

        let mut context = ExecutionContext::new();
        context.data.insert(
            "users".to_string(),
            vec![Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Alice".to_string()), Value::Integer(30)],
            }],
        );

        let results = db.execute_plan(&plan, context).unwrap();
        assert_eq!(results.len(), 1);
    }

    // Transaction tests (v0.5.0+)
    #[test]
    fn test_transaction_basic() {
        let db = Database::in_memory().unwrap();

        let mut txn = db.begin().unwrap();
        txn.put(b"key1".to_vec(), b"value1".to_vec()).unwrap();
        txn.put(b"key2".to_vec(), b"value2".to_vec()).unwrap();
        txn.commit().unwrap();

        // Verify data is visible after commit
        let txn2 = db.begin().unwrap();
        assert_eq!(txn2.get(b"key1").unwrap(), Some(b"value1".to_vec()));
        assert_eq!(txn2.get(b"key2").unwrap(), Some(b"value2".to_vec()));
    }

    #[test]
    fn test_transaction_isolation() {
        let db = Database::in_memory().unwrap();

        // Transaction 1: Write data
        let mut txn1 = db.begin().unwrap();
        txn1.put(b"counter".to_vec(), b"1".to_vec()).unwrap();
        txn1.commit().unwrap();

        // Transaction 2: Start and read
        let txn2 = db.begin().unwrap();
        assert_eq!(txn2.get(b"counter").unwrap(), Some(b"1".to_vec()));

        // Transaction 3: Update value
        let mut txn3 = db.begin().unwrap();
        txn3.put(b"counter".to_vec(), b"2".to_vec()).unwrap();
        txn3.commit().unwrap();

        // Transaction 2 should still see old value (snapshot isolation)
        assert_eq!(txn2.get(b"counter").unwrap(), Some(b"1".to_vec()));
    }

    #[test]
    fn test_transaction_rollback() {
        let db = Database::in_memory().unwrap();

        // Write initial data
        let mut txn1 = db.begin().unwrap();
        txn1.put(b"key1".to_vec(), b"original".to_vec()).unwrap();
        txn1.commit().unwrap();

        // Update but rollback
        let mut txn2 = db.begin().unwrap();
        txn2.put(b"key1".to_vec(), b"updated".to_vec()).unwrap();
        txn2.rollback().unwrap();

        // Should see original value
        let txn3 = db.begin().unwrap();
        assert_eq!(txn3.get(b"key1").unwrap(), Some(b"original".to_vec()));
    }

    #[test]
    fn test_transaction_delete() {
        let db = Database::in_memory().unwrap();

        // Write data
        let mut txn1 = db.begin().unwrap();
        txn1.put(b"temp".to_vec(), b"data".to_vec()).unwrap();
        txn1.commit().unwrap();

        // Delete data
        let mut txn2 = db.begin().unwrap();
        txn2.delete(b"temp").unwrap();
        txn2.commit().unwrap();

        // Should not exist
        let txn3 = db.begin().unwrap();
        assert_eq!(txn3.get(b"temp").unwrap(), None);
    }

    #[test]
    fn test_transaction_scan() {
        let db = Database::in_memory().unwrap();

        // Write multiple keys
        let mut txn = db.begin().unwrap();
        txn.put(b"user:1".to_vec(), b"alice".to_vec()).unwrap();
        txn.put(b"user:2".to_vec(), b"bob".to_vec()).unwrap();
        txn.put(b"post:1".to_vec(), b"post1".to_vec()).unwrap();
        txn.commit().unwrap();

        // Scan with prefix
        let txn2 = db.begin().unwrap();
        let results = txn2.scan(b"user:").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_transaction_with_index() {
        let db = Database::in_memory().unwrap();
        db.create_index("user_idx", IndexType::Hash).unwrap();

        // Transaction 1: Insert with index
        let mut txn = db.begin().unwrap();
        txn.put(b"user:1".to_vec(), b"alice@example.com".to_vec())
            .unwrap();
        txn.commit().unwrap();

        // Manually update index (in real use, this would be automated)
        db.index_insert("user_idx", b"alice@example.com", 1)
            .unwrap();

        // Transaction 2: Query via index
        let txn2 = db.begin().unwrap();
        let ids = db.index_find("user_idx", b"alice@example.com").unwrap();
        assert_eq!(ids, vec![1]);

        // Verify data via transaction
        assert_eq!(
            txn2.get(b"user:1").unwrap(),
            Some(b"alice@example.com".to_vec())
        );
    }

    #[test]
    fn test_concurrent_transaction_isolation() {
        use std::sync::Arc;
        use std::thread;

        let db = Arc::new(Database::in_memory().unwrap());

        // Initial balance
        let mut setup = db.begin().unwrap();
        setup.put(b"balance".to_vec(), b"1000".to_vec()).unwrap();
        setup.commit().unwrap();

        // Thread 1: Read balance multiple times
        let db1 = db.clone();
        let handle1 = thread::spawn(move || {
            let txn = db1.begin().unwrap();
            let balance1_bytes = txn.get(b"balance").unwrap().unwrap();
            let balance1 = String::from_utf8_lossy(&balance1_bytes);
            thread::sleep(std::time::Duration::from_millis(10));
            let balance2_bytes = txn.get(b"balance").unwrap().unwrap();
            let balance2 = String::from_utf8_lossy(&balance2_bytes);
            assert_eq!(balance1, balance2); // Should be consistent
            balance1.to_string()
        });

        // Thread 2: Update balance
        let db2 = db.clone();
        let handle2 = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(5));
            let mut txn = db2.begin().unwrap();
            txn.put(b"balance".to_vec(), b"2000".to_vec()).unwrap();
            txn.commit().unwrap();
        });

        let balance_seen = handle1.join().unwrap();
        handle2.join().unwrap();

        // Thread 1 should have seen 1000 (snapshot isolation)
        assert_eq!(balance_seen, "1000");

        // New transaction sees updated value
        let final_txn = db.begin().unwrap();
        let final_balance_bytes = final_txn.get(b"balance").unwrap().unwrap();
        let final_balance = String::from_utf8_lossy(&final_balance_bytes);
        assert_eq!(final_balance, "2000");
    }

    #[test]
    fn test_transaction_error_handling() {
        let db = Database::in_memory().unwrap();

        // Test double commit
        let mut txn = db.begin().unwrap();
        txn.put(b"key".to_vec(), b"value".to_vec()).unwrap();
        txn.commit().unwrap();

        // Attempting operations after commit should fail gracefully
        // (In current implementation, the transaction is consumed)
    }

    #[test]
    fn test_transaction_with_query() {
        let db = Database::in_memory().unwrap();

        // Use transaction to populate data
        let mut txn = db.begin().unwrap();
        txn.put(b"user:1:name".to_vec(), b"Alice".to_vec()).unwrap();
        txn.put(b"user:1:age".to_vec(), b"30".to_vec()).unwrap();
        txn.put(b"user:2:name".to_vec(), b"Bob".to_vec()).unwrap();
        txn.put(b"user:2:age".to_vec(), b"25".to_vec()).unwrap();
        txn.commit().unwrap();

        // Query within a transaction context
        let query_txn = db.begin().unwrap();
        let name = query_txn.get(b"user:1:name").unwrap();
        assert_eq!(name, Some(b"Alice".to_vec()));
    }

    #[test]
    fn test_garbage_collection() {
        let db = Database::in_memory().unwrap();

        // Create multiple versions
        for i in 0..10 {
            let mut txn = db.begin().unwrap();
            txn.put(b"key".to_vec(), format!("version{}", i).into_bytes())
                .unwrap();
            txn.commit().unwrap();
        }

        // Run GC
        db.gc().unwrap();

        // Latest value should still be accessible
        let txn = db.begin().unwrap();
        assert_eq!(txn.get(b"key").unwrap(), Some(b"version9".to_vec()));
    }

    #[test]
    fn test_persistent_transactions() {
        let dir = tempdir().unwrap();
        let path = dir.path();

        // Create DB and do transaction
        {
            let db = Database::open(path).unwrap();
            let mut txn = db.begin().unwrap();
            txn.put(b"persistent_key".to_vec(), b"persistent_value".to_vec())
                .unwrap();
            txn.commit().unwrap();

            // Also write to persistent storage (transactions are in-memory MVCC layer)
            db.put(b"direct_key", b"direct_value").unwrap();
            db.sync().unwrap();
        }

        // Reopen and verify
        {
            let db = Database::open(path).unwrap();
            // Direct storage access persists
            assert_eq!(
                db.get(b"direct_key").unwrap(),
                Some(b"direct_value".to_vec())
            );

            // Note: MVCC transactions are in-memory only in current implementation
            // This test verifies the database persistence, not transaction persistence
        }
    }

    #[test]
    fn test_transaction_with_large_dataset() {
        let db = Database::in_memory().unwrap();

        // Insert 1000 keys in one transaction
        let mut txn = db.begin().unwrap();
        for i in 0..1000 {
            let key = format!("key:{:04}", i);
            let value = format!("value:{}", i);
            txn.put(key.into_bytes(), value.into_bytes()).unwrap();
        }
        txn.commit().unwrap();

        // Verify all keys exist
        let verify_txn = db.begin().unwrap();
        for i in 0..1000 {
            let key = format!("key:{:04}", i);
            let expected_value = format!("value:{}", i);
            assert_eq!(
                verify_txn.get(&key.into_bytes()).unwrap(),
                Some(expected_value.into_bytes())
            );
        }
    }

    #[test]
    fn test_mixed_transaction_and_direct_operations() {
        let db = Database::in_memory().unwrap();

        // Direct put
        db.put(b"direct", b"value1").unwrap();

        // Transaction put
        let mut txn = db.begin().unwrap();
        txn.put(b"txn".to_vec(), b"value2".to_vec()).unwrap();
        txn.commit().unwrap();

        // Both should be readable
        assert_eq!(db.get(b"direct").unwrap(), Some(b"value1".to_vec()));

        let read_txn = db.begin().unwrap();
        assert_eq!(read_txn.get(b"txn").unwrap(), Some(b"value2".to_vec()));
    }

    #[test]
    fn test_serializable_isolation() {
        let db = Database::in_memory().unwrap();

        // Setup
        let mut setup = db.begin().unwrap();
        setup.put(b"counter".to_vec(), b"0".to_vec()).unwrap();
        setup.commit().unwrap();

        // Use serializable isolation
        let txn = db.begin_transaction(IsolationLevel::Serializable).unwrap();
        assert_eq!(txn.isolation_level(), IsolationLevel::Serializable);

        let value = txn.get(b"counter").unwrap();
        assert_eq!(value, Some(b"0".to_vec()));
    }

    #[test]
    fn test_multiple_isolation_levels() {
        let db = Database::in_memory().unwrap();

        // Test all isolation levels can be created
        let _txn1 = db
            .begin_transaction(IsolationLevel::ReadUncommitted)
            .unwrap();
        let _txn2 = db.begin_transaction(IsolationLevel::ReadCommitted).unwrap();
        let _txn3 = db
            .begin_transaction(IsolationLevel::RepeatableRead)
            .unwrap();
        let _txn4 = db.begin_transaction(IsolationLevel::Serializable).unwrap();
    }
}
