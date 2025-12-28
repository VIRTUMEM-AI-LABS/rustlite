// WAL segment management - handles log rotation, cleanup, and segment metadata
//
// Segments are named: wal-{sequence:016x}.log
// Where sequence is a monotonically increasing hex number

use rustlite_core::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Manages WAL segment files
pub struct SegmentManager {
    wal_dir: PathBuf,
}

/// Information about a WAL segment file
#[derive(Debug, Clone)]
pub struct SegmentInfo {
    /// Path to the segment file
    pub path: PathBuf,
    /// Sequence number extracted from filename
    pub sequence: u64,
    /// File size in bytes
    pub size: u64,
}

impl SegmentManager {
    /// Create a new segment manager for the given WAL directory
    pub fn new(wal_dir: PathBuf) -> Self {
        Self { wal_dir }
    }

    /// List all segment files in order
    pub fn list_segments(&self) -> Result<Vec<SegmentInfo>> {
        if !self.wal_dir.exists() {
            return Ok(Vec::new());
        }

        let mut segments: Vec<SegmentInfo> = fs::read_dir(&self.wal_dir)
            .map_err(|e| Error::Storage(format!("Failed to read WAL directory: {}", e)))?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| self.parse_segment_info(&entry.path()))
            .collect();

        // Sort by sequence number
        segments.sort_by_key(|s| s.sequence);

