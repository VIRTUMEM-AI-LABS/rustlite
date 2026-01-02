// WAL writer module - handles appending records to the log
use crate::record::WalRecord;
use crate::SyncMode;
use rustlite_core::{Error, Result};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use tracing::{debug, info, instrument};

/// Magic bytes for WAL segment files ("RLWL" = RustLite WAL)
const WAL_MAGIC_HEADER: [u8; 4] = *b"RLWL";

/// WAL format version (v1.0.0+)
const WAL_FORMAT_VERSION: u16 = 1;

/// File header written at the start of WAL segment files (v1.0+)
#[derive(Debug, Clone)]
pub struct WalHeader {
    /// Magic bytes: "RLWL"
    pub magic: [u8; 4],
    /// Format version
    pub version: u16,
}

impl WalHeader {
    /// Size of header in bytes
    pub const SIZE: usize = 6; // 4 bytes magic + 2 bytes version

    /// Create a new header with current version
    pub fn new() -> Self {
        Self {
            magic: WAL_MAGIC_HEADER,
            version: WAL_FORMAT_VERSION,
        }
    }

    /// Write header to a writer
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.magic)?;
        writer.write_all(&self.version.to_le_bytes())?;
        Ok(())
    }

    /// Read header from a reader
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;

        if magic != WAL_MAGIC_HEADER {
            return Err(Error::Corruption(format!(
                "Invalid WAL magic: expected {:?}, got {:?}",
                WAL_MAGIC_HEADER, magic
            )));
        }

        let mut version_bytes = [0u8; 2];
        reader.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);

        if version > WAL_FORMAT_VERSION {
            return Err(Error::Corruption(format!(
                "Unsupported WAL version: {} (current: {})",
                version, WAL_FORMAT_VERSION
            )));
        }

        Ok(Self { magic, version })
    }
}
pub struct WalWriter {
    file: BufWriter<File>,
    current_segment: PathBuf,
    current_size: u64,
    max_segment_size: u64,
    sync_mode: SyncMode,
    sequence: u64,
    wal_dir: PathBuf,
}

impl WalWriter {
    #[instrument(skip(wal_dir), fields(wal_dir = ?wal_dir, max_segment_size = max_segment_size))]
    pub fn new(wal_dir: &PathBuf, max_segment_size: u64, sync_mode: SyncMode) -> Result<Self> {
        info!("Creating WAL writer");

        // Create WAL directory if it doesn't exist
        std::fs::create_dir_all(wal_dir)
            .map_err(|e| Error::Storage(format!("Failed to create WAL directory: {}", e)))?;

        // Find existing segments to determine starting sequence
        let starting_sequence = Self::find_max_sequence(wal_dir)?;

        // Generate segment filename with timestamp
        let segment_name = format!("wal-{:016x}.log", starting_sequence);
        let segment_path = wal_dir.join(&segment_name);

        // Open file for appending
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&segment_path)
            .map_err(|e| Error::Storage(format!("Failed to open WAL segment: {}", e)))?;

        // Get current file size for rotation tracking
        let current_size = file.metadata().map(|m| m.len()).unwrap_or(0);

        // Write header if this is a new file (v1.0+)
        if current_size == 0 {
            let header = WalHeader::new();
            header.write_to(&mut file)?;
            file.flush()?;
            debug!("Wrote WAL header to new segment");
        }

        // Get actual size after potentially writing header
        let actual_size = file.metadata().map(|m| m.len()).unwrap_or(0);

