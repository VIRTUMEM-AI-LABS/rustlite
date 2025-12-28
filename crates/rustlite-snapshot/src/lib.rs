//! # RustLite Snapshot Manager
//!
//! Snapshot and backup functionality for RustLite databases.
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
//! This crate provides point-in-time snapshot and backup capabilities
//! for RustLite databases, enabling:
//!
//! - **Point-in-time snapshots**: Create consistent snapshots without blocking writes
//! - **Backup and restore**: Full database backups for disaster recovery
//! - **Incremental snapshots**: Copy only changed files since last snapshot
//!
//! ## Usage
//!
//! ```ignore
//! use rustlite_snapshot::{SnapshotManager, SnapshotConfig};
//!
//! let manager = SnapshotManager::new("/path/to/db", SnapshotConfig::default())?;
//! let snapshot = manager.create_snapshot("/path/to/backup")?;
//! println!("Snapshot created at: {}", snapshot.path);
//! ```

use rustlite_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod manager;

/// Snapshot metadata file name
const SNAPSHOT_META_FILE: &str = "SNAPSHOT_META";

/// Snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    /// Unique snapshot ID
    pub id: String,
    /// Timestamp when snapshot was created (Unix milliseconds)
    pub timestamp: u64,
    /// Path where snapshot is stored
    pub path: String,
    /// Source database path
    pub source_path: String,
    /// Sequence number at snapshot time
    pub sequence: u64,
    /// List of files included in the snapshot
    pub files: Vec<SnapshotFile>,
    /// Total size in bytes
    pub total_size: u64,
    /// Snapshot type (full or incremental)
    pub snapshot_type: SnapshotType,
    /// Parent snapshot ID (for incremental snapshots)
    pub parent_id: Option<String>,
}

/// File included in a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFile {
    /// Relative path within the database directory
    pub relative_path: String,
    /// File size in bytes
    pub size: u64,
    /// Last modified timestamp
    pub modified: u64,
    /// Checksum (CRC32)
    pub checksum: u32,
}

/// Type of snapshot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotType {
    /// Full snapshot - includes all files
    Full,
    /// Incremental snapshot - only changed files since parent
    Incremental,
}

/// Snapshot configuration
#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    /// Include WAL files in snapshot
    pub include_wal: bool,
    /// Verify checksums after copy
    pub verify_checksums: bool,
    /// Compression level (0 = none, 1-9 = gzip levels)
    pub compression: u8,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            include_wal: true,
            verify_checksums: true,
            compression: 0,
        }
    }
}

/// Snapshot manager
pub struct SnapshotManager {
    /// Source database directory
    source_dir: PathBuf,
    /// Configuration
    config: SnapshotConfig,
    /// List of created snapshots
    snapshots: Vec<SnapshotMeta>,
}

impl SnapshotManager {
    /// Create a new snapshot manager for the given database directory
    pub fn new(source_dir: impl AsRef<Path>) -> Result<Self> {
        Self::with_config(source_dir, SnapshotConfig::default())
    }

    /// Create a new snapshot manager with custom configuration
    pub fn with_config(source_dir: impl AsRef<Path>, config: SnapshotConfig) -> Result<Self> {
        let source_dir = source_dir.as_ref().to_path_buf();

        if !source_dir.exists() {
            return Err(Error::Storage(format!(
                "Source directory does not exist: {:?}",
                source_dir
            )));
        }

        Ok(Self {
            source_dir,
            config,
            snapshots: Vec::new(),
        })
    }

    /// Create a full snapshot of the database
    pub fn create_snapshot(&mut self, dest: impl AsRef<Path>) -> Result<SnapshotMeta> {
        let dest = dest.as_ref().to_path_buf();

        // Create destination directory
        fs::create_dir_all(&dest)?;

        // Generate snapshot ID
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let id = format!("snap_{}", timestamp);

        // Collect files to copy
        let mut files = Vec::new();
        let mut total_size = 0u64;

        self.collect_files(
            &self.source_dir.clone(),
            &self.source_dir.clone(),
            &mut files,
            &mut total_size,
        )?;

        // Copy files
        for file in &files {
            let src_path = self.source_dir.join(&file.relative_path);
            let dst_path = dest.join(&file.relative_path);

            // Create parent directories
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Copy file
            fs::copy(&src_path, &dst_path)?;

            // Verify if configured
            if self.config.verify_checksums {
                let copied_checksum = Self::compute_checksum(&dst_path)?;
                if copied_checksum != file.checksum {
                    return Err(Error::Corruption(format!(
                        "Checksum mismatch for {}: expected {}, got {}",
                        file.relative_path, file.checksum, copied_checksum
                    )));
                }
            }
        }
        
        // Get sequence number from manifest
        let sequence = self.read_sequence()?;
        
        // Create metadata
        let meta = SnapshotMeta {
            id: id.clone(),
            timestamp,
            path: dest.to_string_lossy().to_string(),
            source_path: self.source_dir.to_string_lossy().to_string(),
            sequence,
            files,
            total_size,
            snapshot_type: SnapshotType::Full,
            parent_id: None,
        };
        
        // Write metadata file
        self.write_metadata(&dest, &meta)?;
        
        // Track snapshot
        self.snapshots.push(meta.clone());
        
        Ok(meta)
    }

