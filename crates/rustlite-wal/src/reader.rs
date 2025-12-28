// WAL reader module - reads and replays log records
//
// The reader handles:
// 1. Segment discovery - finding all WAL segment files in order
// 2. Record reading - iterating through records in each segment
// 3. CRC validation - verifying data integrity of each record

use crate::record::WalRecord;
use rustlite_core::{Error, Result};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

/// WAL reader for reading records from log segments
pub struct WalReader {
    /// Sorted list of segment file paths
    segments: Vec<PathBuf>,
    /// Index of current segment being read
    current_segment_index: usize,
    /// Buffered reader for current segment
    reader: Option<BufReader<File>>,
    /// Current byte offset within segment
    current_offset: u64,
}

impl WalReader {
    /// Create a new WAL reader for the given WAL directory
    pub fn new(wal_dir: &Path) -> Result<Self> {
        let segments = Self::discover_segments(wal_dir)?;

        let mut reader = Self {
            segments,
            current_segment_index: 0,
            reader: None,
            current_offset: 0,
        };

        // Open first segment if available
        if !reader.segments.is_empty() {
            reader.open_segment(0)?;
        }

        Ok(reader)
    }

    /// Discover and sort all WAL segment files in the directory
    fn discover_segments(wal_dir: &Path) -> Result<Vec<PathBuf>> {
        if !wal_dir.exists() {
            return Ok(Vec::new());
        }

        let mut segments: Vec<PathBuf> = std::fs::read_dir(wal_dir)
            .map_err(|e| Error::Storage(format!("Failed to read WAL directory: {}", e)))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .map(|ext| ext == "log")
                    .unwrap_or(false)
            })
            .collect();

        // Sort segments by filename (which contains sequence number)
        segments.sort();

        Ok(segments)
    }

    /// Open a segment file by index
    fn open_segment(&mut self, index: usize) -> Result<()> {
        if index >= self.segments.len() {
            return Err(Error::Storage("Segment index out of bounds".to_string()));
        }

        let path = &self.segments[index];
        let file = File::open(path)
            .map_err(|e| Error::Storage(format!("Failed to open segment {:?}: {}", path, e)))?;

        self.reader = Some(BufReader::new(file));
        self.current_segment_index = index;
        self.current_offset = 0;

        Ok(())
    }

    /// Move to the next segment
    fn advance_segment(&mut self) -> Result<bool> {
        let next_index = self.current_segment_index + 1;
        if next_index >= self.segments.len() {
            self.reader = None;
            return Ok(false);
        }

        self.open_segment(next_index)?;
        Ok(true)
    }

    /// Read the next record from the WAL
    ///
    /// Returns `Ok(Some(record))` if a record was read successfully,
    /// `Ok(None)` if we've reached the end of all segments,
    /// or an error if reading/parsing failed.
    pub fn next_record(&mut self) -> Result<Option<WalRecord>> {
        loop {
            let reader = match &mut self.reader {
                Some(r) => r,
                None => return Ok(None), // No more segments
            };

            // Try to read a record from current segment
            match Self::read_record(reader) {
                Ok(Some((record, bytes_read))) => {
                    self.current_offset += bytes_read as u64;
                    return Ok(Some(record));
                }
                Ok(None) => {
                    // End of current segment, try next
                    if !self.advance_segment()? {
                        return Ok(None);
                    }
                    // Continue loop to read from new segment
                }
                Err(e) => {
                    // Check if this is an incomplete record at end of file
                    // (possible crash during write)
                    if Self::is_truncation_error(&e) {
                        // Try to advance to next segment
                        if !self.advance_segment()? {
                            return Ok(None);
                        }
                        // Continue loop to read from new segment
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Read a single record from a reader
    ///
    /// Returns the record and number of bytes consumed
    fn read_record(reader: &mut BufReader<File>) -> Result<Option<(WalRecord, usize)>> {
        // Read length field (4 bytes)
        let mut len_buf = [0u8; 4];
        match reader.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Ok(None); // End of file
            }
            Err(e) => {
                return Err(Error::Storage(format!("Failed to read record length: {}", e)));
            }
        }

        let content_len = u32::from_le_bytes(len_buf) as usize;

        // Sanity check on length (max 16MB per record)
        if content_len > 16 * 1024 * 1024 {
            return Err(Error::Storage(format!(
                "Record length too large: {} bytes",
                content_len
            )));
        }

        // Read content (type + payload) and CRC
        let total_data_len = content_len + 4; // content + crc
        let mut data = vec![0u8; total_data_len];

        reader.read_exact(&mut data).map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                Error::Serialization("Incomplete record: truncated".to_string())
            } else {
                Error::Storage(format!("Failed to read record data: {}", e))
            }
        })?;

        // Build full frame for decoding
        let mut frame = Vec::with_capacity(4 + total_data_len);
        frame.extend_from_slice(&len_buf);
        frame.extend_from_slice(&data);

        // Decode record (includes CRC validation)
        let (record, bytes_consumed) = WalRecord::decode(&frame)?;

        Ok(Some((record, bytes_consumed)))
    }

    /// Check if an error indicates a truncated/incomplete record
    fn is_truncation_error(err: &Error) -> bool {
        match err {
            Error::Serialization(msg) => msg.contains("Incomplete") || msg.contains("truncated"),
            _ => false,
        }
    }

    /// Get the number of segments discovered
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Get the current segment index being read
    pub fn current_segment(&self) -> usize {
        self.current_segment_index
    }

    /// Reset reader to the beginning
    pub fn reset(&mut self) -> Result<()> {
        if !self.segments.is_empty() {
            self.open_segment(0)?;
        } else {
            self.reader = None;
            self.current_segment_index = 0;
            self.current_offset = 0;
        }
        Ok(())
    }

    /// Seek to a specific segment
    pub fn seek_to_segment(&mut self, index: usize) -> Result<()> {
        if index >= self.segments.len() {
            return Err(Error::Storage(format!(
                "Segment index {} out of range (have {} segments)",
                index,
                self.segments.len()
            )));
        }
        self.open_segment(index)
    }

    /// Read all remaining records into a vector
    pub fn read_all(&mut self) -> Result<Vec<WalRecord>> {
        let mut records = Vec::new();
        while let Some(record) = self.next_record()? {
            records.push(record);
        }
        Ok(records)
    }
}

