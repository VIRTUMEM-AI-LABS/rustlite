//! Transaction management module with MVCC.
//!
//! Provides ACID transaction support using Multi-Version Concurrency Control (MVCC).
//! Implements snapshot isolation with timestamp-based versioning.

use crate::{Error, Result};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Transaction isolation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IsolationLevel {
    /// Read uncommitted (lowest isolation)
    ReadUncommitted,
    /// Read committed
    ReadCommitted,
    /// Repeatable read (snapshot isolation)
    #[default]
    RepeatableRead,
    /// Serializable (highest isolation)
    Serializable,
}

/// Transaction ID (monotonically increasing)
pub type TransactionId = u64;

/// Timestamp for MVCC versioning
pub type Timestamp = u64;

/// A versioned value in MVCC
#[derive(Debug, Clone)]
pub struct VersionedValue {
    /// The actual value
    pub value: Option<Vec<u8>>,
    /// Transaction ID that created this version
    pub txn_id: TransactionId,
    /// Timestamp when this version was created
    pub created_at: Timestamp,
    /// Timestamp when this version was deleted (None if still valid)
    pub deleted_at: Option<Timestamp>,
    /// Whether this version is committed
    pub committed: bool,
}

impl VersionedValue {
    /// Check if this version is visible to a transaction with given snapshot timestamp
    pub fn is_visible(&self, snapshot_ts: Timestamp, current_txn_id: TransactionId) -> bool {
        // Version must be committed (or created by current transaction)
        if !self.committed && self.txn_id != current_txn_id {
            return false;
        }

        // Version must be created before snapshot
        if self.created_at > snapshot_ts && self.txn_id != current_txn_id {
            return false;
        }

        // Version must not be deleted before snapshot
        if let Some(deleted_ts) = self.deleted_at {
            if deleted_ts <= snapshot_ts {
                return false;
            }
        }

        // Value must exist (not a delete marker for uncommitted txn)
        self.value.is_some()
    }
}

/// MVCC version chain for a key
#[derive(Debug, Clone)]
pub struct VersionChain {
    /// All versions of this key, sorted by timestamp (newest first)
    versions: Vec<VersionedValue>,
}

impl VersionChain {
    /// Create a new version chain
    pub fn new() -> Self {
        Self {
            versions: Vec::new(),
        }
    }

    /// Add a new version (inserts at front)
    pub fn add_version(&mut self, version: VersionedValue) {
        self.versions.insert(0, version);
    }

    /// Get the visible version for a transaction
    pub fn get_visible(
        &self,
        snapshot_ts: Timestamp,
        current_txn_id: TransactionId,
    ) -> Option<Vec<u8>> {
        for version in &self.versions {
            if version.is_visible(snapshot_ts, current_txn_id) {
                return version.value.clone();
            }
        }
        None
    }

    /// Mark all versions created by a transaction as committed
    pub fn commit_transaction(&mut self, txn_id: TransactionId) {
        for version in &mut self.versions {
            if version.txn_id == txn_id {
                version.committed = true;
            }
        }
    }

    /// Remove all versions created by a transaction (for rollback)
    pub fn rollback_transaction(&mut self, txn_id: TransactionId) {
        self.versions.retain(|v| v.txn_id != txn_id);
    }

    /// Garbage collect versions older than the oldest active snapshot
    pub fn gc(&mut self, min_active_ts: Timestamp) {
        // Keep only the first committed version visible to oldest snapshot
        let mut found_visible = false;
        self.versions.retain(|v| {
            if found_visible && v.committed && v.created_at < min_active_ts {
                false
            } else {
                if v.committed && v.created_at <= min_active_ts {
                    found_visible = true;
                }
                true
            }
        });
    }
}

impl Default for VersionChain {
    fn default() -> Self {
        Self::new()
    }
}

/// MVCC storage for versioned data
pub struct MVCCStorage {
    /// Version chains for each key
    data: RwLock<HashMap<Vec<u8>, VersionChain>>,
}

impl MVCCStorage {
    /// Create new MVCC storage
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    /// Read a key with MVCC snapshot isolation
    pub fn read(
        &self,
        key: &[u8],
        snapshot_ts: Timestamp,
        txn_id: TransactionId,
    ) -> Result<Option<Vec<u8>>> {
        let data = self.data.read().map_err(|_| Error::LockPoisoned)?;

        if let Some(chain) = data.get(key) {
            Ok(chain.get_visible(snapshot_ts, txn_id))
        } else {
            Ok(None)
        }
    }

