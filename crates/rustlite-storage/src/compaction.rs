//! Compaction - Background merging and level management
//!
//! Compaction merges SSTables to reduce read amplification and
//! reclaim space from deleted entries (tombstones).

use crate::manifest::Manifest;
use crate::sstable::{delete_sstable, SSTableEntry, SSTableMeta, SSTableReader, SSTableWriter};
use rustlite_core::Result;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering};
use std::sync::Arc;

/// Compaction configuration
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Maximum number of SSTables at level 0 before triggering compaction
    pub level0_trigger: usize,
    /// Size multiplier between levels (e.g., 10 means level N+1 is 10x larger)
    pub level_multiplier: usize,
    /// Maximum size for level 1 in bytes
    pub level1_max_size: u64,
    /// Maximum number of levels
    pub max_levels: u32,
    /// Target file size for output SSTables
    pub target_file_size: u64,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            level0_trigger: 4,
            level_multiplier: 10,
            level1_max_size: 10 * 1024 * 1024, // 10MB
            max_levels: 7,
            target_file_size: 2 * 1024 * 1024, // 2MB
        }
    }
}

/// Statistics for compaction
#[derive(Debug, Clone, Default)]
pub struct CompactionStats {
    /// Total bytes read during compaction
    pub bytes_read: u64,
    /// Total bytes written during compaction
    pub bytes_written: u64,
    /// Number of compactions performed
    pub compaction_count: u64,
    /// Number of entries removed (tombstones + overwritten)
    pub entries_removed: u64,
}

/// Entry for merge iterator (with ordering)
#[derive(Debug)]
struct MergeEntry {
    key: Vec<u8>,
    entry: SSTableEntry,
    source_idx: usize,
}

impl PartialEq for MergeEntry {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.source_idx == other.source_idx
    }
}

impl Eq for MergeEntry {}

impl PartialOrd for MergeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MergeEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max-heap, so we reverse key comparison for min-heap behavior
        // For equal keys, higher source_idx (newer files) should come first
        match other.key.cmp(&self.key) {
            Ordering::Equal => self.source_idx.cmp(&other.source_idx),
            ord => ord,
        }
    }
}

/// Compaction worker
pub struct CompactionWorker {
    /// Database directory
    dir: PathBuf,
    /// Configuration
    config: CompactionConfig,
    /// Statistics
    stats: CompactionStats,
    /// Counter for generating unique SSTable names
    file_counter: AtomicU64,
    /// Flag to stop compaction
    stop_flag: Arc<AtomicBool>,
}

