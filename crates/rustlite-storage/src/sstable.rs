//! SSTable - Sorted String Table format and I/O
//!
//! SSTables are immutable on-disk files that store key-value pairs in sorted order.
//! They are the primary on-disk storage format for LSM-tree databases.
//!
//! ## File Format
//!
//! ```text
//! +------------------+
//! | Data Blocks      |  <- Key-value pairs grouped in blocks
//! +------------------+
//! | Index Block      |  <- Sparse index pointing to data blocks
//! +------------------+
//! | Footer           |  <- Index offset + magic number + CRC
//! +------------------+
//! ```

use crate::memtable::MemtableEntry;
use rustlite_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Magic number for SSTable files
const SSTABLE_MAGIC: u64 = 0x535354_424C_4954; // "SSTBLIT" in hex-ish

/// Default block size (4KB)
const DEFAULT_BLOCK_SIZE: usize = 4096;

/// Entry type tags
const ENTRY_TYPE_VALUE: u8 = 0;
const ENTRY_TYPE_TOMBSTONE: u8 = 1;

/// A single entry in an SSTable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSTableEntry {
    /// The key
    pub key: Vec<u8>,
    /// Entry type: 0 = value, 1 = tombstone
    pub entry_type: u8,
    /// The value (empty for tombstones)
    pub value: Vec<u8>,
}

impl SSTableEntry {
    /// Create a value entry
    pub fn value(key: Vec<u8>, value: Vec<u8>) -> Self {
        Self {
            key,
            entry_type: ENTRY_TYPE_VALUE,
            value,
        }
    }

    /// Create a tombstone entry
    pub fn tombstone(key: Vec<u8>) -> Self {
        Self {
            key,
            entry_type: ENTRY_TYPE_TOMBSTONE,
            value: Vec::new(),
        }
    }

    /// Check if this is a tombstone
    pub fn is_tombstone(&self) -> bool {
        self.entry_type == ENTRY_TYPE_TOMBSTONE
    }
}

/// Index entry pointing to a data block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// First key in the block
    pub first_key: Vec<u8>,
    /// Offset of the block in the file
    pub offset: u64,
    /// Size of the block in bytes
    pub size: u32,
}

/// SSTable footer containing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSTableFooter {
    /// Offset of the index block
    pub index_offset: u64,
    /// Size of the index block
    pub index_size: u32,
    /// Number of entries in the SSTable
    pub entry_count: u64,
    /// Minimum key in the SSTable
    pub min_key: Vec<u8>,
    /// Maximum key in the SSTable
    pub max_key: Vec<u8>,
    /// Magic number for validation
    pub magic: u64,
    /// CRC32 of the footer data
    pub crc: u32,
}

/// SSTable metadata (in-memory representation)
#[derive(Debug, Clone)]
pub struct SSTableMeta {
    /// Path to the SSTable file
    pub path: PathBuf,
    /// Minimum key
    pub min_key: Vec<u8>,
    /// Maximum key
    pub max_key: Vec<u8>,
    /// Number of entries
    pub entry_count: u64,
    /// File size in bytes
    pub file_size: u64,
    /// Level in the LSM tree (0 = newest)
    pub level: u32,
    /// Sequence number when created
    pub sequence: u64,
}

/// SSTable writer - creates new SSTable files
pub struct SSTableWriter {
    /// Output file path
    path: PathBuf,
    /// Buffered writer
    writer: BufWriter<File>,
    /// Current position in file
    position: u64,
    /// Index entries
    index: Vec<IndexEntry>,
    /// Current block buffer
    block_buffer: Vec<u8>,
    /// Block size threshold
    block_size: usize,
    /// First key of current block
    current_block_first_key: Option<Vec<u8>>,
    /// Entry count
    entry_count: u64,
    /// Minimum key
    min_key: Option<Vec<u8>>,
    /// Maximum key
    max_key: Option<Vec<u8>>,
}