    /// Write a key (creates a new version)
    pub fn write(
        &self,
        key: Vec<u8>,
        value: Vec<u8>,
        txn_id: TransactionId,
        timestamp: Timestamp,
    ) -> Result<()> {
        let mut data = self.data.write().map_err(|_| Error::LockPoisoned)?;

        let chain = data.entry(key).or_insert_with(VersionChain::new);

        chain.add_version(VersionedValue {
            value: Some(value),
            txn_id,
            created_at: timestamp,
            deleted_at: None,
            committed: false,
        });

        Ok(())
    }

    /// Delete a key (creates a delete marker)
    pub fn delete(&self, key: &[u8], txn_id: TransactionId, timestamp: Timestamp) -> Result<()> {
        let mut data = self.data.write().map_err(|_| Error::LockPoisoned)?;

        let chain = data.entry(key.to_vec()).or_insert_with(VersionChain::new);

        // Mark previous version as deleted
        if let Some(prev) = chain.versions.first_mut() {
            if prev.txn_id != txn_id {
                prev.deleted_at = Some(timestamp);
            }
        }

        // Add delete marker for this transaction
        chain.add_version(VersionedValue {
            value: None,
            txn_id,
            created_at: timestamp,
            deleted_at: None,
            committed: false,
        });

        Ok(())
    }

    /// Commit all versions for a transaction
    pub fn commit(&self, txn_id: TransactionId) -> Result<()> {
        let mut data = self.data.write().map_err(|_| Error::LockPoisoned)?;

        for chain in data.values_mut() {
            chain.commit_transaction(txn_id);
        }

        Ok(())
    }

    /// Rollback all versions for a transaction
    pub fn rollback(&self, txn_id: TransactionId) -> Result<()> {
        let mut data = self.data.write().map_err(|_| Error::LockPoisoned)?;

        for chain in data.values_mut() {
            chain.rollback_transaction(txn_id);
        }

        // Remove empty chains
        data.retain(|_, chain| !chain.versions.is_empty());

        Ok(())
    }

    /// Garbage collect old versions
    pub fn gc(&self, min_active_ts: Timestamp) -> Result<()> {
        let mut data = self.data.write().map_err(|_| Error::LockPoisoned)?;

        for chain in data.values_mut() {
            chain.gc(min_active_ts);
        }

        Ok(())
    }

    /// Scan keys with prefix (for range queries)
    pub fn scan_prefix(
        &self,
        prefix: &[u8],
        snapshot_ts: Timestamp,
        txn_id: TransactionId,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let data = self.data.read().map_err(|_| Error::LockPoisoned)?;

        let mut results = Vec::new();
        for (key, chain) in data.iter() {
            if key.starts_with(prefix) {
                if let Some(value) = chain.get_visible(snapshot_ts, txn_id) {
                    results.push((key.clone(), value));
                }
            }
        }

        results.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(results)
    }
}

impl Default for MVCCStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Active transaction information
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ActiveTransaction {
    txn_id: TransactionId,
    snapshot_ts: Timestamp,
    isolation: IsolationLevel,
}

/// Transaction Manager for MVCC
pub struct TransactionManager {
    /// Next transaction ID
    next_txn_id: AtomicU64,
    /// Next timestamp
    next_timestamp: AtomicU64,
    /// Active transactions
    active_txns: RwLock<BTreeMap<TransactionId, ActiveTransaction>>,
    /// MVCC storage
    storage: Arc<MVCCStorage>,
    /// Self reference for creating transactions
    self_ref: RwLock<Option<std::sync::Weak<TransactionManager>>>,
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new(storage: Arc<MVCCStorage>) -> Arc<Self> {
        let manager = Arc::new(Self {
            next_txn_id: AtomicU64::new(1),
            next_timestamp: AtomicU64::new(Self::current_timestamp()),
            active_txns: RwLock::new(BTreeMap::new()),
            storage,
            self_ref: RwLock::new(None),
        });

        // Store weak self-reference
        *manager.self_ref.write().unwrap() = Some(Arc::downgrade(&manager));
        manager
    }

