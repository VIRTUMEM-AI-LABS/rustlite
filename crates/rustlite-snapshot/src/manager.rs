//! Snapshot manager implementation details
//!
//! This module contains internal implementation details for snapshot management.

use crate::{SnapshotMeta, SnapshotType};
use std::path::Path;

/// Snapshot manager implementation details
pub struct SnapshotManagerImpl {
    /// Base path for snapshots
    base_path: std::path::PathBuf,
}

impl SnapshotManagerImpl {
    /// Create a new snapshot manager implementation
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Check if a snapshot needs to be incremental
    pub fn should_be_incremental(&self, parent: Option<&SnapshotMeta>) -> bool {
        parent.is_some()
    }

    /// Calculate the diff between two snapshots
    pub fn calculate_diff<'a>(
        &self,
        old: &'a SnapshotMeta,
        new_files: &'a [crate::SnapshotFile],
    ) -> Vec<&'a crate::SnapshotFile> {
        // Find files that have changed
        new_files
            .iter()
            .filter(|new_file| {
                // Check if file exists in old snapshot with different checksum
                old.files
                    .iter()
                    .find(|old_file| old_file.relative_path == new_file.relative_path)
                    .map(|old_file| old_file.checksum != new_file.checksum)
                    .unwrap_or(true) // New file
            })
            .collect()
    }

    /// Get the base path
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

/// Snapshot chain for incremental backups
pub struct SnapshotChain {
    /// Chain of snapshots from oldest to newest
    snapshots: Vec<SnapshotMeta>,
}

impl SnapshotChain {
    /// Create a new empty chain
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    /// Add a snapshot to the chain
    pub fn add(&mut self, snapshot: SnapshotMeta) {
        self.snapshots.push(snapshot);
    }

    /// Get the latest snapshot
    pub fn latest(&self) -> Option<&SnapshotMeta> {
        self.snapshots.last()
    }

    /// Get the full chain for recovery
    pub fn chain(&self) -> &[SnapshotMeta] {
        &self.snapshots
    }

    /// Check if the chain is valid (no gaps)
    pub fn is_valid(&self) -> bool {
        if self.snapshots.is_empty() {
            return true;
        }

        // First snapshot must be full
        if self.snapshots[0].snapshot_type != SnapshotType::Full {
            return false;
        }

        // Each incremental must reference the previous
        for i in 1..self.snapshots.len() {
            if self.snapshots[i].snapshot_type == SnapshotType::Incremental {
                if self.snapshots[i].parent_id.as_ref() != Some(&self.snapshots[i - 1].id) {
                    return false;
                }
            }
        }

        true
    }
}

impl Default for SnapshotChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_chain_empty() {
        let chain = SnapshotChain::new();
        assert!(chain.is_valid());
        assert!(chain.latest().is_none());
    }

    #[test]
    fn test_snapshot_chain_with_full() {
        let mut chain = SnapshotChain::new();
        chain.add(SnapshotMeta {
            id: "snap_1".to_string(),
            timestamp: 1000,
            path: "/backup/snap_1".to_string(),
            source_path: "/db".to_string(),
            sequence: 100,
            files: vec![],
            total_size: 0,
            snapshot_type: SnapshotType::Full,
            parent_id: None,
        });

        assert!(chain.is_valid());
        assert_eq!(chain.latest().unwrap().id, "snap_1");
    }
}
