//! Manifest - Metadata about current SSTables and database state
//!
//! The manifest tracks which SSTable files are currently active,
//! their levels, and the current sequence number. It is used for
//! recovery and compaction coordination.

use crate::sstable::SSTableMeta;
use rustlite_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

/// Manifest file name
const MANIFEST_FILE: &str = "MANIFEST";
/// Manifest backup file name
const MANIFEST_BACKUP: &str = "MANIFEST.bak";

/// Record type for manifest log entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ManifestRecord {
    /// Add a new SSTable
    AddSSTable {
        level: u32,
        path: String,
        min_key: Vec<u8>,
        max_key: Vec<u8>,
        entry_count: u64,
        file_size: u64,
        sequence: u64,
    },
    /// Remove an SSTable (after compaction)
    RemoveSSTable {
        path: String,
    },
    /// Update the current sequence number
    UpdateSequence {
        sequence: u64,
    },
    /// Compaction completed
    CompactionDone {
        level: u32,
        inputs: Vec<String>,
        outputs: Vec<String>,
    },
}

/// SSTable entry in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSSTable {
    /// Level in the LSM tree
    pub level: u32,
    /// Path to the SSTable file
    pub path: String,
    /// Minimum key
    pub min_key: Vec<u8>,
    /// Maximum key
    pub max_key: Vec<u8>,
    /// Number of entries
    pub entry_count: u64,
    /// File size in bytes
    pub file_size: u64,
    /// Sequence number when created
    pub sequence: u64,
}

impl ManifestSSTable {
    /// Convert to SSTableMeta
    pub fn to_meta(&self) -> SSTableMeta {
        SSTableMeta {
            path: PathBuf::from(&self.path),
            min_key: self.min_key.clone(),
            max_key: self.max_key.clone(),
            entry_count: self.entry_count,
            file_size: self.file_size,
            level: self.level,
            sequence: self.sequence,
        }
    }
}

/// Manifest snapshot (complete state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSnapshot {
    /// Current sequence number
    pub sequence: u64,
    /// All active SSTables
    pub sstables: Vec<ManifestSSTable>,
    /// Version number for compatibility
    pub version: u32,
}

impl Default for ManifestSnapshot {
    fn default() -> Self {
        Self {
            sequence: 0,
            sstables: Vec::new(),
            version: 1,
        }
    }
}

/// Manifest manager - tracks database state
pub struct Manifest {
    /// Database directory
    dir: PathBuf,
    /// Current snapshot
    snapshot: ManifestSnapshot,
    /// Log file for incremental updates
    log_writer: Option<BufWriter<File>>,
    /// Number of log entries since last snapshot
    log_entries: usize,
    /// Threshold for rewriting manifest
    log_threshold: usize,
}

impl Manifest {
    /// Open or create a manifest in the given directory
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;

        let manifest_path = dir.join(MANIFEST_FILE);

        let snapshot = if manifest_path.exists() {
            Self::load_snapshot(&manifest_path)?
        } else {
            ManifestSnapshot::default()
        };