/// Iterator implementation for WalReader
impl Iterator for WalReader {
    type Item = Result<WalRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_record() {
            Ok(Some(record)) => Some(Ok(record)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SyncMode, WalWriter};
    use tempfile::TempDir;

    fn setup_test_wal() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let wal_path = temp_dir.path().join("wal");
        std::fs::create_dir_all(&wal_path).expect("Failed to create WAL dir");
        (temp_dir, wal_path)
    }

    #[test]
    fn test_empty_wal_reader() {
        let (_temp_dir, wal_path) = setup_test_wal();

        let mut reader = WalReader::new(&wal_path).expect("Failed to create reader");
        assert_eq!(reader.segment_count(), 0);
        assert!(reader.next_record().unwrap().is_none());
    }

    #[test]
    fn test_read_single_record() {
        let (_temp_dir, wal_path) = setup_test_wal();

        // Write a record
        {
            let mut writer =
                WalWriter::new(&wal_path, 64 * 1024 * 1024, SyncMode::Sync).expect("Failed to create writer");
            let record = WalRecord::put(b"key1".to_vec(), b"value1".to_vec());
            writer.append(record).expect("Failed to append");
            writer.sync().expect("Failed to sync");
        }

        // Read it back
        let mut reader = WalReader::new(&wal_path).expect("Failed to create reader");
        assert_eq!(reader.segment_count(), 1);

        let record = reader.next().unwrap().expect("Expected a record");
        match &record.payload {
            crate::record::RecordPayload::Put { key, value } => {
                assert_eq!(key, b"key1");
                assert_eq!(value, b"value1");
            }
            _ => panic!("Expected Put record"),
        }

        assert!(reader.next_record().unwrap().is_none());
    }