        Ok(Self {
            file: BufWriter::new(file),
            current_segment: segment_path,
            current_size: actual_size,
            max_segment_size,
            sync_mode,
            sequence: starting_sequence,
            wal_dir: wal_dir.clone(),
        })
    }

    /// Find the maximum sequence number from existing segments
    fn find_max_sequence(wal_dir: &PathBuf) -> Result<u64> {
        let mut max_seq = 0u64;

        if let Ok(entries) = std::fs::read_dir(wal_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("wal-") && name.ends_with(".log") {
                        // Parse sequence from filename: wal-{seq}.log
                        if let Some(seq_str) = name
                            .strip_prefix("wal-")
                            .and_then(|s| s.strip_suffix(".log"))
                        {
                            if let Ok(seq) = u64::from_str_radix(seq_str, 16) {
                                max_seq = max_seq.max(seq);
                            }
                        }
                    }
                }
            }
        }

        Ok(max_seq)
    }

    #[instrument(skip(self, record), fields(record_type = ?record))]
    pub fn append(&mut self, record: WalRecord) -> Result<u64> {
        debug!(sequence = self.sequence, "Appending WAL record");

        // Encode the record
        let encoded = record.encode()?;
        let record_size = encoded.len() as u64;

        // Check if we need to rotate to a new segment
        if self.current_size + record_size > self.max_segment_size {
            self.rotate_segment()?;
        }

        // Write the encoded record
        self.file
            .write_all(&encoded)
            .map_err(|e| Error::Storage(format!("Failed to write WAL record: {}", e)))?;

        self.current_size += record_size;
        self.sequence += 1;

        // Sync if required
        if matches!(self.sync_mode, SyncMode::Sync) {
            self.sync()?;
        }

        Ok(self.sequence)
    }

    pub fn sync(&mut self) -> Result<()> {
        self.file
            .flush()
            .map_err(|e| Error::Storage(format!("Failed to flush WAL: {}", e)))?;

        self.file
            .get_ref()
            .sync_all()
            .map_err(|e| Error::Storage(format!("Failed to sync WAL: {}", e)))?;

        Ok(())
    }

    fn rotate_segment(&mut self) -> Result<()> {
        // Sync current segment before rotating
        self.sync()?;

        // Increment sequence for new segment
        self.sequence += 1;

        // Generate new segment filename
        let segment_name = format!("wal-{:016x}.log", self.sequence);
        let new_segment = self.wal_dir.join(&segment_name);

        // Open new segment
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&new_segment)
            .map_err(|e| Error::Storage(format!("Failed to create new segment: {}", e)))?;

        // Write header for new segment (v1.0+)
        let header = WalHeader::new();
        header.write_to(&mut file)?;
        file.flush()?;
        let header_size = WalHeader::SIZE as u64;

        debug!(segment = ?new_segment, "Rotated to new WAL segment");

        // Update state
        self.file = BufWriter::new(file);
        self.current_segment = new_segment;
        self.current_size = header_size;

        Ok(())
    }

    /// Get the current segment path
    pub fn current_segment_path(&self) -> &PathBuf {
        &self.current_segment
    }

    /// Get the current sequence number
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Get the current segment size in bytes
    pub fn current_segment_size(&self) -> u64 {
        self.current_size
    }
}

impl Drop for WalWriter {
    fn drop(&mut self) {
        // Best effort sync on drop
        let _ = self.sync();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_wal() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let wal_path = temp_dir.path().join("wal");
        std::fs::create_dir_all(&wal_path).expect("Failed to create WAL dir");
        (temp_dir, wal_path)
    }

    #[test]
    fn test_writer_creation() {
        let (_temp_dir, wal_path) = setup_test_wal();

        let writer = WalWriter::new(&wal_path, 64 * 1024 * 1024, SyncMode::Sync)
            .expect("Failed to create writer");

        assert!(writer.current_segment_path().exists());
        assert_eq!(writer.sequence(), 0);
    }

    #[test]
    fn test_append_single_record() {
        let (_temp_dir, wal_path) = setup_test_wal();

        let mut writer = WalWriter::new(&wal_path, 64 * 1024 * 1024, SyncMode::Sync)
            .expect("Failed to create writer");

        let record = WalRecord::put(b"key1".to_vec(), b"value1".to_vec());
        let seq = writer.append(record).expect("Failed to append");

        assert_eq!(seq, 1);
        assert!(writer.current_segment_size() > 0);
    }

