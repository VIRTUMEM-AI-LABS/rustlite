//! # RustLite Storage Engine
//!
//! LSM-tree based persistent storage engine for RustLite.
//!
//! ## ⚠️ Internal Implementation Detail
//!
//! **This crate is an internal implementation detail of RustLite.**
//!
//! Users should depend on the main [`rustlite`](https://crates.io/crates/rustlite) crate
//! instead, which provides the stable public API. This crate's API may change
//! without notice between minor versions.
//!
//! ```toml
//! # In your Cargo.toml - use the main crate, not this one:
//! [dependencies]
//! rustlite = "0.3"
//! ```
//!
//! ---
//!
//! This crate provides the storage engine for RustLite, implementing an
//! LSM-tree (Log-Structured Merge-tree) architecture with:
//!
//! - **Memtable**: In-memory write buffer using BTreeMap for sorted order
//! - **SSTable**: Immutable on-disk sorted string tables
//! - **Compaction**: Background merging to reduce read amplification
//! - **Manifest**: Metadata tracking for crash recovery
//!
//! ## Architecture
//!
//! ```text
//! Writes → Memtable (memory) → SSTable (disk)
//!              ↓                    ↓
//!         Flush when full    Compact to lower levels
//! ```

use rustlite_core::{Error, Result};
use rustlite_wal::{RecordPayload, SyncMode, WalConfig, WalManager, WalRecord};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

pub mod compaction;
pub mod manifest;
pub mod memtable;
pub mod sstable;

pub use compaction::{CompactionConfig, CompactionStats, CompactionWorker};
pub use manifest::{Manifest, ManifestSSTable};
pub use memtable::{Memtable, MemtableEntry};
pub use sstable::{SSTableEntry, SSTableMeta, SSTableReader, SSTableWriter};

/// Default memtable flush threshold (4MB)
const DEFAULT_MEMTABLE_SIZE: u64 = 4 * 1024 * 1024;

/// Storage engine configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Maximum memtable size before flushing
    pub memtable_size: u64,
    /// Sync mode for WAL
    pub sync_mode: SyncMode,
    /// Compaction configuration
    pub compaction: CompactionConfig,
    /// Enable background compaction
    pub enable_compaction: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            memtable_size: DEFAULT_MEMTABLE_SIZE,
            sync_mode: SyncMode::Sync,
            compaction: CompactionConfig::default(),
            enable_compaction: true,
        }
    }
}

/// Storage engine manager
///
/// Provides a persistent key-value storage using LSM-tree architecture.
pub struct StorageEngine {
    /// Database directory
    dir: PathBuf,
    /// Configuration
    config: StorageConfig,
    /// Active memtable
    memtable: Arc<RwLock<Memtable>>,
    /// Immutable memtables being flushed
    immutable_memtables: Arc<Mutex<Vec<Arc<Memtable>>>>,
    /// Write-ahead log
    wal: Arc<Mutex<WalManager>>,
    /// Manifest
    manifest: Arc<Mutex<Manifest>>,
    /// Compaction worker
    compactor: Arc<Mutex<CompactionWorker>>,
    /// Current sequence number
    sequence: Arc<RwLock<u64>>,
}

