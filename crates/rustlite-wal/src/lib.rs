//! # RustLite WAL (Write-Ahead Log)
//!
//! Write-Ahead Log implementation for RustLite, providing durable,
//! crash-recoverable transaction logging.
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

// Write-Ahead Log (WAL) implementation for RustLite
// Provides durable, crash-recoverable transaction logging

use rustlite_core::Result;
use serde::{Deserialize, Serialize};

pub mod reader;
pub mod record;
pub mod recovery;
pub mod segment;
pub mod writer;

pub use reader::WalReader;
pub use record::{RecordPayload, RecordType, WalRecord};
pub use recovery::{RecoveryManager, RecoveryStats};
pub use segment::{SegmentInfo, SegmentManager};
pub use writer::WalWriter;

/// WAL configuration options
#[derive(Debug, Clone)]
pub struct WalConfig {
    /// Sync mode: sync, async, or none
    pub sync_mode: SyncMode,
    /// Maximum segment size in bytes before rotation
    pub max_segment_size: u64,
    /// Directory path for WAL segments
    pub wal_dir: std::path::PathBuf,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            sync_mode: SyncMode::Sync,
            max_segment_size: 64 * 1024 * 1024, // 64 MB
            wal_dir: std::path::PathBuf::from("wal"),
        }
    }
}

/// Sync mode for WAL writes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncMode {
    /// Call fsync after every write (strongest durability)
    Sync,
    /// Buffer writes, fsync on segment boundaries (balanced)
    Async,
    /// No fsync (fastest, unsafe for power loss)
    None,
}

/// WAL manager coordinates log writing and recovery
pub struct WalManager {
    config: WalConfig,
    writer: Option<WalWriter>,
}

impl WalManager {
    pub fn new(config: WalConfig) -> Result<Self> {
        Ok(Self {
            config,
            writer: None,
        })
    }

    /// Open the WAL for writing
    ///
    /// This creates or opens the current WAL segment for appending records.
    pub fn open(&mut self) -> Result<()> {
        let writer = WalWriter::new(
            &self.config.wal_dir,
            self.config.max_segment_size,
            self.config.sync_mode,
        )?;
        self.writer = Some(writer);

        Ok(())
    }

    /// Append a record to the WAL
    pub fn append(&mut self, record: WalRecord) -> Result<u64> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| rustlite_core::Error::InvalidOperation("WAL not opened".to_string()))?;
        writer.append(record)
    }

    /// Sync the WAL to disk
    pub fn sync(&mut self) -> Result<()> {
        if let Some(writer) = &mut self.writer {
            writer.sync()
        } else {
            Ok(())
        }
    }

    /// Close the WAL
    pub fn close(&mut self) -> Result<()> {
        if let Some(mut writer) = self.writer.take() {
            writer.sync()?;
        }
        Ok(())
    }

    /// Recover records from the WAL
    ///
    /// This reads all segments and returns committed records for replay.
    /// Incomplete transactions are rolled back (not included).
    pub fn recover(&self) -> Result<Vec<WalRecord>> {
        let recovery = RecoveryManager::new(self.config.clone())?;
        recovery.recover()
    }

    /// Recover records with transaction markers included
    ///
    /// Unlike `recover()`, this includes BEGIN_TX and COMMIT_TX markers.
    pub fn recover_with_markers(&self) -> Result<Vec<WalRecord>> {
        let recovery = RecoveryManager::new(self.config.clone())?;
        recovery.recover_with_markers()
    }

    /// Get statistics about the WAL
    pub fn stats(&self) -> Result<RecoveryStats> {
        let recovery = RecoveryManager::new(self.config.clone())?;
        recovery.get_stats()
    }

    /// Create a reader for the WAL
    pub fn reader(&self) -> Result<WalReader> {
        WalReader::new(&self.config.wal_dir)
    }

    /// Get a segment manager for the WAL
    pub fn segment_manager(&self) -> SegmentManager {
        SegmentManager::new(self.config.wal_dir.clone())
    }

    /// Get the current configuration
    pub fn config(&self) -> &WalConfig {
        &self.config
    }

    /// Check if the WAL is open for writing
    pub fn is_open(&self) -> bool {
        self.writer.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_config() -> (TempDir, WalConfig) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let wal_path = temp_dir.path().join("wal");
        std::fs::create_dir_all(&wal_path).expect("Failed to create WAL dir");

        let config = WalConfig {
            wal_dir: wal_path,
            sync_mode: SyncMode::Sync,
            max_segment_size: 64 * 1024 * 1024,
        };

        (temp_dir, config)
    }

    #[test]
    fn test_wal_config_default() {
        let config = WalConfig::default();
        assert_eq!(config.sync_mode, SyncMode::Sync);
        assert_eq!(config.max_segment_size, 64 * 1024 * 1024);
    }

    #[test]
    fn test_sync_mode() {
        assert_eq!(SyncMode::Sync, SyncMode::Sync);
        assert_ne!(SyncMode::Sync, SyncMode::Async);
    }

    #[test]
    fn test_wal_manager_lifecycle() {
        let (_temp_dir, config) = setup_test_config();

        let mut manager = WalManager::new(config).expect("Failed to create manager");
        assert!(!manager.is_open());

        manager.open().expect("Failed to open");
        assert!(manager.is_open());

        manager.close().expect("Failed to close");
        assert!(!manager.is_open());
    }

    #[test]
    fn test_wal_manager_write_and_recover() {
        let (_temp_dir, config) = setup_test_config();

        // Write some records
        {
            let mut manager = WalManager::new(config.clone()).expect("Failed to create manager");
            manager.open().expect("Failed to open");

            for i in 0..5 {
                let record = WalRecord::put(
                    format!("key{}", i).into_bytes(),
                    format!("value{}", i).into_bytes(),
                );
                manager.append(record).expect("Failed to append");
            }

            manager.sync().expect("Failed to sync");
            manager.close().expect("Failed to close");
        }

        // Recover
        {
            let manager = WalManager::new(config).expect("Failed to create manager");
            let records = manager.recover().expect("Failed to recover");

            assert_eq!(records.len(), 5);
        }
    }

    #[test]
    fn test_wal_manager_stats() {
        let (_temp_dir, config) = setup_test_config();

        // Write some records
        {
            let mut manager = WalManager::new(config.clone()).expect("Failed to create manager");
            manager.open().expect("Failed to open");

            manager.append(WalRecord::begin_tx(1)).expect("Failed");
            manager.append(WalRecord::put(b"k".to_vec(), b"v".to_vec())).expect("Failed");
            manager.append(WalRecord::commit_tx(1)).expect("Failed");

            manager.close().expect("Failed to close");
        }

        let manager = WalManager::new(config).expect("Failed to create manager");
        let stats = manager.stats().expect("Failed to get stats");

        assert_eq!(stats.total_records, 3);
        assert_eq!(stats.transactions_started, 1);
        assert_eq!(stats.transactions_committed, 1);
    }

    #[test]
    fn test_wal_manager_segment_manager() {
        let (_temp_dir, config) = setup_test_config();

        {
            let mut manager = WalManager::new(config.clone()).expect("Failed to create manager");
            manager.open().expect("Failed to open");
            manager.append(WalRecord::put(b"k".to_vec(), b"v".to_vec())).expect("Failed");
            manager.close().expect("Failed to close");
        }

        let manager = WalManager::new(config).expect("Failed to create manager");
        let seg_manager = manager.segment_manager();

        assert_eq!(seg_manager.segment_count().unwrap(), 1);
    }
}