impl SSTableWriter {
    /// Create a new SSTable writer
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        Self::with_block_size(path, DEFAULT_BLOCK_SIZE)
    }

    /// Create a new SSTable writer with custom block size
    pub fn with_block_size(path: impl AsRef<Path>, block_size: usize) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::create(&path)?;

        Ok(Self {
            path,
            writer: BufWriter::new(file),
            position: 0,
            index: Vec::new(),
            block_buffer: Vec::with_capacity(block_size),
            block_size,
            current_block_first_key: None,
            entry_count: 0,
            min_key: None,
            max_key: None,
        })
    }

    /// Add an entry to the SSTable
    pub fn add(&mut self, entry: SSTableEntry) -> Result<()> {
        // Track min/max keys
        if self.min_key.is_none() {
            self.min_key = Some(entry.key.clone());
        }
        self.max_key = Some(entry.key.clone());
        
        // Track first key of block
        if self.current_block_first_key.is_none() {
            self.current_block_first_key = Some(entry.key.clone());
        }
        
        // Serialize entry
        let encoded = bincode::serialize(&entry)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        
        // Write length prefix + entry
        let len = encoded.len() as u32;
        self.block_buffer.extend_from_slice(&len.to_le_bytes());
        self.block_buffer.extend_from_slice(&encoded);
        
        self.entry_count += 1;
        
        // Flush block if it exceeds threshold
        if self.block_buffer.len() >= self.block_size {
            self.flush_block()?;
        }
        
        Ok(())
    }

    /// Flush the current block to disk
    fn flush_block(&mut self) -> Result<()> {
        if self.block_buffer.is_empty() {
            return Ok(());
        }
        
        // Calculate CRC
        let crc = crc32fast::hash(&self.block_buffer);
        
        // Create index entry
        if let Some(first_key) = self.current_block_first_key.take() {
            self.index.push(IndexEntry {
                first_key,
                offset: self.position,
                size: self.block_buffer.len() as u32 + 4, // +4 for CRC
            });
        }

        // Write block data
        self.writer.write_all(&self.block_buffer)?;
        self.position += self.block_buffer.len() as u64;

        // Write block CRC
        self.writer.write_all(&crc.to_le_bytes())?;
        self.position += 4;

        self.block_buffer.clear();

        Ok(())
    }

    /// Finish writing and close the SSTable
    pub fn finish(mut self) -> Result<SSTableMeta> {
        // Flush any remaining data
        self.flush_block()?;
        
        // Write index block
        let index_offset = self.position;
        let index_encoded = bincode::serialize(&self.index)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        let index_size = index_encoded.len() as u32;

        self.writer.write_all(&index_encoded)?;
        self.position += index_size as u64;

        // Write footer
        let min_key = self.min_key.clone().unwrap_or_default();
        let max_key = self.max_key.clone().unwrap_or_default();
        
        let footer_data = SSTableFooter {
            index_offset,
            index_size,
            entry_count: self.entry_count,
            min_key: min_key.clone(),
            max_key: max_key.clone(),
            magic: SSTABLE_MAGIC,
            crc: 0, // Will be set after computing CRC
        };
        
        let footer_encoded = bincode::serialize(&footer_data)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        let footer_crc = crc32fast::hash(&footer_encoded);
        
        // Write footer with correct CRC
        let final_footer = SSTableFooter {
            crc: footer_crc,
            ..footer_data
        };
        let final_footer_encoded = bincode::serialize(&final_footer)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        // Write footer length + footer
        let footer_len = final_footer_encoded.len() as u32;
        self.writer.write_all(&final_footer_encoded)?;
        self.writer.write_all(&footer_len.to_le_bytes())?;

        self.writer.flush()?;

        let file_size = self.position + final_footer_encoded.len() as u64 + 4;
        
        Ok(SSTableMeta {
            path: self.path,
            min_key,
            max_key,
            entry_count: self.entry_count,
            file_size,
            level: 0,
            sequence: 0,
        })
    }

    /// Build an SSTable from a memtable
    pub fn from_memtable<I>(path: impl AsRef<Path>, iter: I) -> Result<SSTableMeta>
    where
        I: Iterator<Item = (Vec<u8>, MemtableEntry)>,
    {
        let mut writer = SSTableWriter::new(path)?;
        
        for (key, entry) in iter {
            let sstable_entry = match entry {
                MemtableEntry::Value(v) => SSTableEntry::value(key, v),
                MemtableEntry::Tombstone => SSTableEntry::tombstone(key),
            };
            writer.add(sstable_entry)?;
        }
        
        writer.finish()
    }
}

/// SSTable reader - reads from existing SSTable files
pub struct SSTableReader {
    /// Path to the SSTable file
    path: PathBuf,
    /// The file handle
    file: BufReader<File>,
    /// Index entries
    index: Vec<IndexEntry>,
    /// Footer metadata
    footer: SSTableFooter,
    /// File size
    file_size: u64,
}