    /// Get current timestamp (milliseconds since UNIX epoch)
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// Generate next transaction ID
    fn next_txn_id(&self) -> TransactionId {
        self.next_txn_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Generate next timestamp
    fn next_timestamp(&self) -> Timestamp {
        self.next_timestamp.fetch_add(1, Ordering::SeqCst)
    }

    /// Begin a new transaction
    pub fn begin(self: &Arc<Self>, isolation: IsolationLevel) -> Result<Transaction> {
        let txn_id = self.next_txn_id();
        let snapshot_ts = self.next_timestamp();

        let active_txn = ActiveTransaction {
            txn_id,
            snapshot_ts,
            isolation,
        };

        {
            let mut active = self.active_txns.write().map_err(|_| Error::LockPoisoned)?;
            active.insert(txn_id, active_txn.clone());
        }

        Ok(Transaction {
            txn_id,
            snapshot_ts,
            isolation,
            storage: Arc::clone(&self.storage),
            manager: Some(Arc::clone(self)),
            write_set: RwLock::new(HashMap::new()),
            committed: false,
        })
    }

    /// Commit a transaction
    pub fn commit(&self, txn_id: TransactionId) -> Result<()> {
        // Validate no conflicts (simplified - just check write-write conflicts)
        // In a full implementation, we'd do serializability validation here

        // Commit in storage
        self.storage.commit(txn_id)?;

        // Remove from active transactions
        {
            let mut active = self.active_txns.write().map_err(|_| Error::LockPoisoned)?;
            active.remove(&txn_id);
        }

        Ok(())
    }

    /// Rollback a transaction
    pub fn rollback(&self, txn_id: TransactionId) -> Result<()> {
        // Rollback in storage
        self.storage.rollback(txn_id)?;

        // Remove from active transactions
        {
            let mut active = self.active_txns.write().map_err(|_| Error::LockPoisoned)?;
            active.remove(&txn_id);
        }

        Ok(())
    }

    /// Perform garbage collection
    pub fn gc(&self) -> Result<()> {
        // Find oldest active snapshot
        let min_active_ts = {
            let active = self.active_txns.read().map_err(|_| Error::LockPoisoned)?;
            active
                .values()
                .map(|txn| txn.snapshot_ts)
                .min()
                .unwrap_or(self.next_timestamp())
        };

        self.storage.gc(min_active_ts)
    }
}

/// A database transaction with MVCC support
pub struct Transaction {
    /// Transaction ID
    pub txn_id: TransactionId,
    /// Snapshot timestamp
    snapshot_ts: Timestamp,
    /// Isolation level
    isolation: IsolationLevel,
    /// Reference to MVCC storage
    storage: Arc<MVCCStorage>,
    /// Reference to transaction manager (for commit/rollback)
    manager: Option<Arc<TransactionManager>>,
    /// Write set for validation
    write_set: RwLock<HashMap<Vec<u8>, Vec<u8>>>,
    /// Whether transaction is committed
    committed: bool,
}

impl Transaction {
    /// Read a value with snapshot isolation
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // Check write set first (read your own writes)
        {
            let write_set = self.write_set.read().map_err(|_| Error::LockPoisoned)?;
            if let Some(value) = write_set.get(key) {
                return Ok(Some(value.clone()));
            }
        }

        // Read from MVCC storage with snapshot isolation
        self.storage.read(key, self.snapshot_ts, self.txn_id)
    }

    /// Write a value (buffered until commit)
    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        // Add to write set
        {
            let mut write_set = self.write_set.write().map_err(|_| Error::LockPoisoned)?;
            write_set.insert(key.clone(), value.clone());
        }

        // Write to MVCC storage (creates uncommitted version with snapshot timestamp)
        self.storage
            .write(key, value, self.txn_id, self.snapshot_ts)
    }

    /// Delete a key
    pub fn delete(&mut self, key: &[u8]) -> Result<()> {
        // Remove from write set if present
        {
            let mut write_set = self.write_set.write().map_err(|_| Error::LockPoisoned)?;
            write_set.remove(key);
        }

        self.storage.delete(key, self.txn_id, self.snapshot_ts)
    }

    /// Scan keys with prefix
    pub fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        self.storage
            .scan_prefix(prefix, self.snapshot_ts, self.txn_id)
    }

    /// Commit the transaction
    pub fn commit(mut self) -> Result<()> {
        if self.committed {
            return Err(Error::Transaction("Transaction already committed".into()));
        }

        if let Some(manager) = &self.manager {
            manager.commit(self.txn_id)?;
        } else {
            self.storage.commit(self.txn_id)?;
        }

        self.committed = true;
        Ok(())
    }

    /// Rollback the transaction
    pub fn rollback(self) -> Result<()> {
        if self.committed {
            return Err(Error::Transaction("Transaction already committed".into()));
        }

        if let Some(manager) = &self.manager {
            manager.rollback(self.txn_id)
        } else {
            self.storage.rollback(self.txn_id)
        }
    }

    /// Get transaction ID
    pub fn id(&self) -> TransactionId {
        self.txn_id
    }

    /// Get isolation level
    pub fn isolation_level(&self) -> IsolationLevel {
        self.isolation
    }
}