        Ok(segments)
    }

    /// Parse segment info from a file path
    fn parse_segment_info(&self, path: &Path) -> Option<SegmentInfo> {
        let name = path.file_name()?.to_str()?;

        // Must match pattern: wal-{hex}.log
        if !name.starts_with("wal-") || !name.ends_with(".log") {
            return None;
        }

        let seq_str = name.strip_prefix("wal-")?.strip_suffix(".log")?;
        let sequence = u64::from_str_radix(seq_str, 16).ok()?;

        let size = fs::metadata(path).ok()?.len();

        Some(SegmentInfo {
            path: path.to_path_buf(),
            sequence,
            size,
        })
    }

    /// Get the total size of all segments
    pub fn total_size(&self) -> Result<u64> {
        let segments = self.list_segments()?;
        Ok(segments.iter().map(|s| s.size).sum())
    }

    /// Get the number of segment files
    pub fn segment_count(&self) -> Result<usize> {
        Ok(self.list_segments()?.len())
    }

    /// Delete segments older than the given sequence number
    ///
    /// This is useful after a checkpoint to reclaim disk space.
    /// Returns the number of segments deleted.
    pub fn cleanup_before(&self, sequence: u64) -> Result<usize> {
        let segments = self.list_segments()?;
        let mut deleted = 0;

        for segment in segments {
            if segment.sequence < sequence {
                fs::remove_file(&segment.path).map_err(|e| {
                    Error::Storage(format!(
                        "Failed to delete segment {:?}: {}",
                        segment.path, e
                    ))
                })?;
                deleted += 1;
            }
        }

        Ok(deleted)
    }

    /// Delete all segment files
    ///
    /// Use with caution - this removes all WAL data!
    pub fn cleanup_all(&self) -> Result<usize> {
        let segments = self.list_segments()?;
        let count = segments.len();

        for segment in segments {
            fs::remove_file(&segment.path).map_err(|e| {
                Error::Storage(format!(
                    "Failed to delete segment {:?}: {}",
                    segment.path, e
                ))
            })?;
        }

        Ok(count)
    }

    /// Get the latest (highest sequence) segment
    pub fn latest_segment(&self) -> Result<Option<SegmentInfo>> {
        let segments = self.list_segments()?;
        Ok(segments.into_iter().last())
    }

    /// Get the oldest (lowest sequence) segment
    pub fn oldest_segment(&self) -> Result<Option<SegmentInfo>> {
        let segments = self.list_segments()?;
        Ok(segments.into_iter().next())
    }

    /// Check if the WAL directory exists and is accessible
    pub fn is_available(&self) -> bool {
        self.wal_dir.exists() && self.wal_dir.is_dir()
    }

    /// Create the WAL directory if it doesn't exist
    pub fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.wal_dir)
            .map_err(|e| Error::Storage(format!("Failed to create WAL directory: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SyncMode, WalWriter, WalRecord};
    use tempfile::TempDir;

    fn setup_test_wal() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let wal_path = temp_dir.path().join("wal");
        std::fs::create_dir_all(&wal_path).expect("Failed to create WAL dir");
        (temp_dir, wal_path)
    }

    #[test]
    fn test_empty_directory() {
        let (_temp_dir, wal_path) = setup_test_wal();

        let manager = SegmentManager::new(wal_path);
        let segments = manager.list_segments().expect("Failed to list segments");

        assert!(segments.is_empty());
        assert_eq!(manager.segment_count().unwrap(), 0);
        assert_eq!(manager.total_size().unwrap(), 0);
    }

    #[test]
    fn test_list_segments() {
        let (_temp_dir, wal_path) = setup_test_wal();

        // Create some segments by writing and rotating
        {
            let mut writer = WalWriter::new(&wal_path, 50, SyncMode::Sync)
                .expect("Failed to create writer");

            for i in 0..10 {
                writer
                    .append(WalRecord::put(
                        format!("key{}", i).into_bytes(),
                        format!("value{}", i).into_bytes(),
                    ))
                    .expect("Failed to append");
            }
        }

        let manager = SegmentManager::new(wal_path);
        let segments = manager.list_segments().expect("Failed to list segments");

        assert!(!segments.is_empty());
        // Verify segments are sorted by sequence
        for i in 1..segments.len() {
            assert!(segments[i].sequence > segments[i - 1].sequence);
        }
    }

    #[test]
    fn test_total_size() {
        let (_temp_dir, wal_path) = setup_test_wal();

        {
            let mut writer = WalWriter::new(&wal_path, 64 * 1024 * 1024, SyncMode::Sync)
                .expect("Failed to create writer");

            for i in 0..5 {
                writer
                    .append(WalRecord::put(
                        format!("key{}", i).into_bytes(),
                        format!("value{}", i).into_bytes(),
                    ))
                    .expect("Failed to append");
            }
        }

        let manager = SegmentManager::new(wal_path);
        let total_size = manager.total_size().expect("Failed to get total size");

        assert!(total_size > 0);
    }

    #[test]
    fn test_cleanup_all() {
        let (_temp_dir, wal_path) = setup_test_wal();

        {
            let mut writer = WalWriter::new(&wal_path, 50, SyncMode::Sync)
                .expect("Failed to create writer");

            for i in 0..10 {
                writer
                    .append(WalRecord::put(
                        format!("key{}", i).into_bytes(),
                        format!("val{}", i).into_bytes(),
                    ))
                    .expect("Failed to append");
            }
        }

        let manager = SegmentManager::new(wal_path);

        let initial_count = manager.segment_count().unwrap();
        assert!(initial_count > 0);

        let deleted = manager.cleanup_all().expect("Failed to cleanup");
        assert_eq!(deleted, initial_count);

        assert_eq!(manager.segment_count().unwrap(), 0);
    }

    #[test]
    fn test_latest_and_oldest() {
        let (_temp_dir, wal_path) = setup_test_wal();

        {
            let mut writer = WalWriter::new(&wal_path, 50, SyncMode::Sync)
                .expect("Failed to create writer");

            for i in 0..10 {
                writer
                    .append(WalRecord::put(
                        format!("key{}", i).into_bytes(),
                        format!("val{}", i).into_bytes(),
                    ))
                    .expect("Failed to append");
            }
        }

        let manager = SegmentManager::new(wal_path);

        let oldest = manager.oldest_segment().unwrap().expect("Should have oldest");
        let latest = manager.latest_segment().unwrap().expect("Should have latest");

        assert!(oldest.sequence <= latest.sequence);
    }
}