impl SSTableReader {
    /// Open an SSTable file for reading
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path)?;

        // Get file size
        let file_size = file.metadata()?.len();

        if file_size < 4 {
            return Err(Error::Corruption("SSTable too small".into()));
        }

        // Read footer length (last 4 bytes)
        file.seek(SeekFrom::End(-4))?;
        let mut footer_len_buf = [0u8; 4];
        file.read_exact(&mut footer_len_buf)?;
        let footer_len = u32::from_le_bytes(footer_len_buf) as i64;

        // Read footer
        file.seek(SeekFrom::End(-4 - footer_len))?;
        let mut footer_buf = vec![0u8; footer_len as usize];
        file.read_exact(&mut footer_buf)?;

        let footer: SSTableFooter = bincode::deserialize(&footer_buf)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        // Validate magic number
        if footer.magic != SSTABLE_MAGIC {
            return Err(Error::Corruption("Invalid SSTable magic number".into()));
        }

        // Read index
        file.seek(SeekFrom::Start(footer.index_offset))?;
        let mut index_buf = vec![0u8; footer.index_size as usize];
        file.read_exact(&mut index_buf)?;

        let index: Vec<IndexEntry> = bincode::deserialize(&index_buf)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        Ok(Self {
            path,
            file: BufReader::new(file.try_clone()?),
            index,
            footer,
            file_size,
        })
    }

    /// Get a value by key
    pub fn get(&mut self, key: &[u8]) -> Result<Option<SSTableEntry>> {
        // Binary search to find the block that might contain the key
        let block_idx = self.index.partition_point(|entry| entry.first_key.as_slice() <= key);
        
        // The key would be in the previous block (if any)
        if block_idx == 0 {
            // Key is smaller than all keys in the SSTable
            if key < self.footer.min_key.as_slice() {
                return Ok(None);
            }
        }
        
        // Check the block
        let block_idx = if block_idx > 0 { block_idx - 1 } else { 0 };
        
        if block_idx >= self.index.len() {
            return Ok(None);
        }
        
        // Read and search the block
        let block = self.read_block(block_idx)?;
        
        for entry in block {
            if entry.key.as_slice() == key {
                return Ok(Some(entry));
            }
            if entry.key.as_slice() > key {
                break;
            }
        }
        
        Ok(None)
    }

    /// Read a data block by index
    fn read_block(&mut self, block_idx: usize) -> Result<Vec<SSTableEntry>> {
        let index_entry = &self.index[block_idx];

        self.file.seek(SeekFrom::Start(index_entry.offset))?;

        let data_size = index_entry.size as usize - 4; // Subtract CRC size
        let mut data_buf = vec![0u8; data_size];
        self.file.read_exact(&mut data_buf)?;

        // Read and verify CRC
        let mut crc_buf = [0u8; 4];
        self.file.read_exact(&mut crc_buf)?;
        let stored_crc = u32::from_le_bytes(crc_buf);
        let computed_crc = crc32fast::hash(&data_buf);

        if stored_crc != computed_crc {
            return Err(Error::Corruption("Block CRC mismatch".into()));
        }
        
        // Parse entries from block
        let mut entries = Vec::new();
        let mut offset = 0;
        
        while offset < data_buf.len() {
            if offset + 4 > data_buf.len() {
                break;
            }
            
            let len = u32::from_le_bytes([
                data_buf[offset],
                data_buf[offset + 1],
                data_buf[offset + 2],
                data_buf[offset + 3],
            ]) as usize;
            offset += 4;
            
            if offset + len > data_buf.len() {
                break;
            }
            
            let entry: SSTableEntry = bincode::deserialize(&data_buf[offset..offset + len])
                .map_err(|e| Error::Serialization(e.to_string()))?;
            entries.push(entry);
            offset += len;
        }
        
        Ok(entries)
    }

    /// Get metadata about this SSTable
    pub fn metadata(&self) -> SSTableMeta {
        SSTableMeta {
            path: self.path.clone(),
            min_key: self.footer.min_key.clone(),
            max_key: self.footer.max_key.clone(),
            entry_count: self.footer.entry_count,
            file_size: self.file_size,
            level: 0,
            sequence: 0,
        }
    }

    /// Check if a key might be in this SSTable (range check)
    pub fn might_contain(&self, key: &[u8]) -> bool {
        key >= self.footer.min_key.as_slice() && key <= self.footer.max_key.as_slice()
    }

    /// Iterate over all entries in the SSTable
    pub fn iter(&mut self) -> Result<SSTableIterator<'_>> {
        Ok(SSTableIterator {
            reader: self,
            block_idx: 0,
            block_entries: Vec::new(),
            entry_idx: 0,
        })
    }
}

/// Iterator over SSTable entries
pub struct SSTableIterator<'a> {
    reader: &'a mut SSTableReader,
    block_idx: usize,
    block_entries: Vec<SSTableEntry>,
    entry_idx: usize,
}