    #[test]
    fn test_append_multiple_records() {
        let (_temp_dir, wal_path) = setup_test_wal();

        let mut writer = WalWriter::new(&wal_path, 64 * 1024 * 1024, SyncMode::Sync)
            .expect("Failed to create writer");

        for i in 0..10 {
            let record = WalRecord::put(
                format!("key{}", i).into_bytes(),
                format!("value{}", i).into_bytes(),
            );
            let seq = writer.append(record).expect("Failed to append");
            assert_eq!(seq, i as u64 + 1);
        }
    }

    #[test]
    fn test_segment_rotation() {
        let (_temp_dir, wal_path) = setup_test_wal();

        // Use small segment size to force rotation
        let mut writer =
            WalWriter::new(&wal_path, 100, SyncMode::Sync).expect("Failed to create writer");

        let initial_segment = writer.current_segment_path().clone();

        // Write enough records to trigger rotation
        for i in 0..10 {
            let record = WalRecord::put(
                format!("key{}", i).into_bytes(),
                format!("value{}", i).into_bytes(),
            );
            writer.append(record).expect("Failed to append");
        }

        // Segment should have changed
        assert_ne!(writer.current_segment_path(), &initial_segment);

        // Should have multiple segment files
        let segments: Vec<_> = std::fs::read_dir(&wal_path)
            .expect("Failed to read dir")
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "log")
                    .unwrap_or(false)
            })
            .collect();

        assert!(
            segments.len() > 1,
            "Expected multiple segments after rotation"
        );
    }

    #[test]
    fn test_sync_modes() {
        for sync_mode in [SyncMode::Sync, SyncMode::Async, SyncMode::None] {
            let (_temp_dir, wal_path) = setup_test_wal();

            let mut writer = WalWriter::new(&wal_path, 64 * 1024 * 1024, sync_mode)
                .expect("Failed to create writer");

            let record = WalRecord::put(b"key".to_vec(), b"value".to_vec());
            writer.append(record).expect("Failed to append");

            // Explicit sync should work in all modes
            writer.sync().expect("Failed to sync");
        }
    }

    #[test]
    fn test_writer_resume_sequence() {
        let (_temp_dir, wal_path) = setup_test_wal();

        // Write some records
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

        // Create new writer - should resume from existing sequence
        let writer = WalWriter::new(&wal_path, 64 * 1024 * 1024, SyncMode::Sync)
            .expect("Failed to create writer");

        // Should pick up from the existing segment
        assert!(writer.current_segment_path().exists());
    }

    #[test]
    fn test_different_record_types() {
        let (_temp_dir, wal_path) = setup_test_wal();

        let mut writer = WalWriter::new(&wal_path, 64 * 1024 * 1024, SyncMode::Sync)
            .expect("Failed to create writer");

        // PUT record
        writer
            .append(WalRecord::put(b"key1".to_vec(), b"value1".to_vec()))
            .expect("Failed to append PUT");

        // DELETE record
        writer
            .append(WalRecord::delete(b"key2".to_vec()))
            .expect("Failed to append DELETE");

        // Transaction records
        writer
            .append(WalRecord::begin_tx(1))
            .expect("Failed to append BEGIN_TX");
        writer
            .append(WalRecord::commit_tx(1))
            .expect("Failed to append COMMIT_TX");

        // Checkpoint record
        writer
            .append(WalRecord::checkpoint(100))
            .expect("Failed to append CHECKPOINT");

        assert_eq!(writer.sequence(), 5);
    }

    #[test]
    fn test_large_record() {
        let (_temp_dir, wal_path) = setup_test_wal();

        let mut writer = WalWriter::new(&wal_path, 64 * 1024 * 1024, SyncMode::Sync)
            .expect("Failed to create writer");

        // Create a large value (1MB)
        let large_value = vec![0u8; 1024 * 1024];
        let record = WalRecord::put(b"large_key".to_vec(), large_value);

        writer
            .append(record)
            .expect("Failed to append large record");

        assert!(writer.current_segment_size() > 1024 * 1024);
    }
}