    #[test]
    fn test_read_multiple_records() {
        let (_temp_dir, wal_path) = setup_test_wal();

        // Write multiple records
        {
            let mut writer =
                WalWriter::new(&wal_path, 64 * 1024 * 1024, SyncMode::Sync).expect("Failed to create writer");

            for i in 0..10 {
                let record = WalRecord::put(
                    format!("key{}", i).into_bytes(),
                    format!("value{}", i).into_bytes(),
                );
                writer.append(record).expect("Failed to append");
            }
            writer.sync().expect("Failed to sync");
        }

        // Read them back
        let mut reader = WalReader::new(&wal_path).expect("Failed to create reader");
        let records = reader.read_all().expect("Failed to read all");

        assert_eq!(records.len(), 10);
    }

    #[test]
    fn test_read_across_segment_rotation() {
        let (_temp_dir, wal_path) = setup_test_wal();

        // Write with small segment size to force rotation
        {
            let mut writer =
                WalWriter::new(&wal_path, 100, SyncMode::Sync).expect("Failed to create writer");

            for i in 0..20 {
                let record = WalRecord::put(
                    format!("key{}", i).into_bytes(),
                    format!("value{}", i).into_bytes(),
                );
                writer.append(record).expect("Failed to append");
            }
            writer.sync().expect("Failed to sync");
        }

        // Read all records
        let mut reader = WalReader::new(&wal_path).expect("Failed to create reader");
        let records = reader.read_all().expect("Failed to read all");

        assert_eq!(records.len(), 20);
        // Verify multiple segments were created
        assert!(reader.segment_count() > 1, "Expected multiple segments");
    }

    #[test]
    fn test_reader_reset() {
        let (_temp_dir, wal_path) = setup_test_wal();

        // Write some records
        {
            let mut writer =
                WalWriter::new(&wal_path, 64 * 1024 * 1024, SyncMode::Sync).expect("Failed to create writer");

            for i in 0..5 {
                let record = WalRecord::put(
                    format!("key{}", i).into_bytes(),
                    format!("value{}", i).into_bytes(),
                );
                writer.append(record).expect("Failed to append");
            }
            writer.sync().expect("Failed to sync");
        }

        let mut reader = WalReader::new(&wal_path).expect("Failed to create reader");

        // Read all
        let first_read = reader.read_all().expect("Failed to read all");
        assert_eq!(first_read.len(), 5);

        // Reset and read again
        reader.reset().expect("Failed to reset");
        let second_read = reader.read_all().expect("Failed to read all");
        assert_eq!(second_read.len(), 5);
    }

    #[test]
    fn test_reader_with_transaction_markers() {
        let (_temp_dir, wal_path) = setup_test_wal();

        // Write transaction sequence
        {
            let mut writer =
                WalWriter::new(&wal_path, 64 * 1024 * 1024, SyncMode::Sync).expect("Failed to create writer");

            writer.append(WalRecord::begin_tx(1)).expect("Failed to append");
            writer
                .append(WalRecord::put(b"key1".to_vec(), b"val1".to_vec()))
                .expect("Failed to append");
            writer
                .append(WalRecord::put(b"key2".to_vec(), b"val2".to_vec()))
                .expect("Failed to append");
            writer.append(WalRecord::commit_tx(1)).expect("Failed to append");
            writer.sync().expect("Failed to sync");
        }

        let mut reader = WalReader::new(&wal_path).expect("Failed to create reader");
        let records = reader.read_all().expect("Failed to read all");

        assert_eq!(records.len(), 4);
        assert_eq!(records[0].record_type, crate::RecordType::BeginTx);
        assert_eq!(records[1].record_type, crate::RecordType::Put);
        assert_eq!(records[2].record_type, crate::RecordType::Put);
        assert_eq!(records[3].record_type, crate::RecordType::CommitTx);
    }
}