    /// Collect all files to include in the snapshot
    fn collect_files(
        &self,
        dir: &Path,
        base: &Path,
        files: &mut Vec<SnapshotFile>,
        total_size: &mut u64,
    ) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip certain directories/files
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "lock" || name.starts_with('.') {
                continue;
            }

            // Skip WAL if not configured
            if !self.config.include_wal && name == "wal" {
                continue;
            }

            if path.is_dir() {
                self.collect_files(&path, base, files, total_size)?;
            } else {
                let relative_path = path
                    .strip_prefix(base)
                    .map_err(|_| Error::Storage("Failed to get relative path".into()))?
                    .to_string_lossy()
                    .to_string();

                let metadata = fs::metadata(&path)?;
                let size = metadata.len();
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                let checksum = Self::compute_checksum(&path)?;

                files.push(SnapshotFile {
                    relative_path,
                    size,
                    modified,
                    checksum,
                });

                *total_size += size;
            }
        }

        Ok(())
    }

    /// Compute CRC32 checksum of a file
    fn compute_checksum(path: &Path) -> Result<u32> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut hasher = crc32fast::Hasher::new();

        let mut buffer = [0u8; 8192];
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(hasher.finalize())
    }

    /// Read sequence number from manifest
    fn read_sequence(&self) -> Result<u64> {
        // Try to read from manifest
        let manifest_path = self.source_dir.join("MANIFEST");
        if !manifest_path.exists() {
            return Ok(0);
        }
        
        // For now, return 0 - in a real implementation, we'd parse the manifest
        Ok(0)
    }

    /// Write snapshot metadata to file
    fn write_metadata(&self, dest: &Path, meta: &SnapshotMeta) -> Result<()> {
        let meta_path = dest.join(SNAPSHOT_META_FILE);
        let file = File::create(&meta_path)?;
        let mut writer = BufWriter::new(file);

        let encoded =
            bincode::serialize(meta).map_err(|e| Error::Serialization(e.to_string()))?;

        writer.write_all(&encoded)?;
        writer.flush()?;

        Ok(())
    }

    /// Load snapshot metadata from a snapshot directory
    pub fn load_snapshot(snapshot_dir: impl AsRef<Path>) -> Result<SnapshotMeta> {
        let meta_path = snapshot_dir.as_ref().join(SNAPSHOT_META_FILE);
        let file = File::open(&meta_path)?;
        let mut reader = BufReader::new(file);

        let mut contents = Vec::new();
        reader.read_to_end(&mut contents)?;

        let meta: SnapshotMeta =
            bincode::deserialize(&contents).map_err(|e| Error::Serialization(e.to_string()))?;

        Ok(meta)
    }

    /// Restore a database from a snapshot
    pub fn restore_snapshot(&self, snapshot: &SnapshotMeta, dest: impl AsRef<Path>) -> Result<()> {
        let dest = dest.as_ref().to_path_buf();
        let snapshot_dir = PathBuf::from(&snapshot.path);

        // Create destination directory
        fs::create_dir_all(&dest)?;

        // Copy all files from snapshot
        for file in &snapshot.files {
            let src_path = snapshot_dir.join(&file.relative_path);
            let dst_path = dest.join(&file.relative_path);

            // Create parent directories
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Copy file
            if src_path.exists() {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    /// List all tracked snapshots
    pub fn list_snapshots(&self) -> &[SnapshotMeta] {
        &self.snapshots
    }

    /// Delete a snapshot
    pub fn delete_snapshot(&mut self, snapshot_id: &str) -> Result<bool> {
        // Find and remove from tracking
        let pos = self.snapshots.iter().position(|s| s.id == snapshot_id);

        if let Some(idx) = pos {
            let snapshot = self.snapshots.remove(idx);

            // Delete the directory
            let path = PathBuf::from(&snapshot.path);
            if path.exists() {
                fs::remove_dir_all(&path)?;
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get snapshot by ID
    pub fn get_snapshot(&self, id: &str) -> Option<&SnapshotMeta> {
        self.snapshots.iter().find(|s| s.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_db(dir: &Path) {
        // Create a mock database structure
        fs::create_dir_all(dir.join("sst")).unwrap();
        fs::create_dir_all(dir.join("wal")).unwrap();
        
        fs::write(dir.join("MANIFEST"), b"test manifest").unwrap();
        fs::write(dir.join("sst/L0_001.sst"), b"test sstable data").unwrap();
        fs::write(dir.join("wal/00000001.wal"), b"test wal data").unwrap();
    }

    #[test]
    fn test_snapshot_manager_new() {
        let dir = tempdir().unwrap();
        create_test_db(dir.path());
        
        let manager = SnapshotManager::new(dir.path()).unwrap();
        assert!(manager.list_snapshots().is_empty());
    }

    #[test]
    fn test_create_snapshot() {
        let source_dir = tempdir().unwrap();
        let dest_dir = tempdir().unwrap();
        
        create_test_db(source_dir.path());
        
        let mut manager = SnapshotManager::new(source_dir.path()).unwrap();
        let snapshot = manager.create_snapshot(dest_dir.path()).unwrap();
        
        assert!(snapshot.id.starts_with("snap_"));
        assert_eq!(snapshot.snapshot_type, SnapshotType::Full);
        assert!(!snapshot.files.is_empty());
        
        // Verify files were copied
        assert!(dest_dir.path().join("MANIFEST").exists());
        assert!(dest_dir.path().join("sst/L0_001.sst").exists());
        assert!(dest_dir.path().join("wal/00000001.wal").exists());
        assert!(dest_dir.path().join(SNAPSHOT_META_FILE).exists());
    }

    #[test]
    fn test_load_snapshot() {
        let source_dir = tempdir().unwrap();
        let dest_dir = tempdir().unwrap();
        
        create_test_db(source_dir.path());
        
        let mut manager = SnapshotManager::new(source_dir.path()).unwrap();
        let original = manager.create_snapshot(dest_dir.path()).unwrap();
        
        // Load the snapshot from disk
        let loaded = SnapshotManager::load_snapshot(dest_dir.path()).unwrap();
        
        assert_eq!(loaded.id, original.id);
        assert_eq!(loaded.files.len(), original.files.len());
    }

    #[test]
    fn test_restore_snapshot() {
        let source_dir = tempdir().unwrap();
        let snapshot_dir = tempdir().unwrap();
        let restore_dir = tempdir().unwrap();
        
        create_test_db(source_dir.path());
        
        let mut manager = SnapshotManager::new(source_dir.path()).unwrap();
        let snapshot = manager.create_snapshot(snapshot_dir.path()).unwrap();
        
        // Restore to new location
        manager.restore_snapshot(&snapshot, restore_dir.path()).unwrap();
        
        // Verify files were restored
        assert!(restore_dir.path().join("MANIFEST").exists());
        assert!(restore_dir.path().join("sst/L0_001.sst").exists());
    }

    #[test]
    fn test_delete_snapshot() {
        let source_dir = tempdir().unwrap();
        let dest_dir = tempdir().unwrap();
        
        create_test_db(source_dir.path());
        
        let mut manager = SnapshotManager::new(source_dir.path()).unwrap();
        let snapshot = manager.create_snapshot(dest_dir.path()).unwrap();
        
        assert_eq!(manager.list_snapshots().len(), 1);
        
        let deleted = manager.delete_snapshot(&snapshot.id).unwrap();
        assert!(deleted);
        assert!(manager.list_snapshots().is_empty());
    }

    #[test]
    fn test_checksum_verification() {
        let source_dir = tempdir().unwrap();
        
        create_test_db(source_dir.path());
        
        // Compute checksum
        let checksum = SnapshotManager::compute_checksum(&source_dir.path().join("MANIFEST")).unwrap();
        assert!(checksum > 0);
    }

    #[test]
    fn test_snapshot_without_wal() {
        let source_dir = tempdir().unwrap();
        let dest_dir = tempdir().unwrap();
        
        create_test_db(source_dir.path());
        
        let config = SnapshotConfig {
            include_wal: false,
            ..Default::default()
        };
        
        let mut manager = SnapshotManager::with_config(source_dir.path(), config).unwrap();
        let snapshot = manager.create_snapshot(dest_dir.path()).unwrap();
        
        // WAL should not be included
        assert!(!snapshot.files.iter().any(|f| f.relative_path.contains("wal")));
    }
}