        // Open log file for appending
        let log_writer = Some(BufWriter::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&manifest_path)?,
        ));

        Ok(Self {
            dir,
            snapshot,
            log_writer,
            log_entries: 0,
            log_threshold: 100, // Rewrite after 100 incremental entries
        })
    }

    /// Load a manifest snapshot from disk
    fn load_snapshot(path: &Path) -> Result<ManifestSnapshot> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let mut contents = Vec::new();
        reader.read_to_end(&mut contents)?;

        if contents.is_empty() {
            return Ok(ManifestSnapshot::default());
        }

        // Try to deserialize as snapshot
        match bincode::deserialize::<ManifestSnapshot>(&contents) {
            Ok(snapshot) => Ok(snapshot),
            Err(_) => {
                // Fall back to empty manifest
                Ok(ManifestSnapshot::default())
            }
        }
    }

    /// Write a record to the manifest log
    fn write_record(&mut self, record: &ManifestRecord) -> Result<()> {
        if let Some(ref mut writer) = self.log_writer {
            let encoded = bincode::serialize(record)
                .map_err(|e| Error::Serialization(e.to_string()))?;
            let len = encoded.len() as u32;

            writer.write_all(&len.to_le_bytes())?;
            writer.write_all(&encoded)?;
            writer.flush()?;

            self.log_entries += 1;

            // Rewrite manifest if threshold reached
            if self.log_entries >= self.log_threshold {
                self.rewrite()?;
            }
        }

        Ok(())
    }

    /// Rewrite the manifest as a fresh snapshot
    pub fn rewrite(&mut self) -> Result<()> {
        // Close current log writer
        self.log_writer = None;

        let manifest_path = self.dir.join(MANIFEST_FILE);
        let backup_path = self.dir.join(MANIFEST_BACKUP);

        // Backup current manifest
        if manifest_path.exists() {
            fs::copy(&manifest_path, &backup_path)?;
        }

        // Write new snapshot
        let encoded = bincode::serialize(&self.snapshot)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        fs::write(&manifest_path, &encoded)?;

        // Remove backup
        let _ = fs::remove_file(&backup_path);

        // Reopen log writer
        self.log_writer = Some(BufWriter::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&manifest_path)?,
        ));

        // Write the snapshot to the new file
        if let Some(ref mut writer) = self.log_writer {
            writer.write_all(&encoded)?;
            writer.flush()?;
        }

        self.log_entries = 0;

        Ok(())
    }

    /// Add an SSTable to the manifest
    pub fn add_sstable(&mut self, meta: &SSTableMeta) -> Result<()> {
        let sstable = ManifestSSTable {
            level: meta.level,
            path: meta.path.to_string_lossy().to_string(),
            min_key: meta.min_key.clone(),
            max_key: meta.max_key.clone(),
            entry_count: meta.entry_count,
            file_size: meta.file_size,
            sequence: meta.sequence,
        };
        
        self.snapshot.sstables.push(sstable);
        
        self.write_record(&ManifestRecord::AddSSTable {
            level: meta.level,
            path: meta.path.to_string_lossy().to_string(),
            min_key: meta.min_key.clone(),
            max_key: meta.max_key.clone(),
            entry_count: meta.entry_count,
            file_size: meta.file_size,
            sequence: meta.sequence,
        })?;
        
        Ok(())
    }

    /// Remove an SSTable from the manifest
    pub fn remove_sstable(&mut self, path: &Path) -> Result<()> {
        let path_str = path.to_string_lossy().to_string();
        
        self.snapshot.sstables.retain(|s| s.path != path_str);
        
        self.write_record(&ManifestRecord::RemoveSSTable {
            path: path_str,
        })?;
        
        Ok(())
    }

    /// Update the sequence number
    pub fn update_sequence(&mut self, sequence: u64) -> Result<()> {
        self.snapshot.sequence = sequence;
        
        self.write_record(&ManifestRecord::UpdateSequence { sequence })?;
        
        Ok(())
    }

    /// Get the current sequence number
    pub fn sequence(&self) -> u64 {
        self.snapshot.sequence
    }

    /// Get all SSTables at a given level
    pub fn sstables_at_level(&self, level: u32) -> Vec<&ManifestSSTable> {
        self.snapshot.sstables.iter()
            .filter(|s| s.level == level)
            .collect()
    }

    /// Get all SSTables
    pub fn all_sstables(&self) -> &[ManifestSSTable] {
        &self.snapshot.sstables
    }

    /// Get the number of SSTables at each level
    pub fn level_counts(&self) -> Vec<usize> {
        let max_level = self.snapshot.sstables.iter()
            .map(|s| s.level)
            .max()
            .unwrap_or(0);
        
        let mut counts = vec![0usize; (max_level + 1) as usize];
        for sst in &self.snapshot.sstables {
            counts[sst.level as usize] += 1;
        }
        
        counts
    }

    /// Get total size of all SSTables
    pub fn total_size(&self) -> u64 {
        self.snapshot.sstables.iter().map(|s| s.file_size).sum()
    }

    /// Record a compaction completion
    pub fn record_compaction(&mut self, level: u32, inputs: Vec<PathBuf>, outputs: Vec<SSTableMeta>) -> Result<()> {
        // Remove input files from manifest
        for input in &inputs {
            self.snapshot.sstables.retain(|s| s.path != input.to_string_lossy());
        }
        
        // Add output files to manifest
        for output in &outputs {
            let sstable = ManifestSSTable {
                level: output.level,
                path: output.path.to_string_lossy().to_string(),
                min_key: output.min_key.clone(),
                max_key: output.max_key.clone(),
                entry_count: output.entry_count,
                file_size: output.file_size,
                sequence: output.sequence,
            };
            self.snapshot.sstables.push(sstable);
        }
        
        // Write record
        self.write_record(&ManifestRecord::CompactionDone {
            level,
            inputs: inputs.iter().map(|p| p.to_string_lossy().to_string()).collect(),
            outputs: outputs.iter().map(|p| p.path.to_string_lossy().to_string()).collect(),
        })?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_manifest_create() {
        let dir = tempdir().unwrap();
        let manifest = Manifest::open(dir.path()).unwrap();
        
        assert_eq!(manifest.sequence(), 0);
        assert!(manifest.all_sstables().is_empty());
    }

    #[test]
    fn test_manifest_add_sstable() {
        let dir = tempdir().unwrap();
        let mut manifest = Manifest::open(dir.path()).unwrap();
        
        let meta = SSTableMeta {
            path: PathBuf::from("test.sst"),
            min_key: b"a".to_vec(),
            max_key: b"z".to_vec(),
            entry_count: 100,
            file_size: 1024,
            level: 0,
            sequence: 1,
        };
        
        manifest.add_sstable(&meta).unwrap();
        
        assert_eq!(manifest.all_sstables().len(), 1);
        assert_eq!(manifest.sstables_at_level(0).len(), 1);
        assert_eq!(manifest.sstables_at_level(1).len(), 0);
    }

    #[test]
    fn test_manifest_remove_sstable() {
        let dir = tempdir().unwrap();
        let mut manifest = Manifest::open(dir.path()).unwrap();
        
        let meta = SSTableMeta {
            path: PathBuf::from("test.sst"),
            min_key: b"a".to_vec(),
            max_key: b"z".to_vec(),
            entry_count: 100,
            file_size: 1024,
            level: 0,
            sequence: 1,
        };
        
        manifest.add_sstable(&meta).unwrap();
        assert_eq!(manifest.all_sstables().len(), 1);
        
        manifest.remove_sstable(Path::new("test.sst")).unwrap();
        assert!(manifest.all_sstables().is_empty());
    }

    #[test]
    fn test_manifest_sequence() {
        let dir = tempdir().unwrap();
        let mut manifest = Manifest::open(dir.path()).unwrap();
        
        manifest.update_sequence(100).unwrap();
        assert_eq!(manifest.sequence(), 100);
    }

    #[test]
    fn test_manifest_level_counts() {
        let dir = tempdir().unwrap();
        let mut manifest = Manifest::open(dir.path()).unwrap();
        
        for i in 0..3 {
            let meta = SSTableMeta {
                path: PathBuf::from(format!("l0_{}.sst", i)),
                min_key: vec![],
                max_key: vec![],
                entry_count: 0,
                file_size: 0,
                level: 0,
                sequence: 0,
            };
            manifest.add_sstable(&meta).unwrap();
        }
        
        for i in 0..2 {
            let meta = SSTableMeta {
                path: PathBuf::from(format!("l1_{}.sst", i)),
                min_key: vec![],
                max_key: vec![],
                entry_count: 0,
                file_size: 0,
                level: 1,
                sequence: 0,
            };
            manifest.add_sstable(&meta).unwrap();
        }
        
        let counts = manifest.level_counts();
        assert_eq!(counts[0], 3);
        assert_eq!(counts[1], 2);
    }
}