impl CompactionWorker {
    /// Create a new compaction worker
    pub fn new(dir: impl AsRef<Path>, config: CompactionConfig) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
            config,
            stats: CompactionStats::default(),
            file_counter: AtomicU64::new(0),
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the stop flag for external control
    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stop_flag)
    }

    /// Check if compaction is needed for level 0
    pub fn needs_compaction(&self, manifest: &Manifest) -> bool {
        let level0_count = manifest.sstables_at_level(0).len();
        level0_count >= self.config.level0_trigger
    }

    /// Check which level needs compaction
    pub fn pick_compaction_level(&self, manifest: &Manifest) -> Option<u32> {
        // Check level 0 first
        let level0_count = manifest.sstables_at_level(0).len();
        if level0_count >= self.config.level0_trigger {
            return Some(0);
        }
        
        // Check other levels by size
        for level in 1..self.config.max_levels {
            let level_size: u64 = manifest.sstables_at_level(level)
                .iter()
                .map(|s| s.file_size)
                .sum();
            
            let max_size = self.max_size_for_level(level);
            if level_size > max_size {
                return Some(level);
            }
        }
        
        None
    }

    /// Get the maximum size for a level
    fn max_size_for_level(&self, level: u32) -> u64 {
        if level == 0 {
            return u64::MAX; // Level 0 is count-based, not size-based
        }
        
        let mut size = self.config.level1_max_size;
        for _ in 1..level {
            size *= self.config.level_multiplier as u64;
        }
        size
    }

    /// Generate a unique SSTable path
    fn next_sstable_path(&self, level: u32) -> PathBuf {
        let counter = self.file_counter.fetch_add(1, AtomicOrdering::SeqCst);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        
        self.dir.join("sst").join(format!(
            "L{}_{}_{}.sst",
            level, timestamp, counter
        ))
    }

    /// Compact level 0 to level 1
    pub fn compact_level0(&mut self, manifest: &mut Manifest) -> Result<()> {
        let level0_sstables = manifest.sstables_at_level(0);
        if level0_sstables.is_empty() {
            return Ok(());
        }
        
        // Collect all level 0 SSTable paths
        let input_paths: Vec<PathBuf> = level0_sstables
            .iter()
            .map(|s| PathBuf::from(&s.path))
            .collect();
        
        // Find overlapping level 1 SSTables
        let level1_sstables = manifest.sstables_at_level(1);
        
        // For simplicity, merge all level 0 with overlapping level 1
        let mut all_inputs: Vec<PathBuf> = input_paths.clone();
        
        // Get min/max key range from level 0
        let min_key: Vec<u8> = level0_sstables.iter()
            .map(|s| s.min_key.clone())
            .min()
            .unwrap_or_default();
        let max_key: Vec<u8> = level0_sstables.iter()
            .map(|s| s.max_key.clone())
            .max()
            .unwrap_or_default();
        
        // Add overlapping level 1 SSTables
        for sst in level1_sstables {
            if sst.max_key >= min_key && sst.min_key <= max_key {
                all_inputs.push(PathBuf::from(&sst.path));
            }
        }
        
        // Perform the merge
        let outputs = self.merge_sstables(&all_inputs, 1)?;
        
        // Update manifest
        manifest.record_compaction(0, all_inputs.clone(), outputs)?;
        
        // Delete old files
        for path in all_inputs {
            let _ = delete_sstable(&path);
        }
        
        self.stats.compaction_count += 1;
        
        Ok(())
    }

    /// Merge multiple SSTables into new SSTables at the target level
    fn merge_sstables(&mut self, inputs: &[PathBuf], target_level: u32) -> Result<Vec<SSTableMeta>> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        
        // Create SST directory if needed
        let sst_dir = self.dir.join("sst");
        std::fs::create_dir_all(&sst_dir)?;
        
        // Open all input SSTables
        let mut readers: Vec<SSTableReader> = Vec::new();
        for path in inputs {
            if path.exists() {
                match SSTableReader::open(path) {
                    Ok(reader) => {
                        self.stats.bytes_read += reader.metadata().file_size;
                        readers.push(reader);
                    }
                    Err(_) => continue, // Skip corrupted files
                }
            }
        }
        
        if readers.is_empty() {
            return Ok(Vec::new());
        }
        
        // Initialize merge heap
        let mut heap: BinaryHeap<MergeEntry> = BinaryHeap::new();
        let mut iterators: Vec<_> = readers.iter_mut()
            .map(|r| r.iter())
            .collect::<Result<Vec<_>>>()?;
        
        // Prime the heap with first entry from each SSTable
        for (idx, iter) in iterators.iter_mut().enumerate() {
            if let Some(entry) = iter.next_entry()? {
                heap.push(MergeEntry {
                    key: entry.key.clone(),
                    entry,
                    source_idx: idx,
                });
            }
        }
        
        // Output SSTables
        let mut outputs: Vec<SSTableMeta> = Vec::new();
        let mut current_writer: Option<SSTableWriter> = None;
        let mut current_size: u64 = 0;
        let mut last_key: Option<Vec<u8>> = None;
        
        while let Some(merge_entry) = heap.pop() {
            // Skip duplicate keys (keep the newest - higher source_idx)
            if last_key.as_ref() == Some(&merge_entry.key) {
                self.stats.entries_removed += 1;
                // Advance the iterator that provided this entry
                if let Some(next) = iterators[merge_entry.source_idx].next_entry()? {
                    heap.push(MergeEntry {
                        key: next.key.clone(),
                        entry: next,
                        source_idx: merge_entry.source_idx,
                    });
                }
                continue;
            }
            
            // Start a new SSTable if needed
            if current_writer.is_none() || current_size >= self.config.target_file_size {
                // Finish current writer
                if let Some(writer) = current_writer.take() {
                    let meta = writer.finish()?;
                    self.stats.bytes_written += meta.file_size;
                    outputs.push(meta);
                }
                
                // Start new writer
                let path = self.next_sstable_path(target_level);
                current_writer = Some(SSTableWriter::new(&path)?);
                current_size = 0;
            }
            
            // Write entry
            if let Some(ref mut writer) = current_writer {
                let entry_size = merge_entry.entry.key.len() + merge_entry.entry.value.len() + 10;
                writer.add(merge_entry.entry.clone())?;
                current_size += entry_size as u64;
            }
            
            last_key = Some(merge_entry.key);
            
            // Advance the iterator that provided this entry
            if let Some(next) = iterators[merge_entry.source_idx].next_entry()? {
                heap.push(MergeEntry {
                    key: next.key.clone(),
                    entry: next,
                    source_idx: merge_entry.source_idx,
                });
            }
        }
        
        // Finish last writer
        if let Some(writer) = current_writer {
            let meta = writer.finish()?;
            self.stats.bytes_written += meta.file_size;
            outputs.push(meta);
        }
        
        // Update level in output metadata
        let outputs: Vec<SSTableMeta> = outputs.into_iter()
            .map(|mut m| {
                m.level = target_level;
                m
            })
            .collect();
        
        Ok(outputs)
    }

    /// Get compaction statistics
    pub fn stats(&self) -> &CompactionStats {
        &self.stats
    }

    /// Run a single compaction pass
    pub fn run_once(&mut self, manifest: &mut Manifest) -> Result<bool> {
        if self.stop_flag.load(AtomicOrdering::Relaxed) {
            return Ok(false);
        }
        
        if let Some(level) = self.pick_compaction_level(manifest) {
            if level == 0 {
                self.compact_level0(manifest)?;
                return Ok(true);
            }
            // TODO: Implement higher level compaction
        }
        
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sstable::SSTableWriter;
    use tempfile::tempdir;

    #[test]
    fn test_compaction_config_default() {
        let config = CompactionConfig::default();
        assert_eq!(config.level0_trigger, 4);
        assert_eq!(config.max_levels, 7);
    }

    #[test]
    fn test_merge_entry_ordering() {
        let e1 = MergeEntry {
            key: b"a".to_vec(),
            entry: SSTableEntry::value(b"a".to_vec(), b"1".to_vec()),
            source_idx: 0,
        };
        let e2 = MergeEntry {
            key: b"b".to_vec(),
            entry: SSTableEntry::value(b"b".to_vec(), b"2".to_vec()),
            source_idx: 0,
        };
        
        // In a max-heap, larger values come first
        // We want min-heap behavior, so e1 (key "a") should be "greater"
        assert!(e1 > e2);
    }

    #[test]
    fn test_needs_compaction() {
        let dir = tempdir().unwrap();
        let config = CompactionConfig {
            level0_trigger: 2,
            ..Default::default()
        };
        let worker = CompactionWorker::new(dir.path(), config);
        let mut manifest = Manifest::open(dir.path()).unwrap();
        
        assert!(!worker.needs_compaction(&manifest));
        
        // Add level 0 SSTables
        for i in 0..2 {
            let meta = SSTableMeta {
                path: PathBuf::from(format!("test{}.sst", i)),
                min_key: vec![],
                max_key: vec![],
                entry_count: 0,
                file_size: 0,
                level: 0,
                sequence: 0,
            };
            manifest.add_sstable(&meta).unwrap();
        }
        
        assert!(worker.needs_compaction(&manifest));
    }

    #[test]
    fn test_merge_sstables() {
        let dir = tempdir().unwrap();
        let sst_dir = dir.path().join("sst");
        std::fs::create_dir_all(&sst_dir).unwrap();
        
        // Create two SSTables with overlapping keys
        let path1 = sst_dir.join("test1.sst");
        let mut writer1 = SSTableWriter::new(&path1).unwrap();
        writer1.add(SSTableEntry::value(b"a".to_vec(), b"1".to_vec())).unwrap();
        writer1.add(SSTableEntry::value(b"c".to_vec(), b"3".to_vec())).unwrap();
        writer1.finish().unwrap();
        
        let path2 = sst_dir.join("test2.sst");
        let mut writer2 = SSTableWriter::new(&path2).unwrap();
        writer2.add(SSTableEntry::value(b"b".to_vec(), b"2".to_vec())).unwrap();
        writer2.add(SSTableEntry::value(b"c".to_vec(), b"3-new".to_vec())).unwrap(); // Overwrites
        writer2.finish().unwrap();
        
        // Merge
        let config = CompactionConfig::default();
        let mut worker = CompactionWorker::new(dir.path(), config);
        let outputs = worker.merge_sstables(&[path1, path2], 1).unwrap();
        
        assert!(!outputs.is_empty());
        
        // Verify merged content
        let mut reader = SSTableReader::open(&outputs[0].path).unwrap();
        assert_eq!(reader.get(b"a").unwrap().unwrap().value, b"1".to_vec());
        assert_eq!(reader.get(b"b").unwrap().unwrap().value, b"2".to_vec());
        // "c" should have the newer value from the second SSTable
        assert_eq!(reader.get(b"c").unwrap().unwrap().value, b"3-new".to_vec());
    }
}