impl<'a> SSTableIterator<'a> {
    /// Get the next entry
    pub fn next_entry(&mut self) -> Result<Option<SSTableEntry>> {
        loop {
            // If we have entries in the current block, return the next one
            if self.entry_idx < self.block_entries.len() {
                let entry = self.block_entries[self.entry_idx].clone();
                self.entry_idx += 1;
                return Ok(Some(entry));
            }
            
            // Load the next block
            if self.block_idx >= self.reader.index.len() {
                return Ok(None);
            }
            
            self.block_entries = self.reader.read_block(self.block_idx)?;
            self.block_idx += 1;
            self.entry_idx = 0;
        }
    }
}

/// Delete an SSTable file
pub fn delete_sstable(path: impl AsRef<Path>) -> Result<()> {
    fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sstable_write_read() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.sst");
        
        // Write SSTable
        let mut writer = SSTableWriter::new(&path).unwrap();
        writer.add(SSTableEntry::value(b"a".to_vec(), b"1".to_vec())).unwrap();
        writer.add(SSTableEntry::value(b"b".to_vec(), b"2".to_vec())).unwrap();
        writer.add(SSTableEntry::value(b"c".to_vec(), b"3".to_vec())).unwrap();
        let meta = writer.finish().unwrap();
        
        assert_eq!(meta.entry_count, 3);
        assert_eq!(meta.min_key, b"a".to_vec());
        assert_eq!(meta.max_key, b"c".to_vec());
        
        // Read SSTable
        let mut reader = SSTableReader::open(&path).unwrap();
        
        let entry = reader.get(b"a").unwrap().unwrap();
        assert_eq!(entry.value, b"1".to_vec());
        
        let entry = reader.get(b"b").unwrap().unwrap();
        assert_eq!(entry.value, b"2".to_vec());
        
        let entry = reader.get(b"c").unwrap().unwrap();
        assert_eq!(entry.value, b"3".to_vec());
        
        assert!(reader.get(b"d").unwrap().is_none());
    }

    #[test]
    fn test_sstable_tombstone() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.sst");

        // Keys must be added in sorted order
        let mut writer = SSTableWriter::new(&path).unwrap();
        writer
            .add(SSTableEntry::tombstone(b"deleted".to_vec()))
            .unwrap();
        writer
            .add(SSTableEntry::value(b"key".to_vec(), b"value".to_vec()))
            .unwrap();
        writer.finish().unwrap();

        let mut reader = SSTableReader::open(&path).unwrap();

        let entry = reader.get(b"key").unwrap().unwrap();
        assert!(!entry.is_tombstone());

        let entry = reader.get(b"deleted").unwrap().unwrap();
        assert!(entry.is_tombstone());
    }

    #[test]
    fn test_sstable_iterator() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.sst");
        
        let mut writer = SSTableWriter::new(&path).unwrap();
        for i in 0..100 {
            let key = format!("key{:03}", i);
            let value = format!("value{}", i);
            writer.add(SSTableEntry::value(key.into_bytes(), value.into_bytes())).unwrap();
        }
        writer.finish().unwrap();
        
        let mut reader = SSTableReader::open(&path).unwrap();
        let mut iter = reader.iter().unwrap();
        
        let mut count = 0;
        while let Some(_entry) = iter.next_entry().unwrap() {
            count += 1;
        }
        
        assert_eq!(count, 100);
    }

    #[test]
    fn test_sstable_from_memtable() {
        use crate::memtable::Memtable;
        
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.sst");
        
        let mut mt = Memtable::new();
        mt.put(b"a".to_vec(), b"1".to_vec());
        mt.put(b"b".to_vec(), b"2".to_vec());
        mt.delete(b"c".to_vec());
        
        let meta = SSTableWriter::from_memtable(&path, mt.into_iter()).unwrap();
        
        assert_eq!(meta.entry_count, 3);
        
        let mut reader = SSTableReader::open(&path).unwrap();
        assert_eq!(reader.get(b"a").unwrap().unwrap().value, b"1".to_vec());
        assert!(reader.get(b"c").unwrap().unwrap().is_tombstone());
    }

    #[test]
    fn test_sstable_might_contain() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.sst");
        
        let mut writer = SSTableWriter::new(&path).unwrap();
        writer.add(SSTableEntry::value(b"b".to_vec(), b"2".to_vec())).unwrap();
        writer.add(SSTableEntry::value(b"d".to_vec(), b"4".to_vec())).unwrap();
        writer.finish().unwrap();
        
        let reader = SSTableReader::open(&path).unwrap();
        
        assert!(!reader.might_contain(b"a")); // Before range
        assert!(reader.might_contain(b"b"));  // In range
        assert!(reader.might_contain(b"c"));  // In range (might be there)
        assert!(reader.might_contain(b"d"));  // In range
        assert!(!reader.might_contain(b"e")); // After range
    }
}