impl StorageEngine {
    /// Open or create a storage engine at the given path
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_config(path, StorageConfig::default())
    }

    /// Open or create a storage engine with custom configuration
    pub fn open_with_config(path: impl AsRef<Path>, config: StorageConfig) -> Result<Self> {
        let dir = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;

        // Create subdirectories
        std::fs::create_dir_all(dir.join("wal"))?;
        std::fs::create_dir_all(dir.join("sst"))?;

        // Open WAL
        let wal_config = WalConfig {
            wal_dir: dir.join("wal"),
            sync_mode: config.sync_mode,
            ..Default::default()
        };
        let mut wal = WalManager::new(wal_config)?;
        wal.open()?;

        // Open manifest
        let manifest = Manifest::open(&dir)?;
        let sequence = manifest.sequence();

        // Create compactor
        let compactor = CompactionWorker::new(&dir, config.compaction.clone());

        // Create memtable
        let memtable = Memtable::with_sequence(sequence);

        let engine = Self {
            dir,
            config,
            memtable: Arc::new(RwLock::new(memtable)),
            immutable_memtables: Arc::new(Mutex::new(Vec::new())),
            wal: Arc::new(Mutex::new(wal)),
            manifest: Arc::new(Mutex::new(manifest)),
            compactor: Arc::new(Mutex::new(compactor)),
            sequence: Arc::new(RwLock::new(sequence)),
        };

        // Recover from WAL
        engine.recover()?;

        Ok(engine)
    }

    /// Recover from WAL after crash
    fn recover(&self) -> Result<()> {
        let wal = self.wal.lock().map_err(|_| Error::LockPoisoned)?;
        let records = wal.recover()?;

        let mut memtable = self.memtable.write().map_err(|_| Error::LockPoisoned)?;

        for record in records {
            match &record.payload {
                RecordPayload::Put { key, value } => {
                    memtable.put(key.clone(), value.clone());
                }
                RecordPayload::Delete { key } => {
                    memtable.delete(key.clone());
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Insert or update a key-value pair
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        // Get next sequence number
        let _seq = {
            let mut sequence = self.sequence.write().map_err(|_| Error::LockPoisoned)?;
            *sequence += 1;
            *sequence
        };

        // Write to WAL first
        {
            let mut wal = self.wal.lock().map_err(|_| Error::LockPoisoned)?;
            let record = WalRecord::put(key.to_vec(), value.to_vec());
            wal.append(record)?;
        }

        // Write to memtable
        {
            let mut memtable = self.memtable.write().map_err(|_| Error::LockPoisoned)?;
            memtable.put(key.to_vec(), value.to_vec());
        }

        // Check if flush is needed
        self.maybe_flush()?;

        Ok(())
    }

    /// Retrieve a value by key
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // Check active memtable first
        {
            let memtable = self.memtable.read().map_err(|_| Error::LockPoisoned)?;
            if let Some(result) = memtable.get(key) {
                return match result {
                    Some(value) => Ok(Some(value.to_vec())),
                    None => Ok(None), // Tombstone
                };
            }
        }

        // Check immutable memtables (newest first)
        {
            let immutable = self
                .immutable_memtables
                .lock()
                .map_err(|_| Error::LockPoisoned)?;
            for mt in immutable.iter().rev() {
                if let Some(result) = mt.get(key) {
                    return match result {
                        Some(value) => Ok(Some(value.to_vec())),
                        None => Ok(None), // Tombstone
                    };
                }
            }
        }

        // Check SSTables (newest first, level 0 first)
        {
            let manifest = self.manifest.lock().map_err(|_| Error::LockPoisoned)?;

            // Check each level
            for level in 0..7 {
                let sstables = manifest.sstables_at_level(level);

                // Sort by sequence (newest first)
                let mut sorted: Vec<_> = sstables.iter().collect();
                sorted.sort_by(|a, b| b.sequence.cmp(&a.sequence));

                for sst in sorted {
                    // Quick range check
                    if key < sst.min_key.as_slice() || key > sst.max_key.as_slice() {
                        continue;
                    }

                    // Open and search SSTable
                    let path = PathBuf::from(&sst.path);
                    if let Ok(mut reader) = SSTableReader::open(&path) {
                        if let Ok(Some(entry)) = reader.get(key) {
                            if entry.is_tombstone() {
                                return Ok(None);
                            }
                            return Ok(Some(entry.value));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Delete a key
    pub fn delete(&self, key: &[u8]) -> Result<()> {
        // Get next sequence number
        let _seq = {
            let mut sequence = self.sequence.write().map_err(|_| Error::LockPoisoned)?;
            *sequence += 1;
            *sequence
        };

        // Write to WAL first
        {
            let mut wal = self.wal.lock().map_err(|_| Error::LockPoisoned)?;
            let record = WalRecord::delete(key.to_vec());
            wal.append(record)?;
        }

        // Write tombstone to memtable
        {
            let mut memtable = self.memtable.write().map_err(|_| Error::LockPoisoned)?;
            memtable.delete(key.to_vec());
        }

        Ok(())
    }

    /// Check if memtable needs flushing and trigger if so
    fn maybe_flush(&self) -> Result<()> {
        let should_flush = {
            let memtable = self.memtable.read().map_err(|_| Error::LockPoisoned)?;
            memtable.size_bytes() >= self.config.memtable_size
        };

        if should_flush {
            self.flush()?;
        }

        Ok(())
    }

    /// Flush the current memtable to disk as an SSTable
    pub fn flush(&self) -> Result<()> {
        // Swap memtable
        let old_memtable = {
            let mut memtable = self.memtable.write().map_err(|_| Error::LockPoisoned)?;
            let sequence = memtable.sequence();
            let old = std::mem::replace(&mut *memtable, Memtable::with_sequence(sequence));
            Arc::new(old)
        };

        if old_memtable.is_empty() {
            return Ok(());
        }

        // Add to immutable list
        {
            let mut immutable = self
                .immutable_memtables
                .lock()
                .map_err(|_| Error::LockPoisoned)?;
            immutable.push(Arc::clone(&old_memtable));
        }

        // Generate SSTable path
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let sst_path = self.dir.join("sst").join(format!("L0_{}.sst", timestamp));

        // Create a cloned memtable for iteration
        let mt_for_iter = {
            let entries: Vec<_> = old_memtable
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            entries
        };

        // Write SSTable
        let meta = SSTableWriter::from_memtable(&sst_path, mt_for_iter.into_iter())?;

        // Update manifest
        {
            let mut manifest = self.manifest.lock().map_err(|_| Error::LockPoisoned)?;
            manifest.add_sstable(&meta)?;
            manifest.update_sequence(old_memtable.sequence())?;
        }

        // Remove from immutable list
        {
            let mut immutable = self
                .immutable_memtables
                .lock()
                .map_err(|_| Error::LockPoisoned)?;
            immutable.retain(|m| !Arc::ptr_eq(m, &old_memtable));
        }

        // Maybe trigger compaction
        if self.config.enable_compaction {
            self.maybe_compact()?;
        }

        Ok(())
    }

    /// Check if compaction is needed and run if so
    fn maybe_compact(&self) -> Result<()> {
        let mut compactor = self.compactor.lock().map_err(|_| Error::LockPoisoned)?;
        let mut manifest = self.manifest.lock().map_err(|_| Error::LockPoisoned)?;

        if compactor.needs_compaction(&manifest) {
            compactor.compact_level0(&mut manifest)?;
        }

        Ok(())
    }

    /// Force sync all data to disk
    pub fn sync(&self) -> Result<()> {
        // Sync WAL
        {
            let mut wal = self.wal.lock().map_err(|_| Error::LockPoisoned)?;
            wal.sync()?;
        }

        // Flush memtable
        self.flush()?;

        // Rewrite manifest
        {
            let mut manifest = self.manifest.lock().map_err(|_| Error::LockPoisoned)?;
            manifest.rewrite()?;
        }

        Ok(())
    }

    /// Get storage statistics
    pub fn stats(&self) -> StorageStats {
        let memtable = self.memtable.read().ok();
        let manifest = self.manifest.lock().ok();
        let compactor = self.compactor.lock().ok();

        let (memtable_size, memtable_entries) = match &memtable {
            Some(m) => (m.size_bytes(), m.len()),
            None => (0, 0),
        };

        StorageStats {
            memtable_size,
            memtable_entries,
            sstable_count: manifest
                .as_ref()
                .map(|m| m.all_sstables().len())
                .unwrap_or(0),
            total_disk_size: manifest.as_ref().map(|m| m.total_size()).unwrap_or(0),
            level_counts: manifest.map(|m| m.level_counts()).unwrap_or_default(),
            compaction_stats: compactor.map(|c| c.stats().clone()).unwrap_or_default(),
        }
    }

    /// Close the storage engine
    pub fn close(self) -> Result<()> {
        // Flush any remaining data
        self.flush()?;
        self.sync()?;
        Ok(())
    }
}

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// Current memtable size in bytes
    pub memtable_size: u64,
    /// Number of entries in memtable
    pub memtable_entries: usize,
    /// Total number of SSTables
    pub sstable_count: usize,
    /// Total disk size of SSTables
    pub total_disk_size: u64,
    /// Number of SSTables at each level
    pub level_counts: Vec<usize>,
    /// Compaction statistics
    pub compaction_stats: CompactionStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_storage_engine_basic() {
        let dir = tempdir().unwrap();
        let engine = StorageEngine::open(dir.path()).unwrap();

        // Put and get
        engine.put(b"key1", b"value1").unwrap();
        engine.put(b"key2", b"value2").unwrap();

        assert_eq!(engine.get(b"key1").unwrap(), Some(b"value1".to_vec()));
        assert_eq!(engine.get(b"key2").unwrap(), Some(b"value2".to_vec()));
        assert_eq!(engine.get(b"key3").unwrap(), None);
    }

    #[test]
    fn test_storage_engine_update() {
        let dir = tempdir().unwrap();
        let engine = StorageEngine::open(dir.path()).unwrap();

        engine.put(b"key", b"value1").unwrap();
        assert_eq!(engine.get(b"key").unwrap(), Some(b"value1".to_vec()));

        engine.put(b"key", b"value2").unwrap();
        assert_eq!(engine.get(b"key").unwrap(), Some(b"value2".to_vec()));
    }

    #[test]
    fn test_storage_engine_delete() {
        let dir = tempdir().unwrap();
        let engine = StorageEngine::open(dir.path()).unwrap();

        engine.put(b"key", b"value").unwrap();
        assert_eq!(engine.get(b"key").unwrap(), Some(b"value".to_vec()));

        engine.delete(b"key").unwrap();
        assert_eq!(engine.get(b"key").unwrap(), None);
    }

    #[test]
    fn test_storage_engine_flush() {
        let dir = tempdir().unwrap();
        let config = StorageConfig {
            memtable_size: 100, // Very small to trigger flush
            enable_compaction: false,
            ..Default::default()
        };
        let engine = StorageEngine::open_with_config(dir.path(), config).unwrap();

        // Write enough to trigger flush
        for i in 0..10 {
            let key = format!("key{:03}", i);
            let value = format!("value{}", i);
            engine.put(key.as_bytes(), value.as_bytes()).unwrap();
        }

        // Force flush
        engine.flush().unwrap();

        // Data should still be accessible
        assert_eq!(engine.get(b"key000").unwrap(), Some(b"value0".to_vec()));

        // Check stats
        let stats = engine.stats();
        assert!(stats.sstable_count > 0 || stats.memtable_entries > 0);
    }

    #[test]
    fn test_storage_engine_recovery() {
        let dir = tempdir().unwrap();

        // Write some data
        {
            let engine = StorageEngine::open(dir.path()).unwrap();
            engine.put(b"persistent", b"data").unwrap();
            // Don't call close - simulate crash
        }

        // Reopen and verify data is recovered
        {
            let engine = StorageEngine::open(dir.path()).unwrap();
            assert_eq!(engine.get(b"persistent").unwrap(), Some(b"data".to_vec()));
        }
    }

    #[test]
    fn test_storage_stats() {
        let dir = tempdir().unwrap();
        let engine = StorageEngine::open(dir.path()).unwrap();

        engine.put(b"key", b"value").unwrap();

        let stats = engine.stats();
        assert!(stats.memtable_size > 0 || stats.memtable_entries > 0);
    }
}
