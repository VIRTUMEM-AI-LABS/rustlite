//! Memtable - In-memory sorted write buffer
//!
//! The Memtable is an in-memory data structure that holds recent writes
//! before they are flushed to disk as SSTables. It uses a BTreeMap for
//! sorted key order, which enables efficient range scans and ordered iteration.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Entry value in the memtable - can be a value or a tombstone (deletion marker)
#[derive(Debug, Clone, PartialEq)]
pub enum MemtableEntry {
    /// A live value
    Value(Vec<u8>),
    /// A tombstone marking deletion
    Tombstone,
}

impl MemtableEntry {
    /// Returns the size of this entry in bytes
    pub fn size(&self) -> usize {
        match self {
            MemtableEntry::Value(v) => v.len() + 1, // +1 for type tag
            MemtableEntry::Tombstone => 1,
        }
    }
}

/// Memtable - an in-memory sorted write buffer
///
/// Provides O(log n) insert, lookup, and delete operations.
/// When the memtable reaches a size threshold, it should be flushed
/// to disk as an SSTable.
#[derive(Debug)]
pub struct Memtable {
    /// The underlying sorted map
    data: BTreeMap<Vec<u8>, MemtableEntry>,
    /// Approximate size in bytes (for flush threshold checking)
    size_bytes: AtomicU64,
    /// Sequence number for MVCC (future use)
    sequence: AtomicU64,
}

impl Memtable {
    /// Creates a new empty Memtable
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            size_bytes: AtomicU64::new(0),
            sequence: AtomicU64::new(0),
        }
    }

    /// Creates a new Memtable with a starting sequence number
    pub fn with_sequence(sequence: u64) -> Self {
        Self {
            data: BTreeMap::new(),
            size_bytes: AtomicU64::new(0),
            sequence: AtomicU64::new(sequence),
        }
    }

    /// Inserts or updates a key-value pair
    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let key_size = key.len() as u64;
        let value_size = value.len() as u64 + 1; // +1 for entry type

        // Remove old entry size if exists
        if let Some(old) = self.data.get(&key) {
            let old_size = old.size() as u64;
            self.size_bytes
                .fetch_sub(key_size + old_size, Ordering::Relaxed);
        }

        self.data.insert(key.clone(), MemtableEntry::Value(value));
        self.size_bytes
            .fetch_add(key_size + value_size, Ordering::Relaxed);
        self.sequence.fetch_add(1, Ordering::Relaxed);
    }

    /// Retrieves a value by key
    ///
    /// Returns:
    /// - `Some(Some(value))` if the key exists with a value
    /// - `Some(None)` if the key was deleted (tombstone)
    /// - `None` if the key is not in the memtable
    pub fn get(&self, key: &[u8]) -> Option<Option<&[u8]>> {
        self.data.get(key).map(|entry| match entry {
            MemtableEntry::Value(v) => Some(v.as_slice()),
            MemtableEntry::Tombstone => None,
        })
    }

    /// Marks a key as deleted with a tombstone
    pub fn delete(&mut self, key: Vec<u8>) {
        let key_size = key.len() as u64;

        // Remove old entry size if exists
        if let Some(old) = self.data.get(&key) {
            let old_size = old.size() as u64;
            self.size_bytes
                .fetch_sub(key_size + old_size, Ordering::Relaxed);
        }

        self.data.insert(key.clone(), MemtableEntry::Tombstone);
        self.size_bytes.fetch_add(key_size + 1, Ordering::Relaxed); // +1 for tombstone
        self.sequence.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns the approximate size of the memtable in bytes
    pub fn size_bytes(&self) -> u64 {
        self.size_bytes.load(Ordering::Relaxed)
    }

    /// Returns the number of entries in the memtable
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the memtable is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the current sequence number
    pub fn sequence(&self) -> u64 {
        self.sequence.load(Ordering::Relaxed)
    }

    /// Returns an iterator over all entries in sorted order
    pub fn iter(&self) -> impl Iterator<Item = (&Vec<u8>, &MemtableEntry)> {
        self.data.iter()
    }

    /// Returns an iterator over a range of keys
    pub fn range<R>(&self, range: R) -> impl Iterator<Item = (&Vec<u8>, &MemtableEntry)>
    where
        R: std::ops::RangeBounds<Vec<u8>>,
    {
        self.data.range(range)
    }

    /// Clears the memtable
    pub fn clear(&mut self) {
        self.data.clear();
        self.size_bytes.store(0, Ordering::Relaxed);
    }

    /// Consumes the memtable and returns all entries sorted by key
    pub fn drain(self) -> impl Iterator<Item = (Vec<u8>, MemtableEntry)> {
        self.data.into_iter()
    }
}

impl Default for Memtable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memtable_new() {
        let mt = Memtable::new();
        assert!(mt.is_empty());
        assert_eq!(mt.len(), 0);
        assert_eq!(mt.size_bytes(), 0);
    }

    #[test]
    fn test_memtable_put_get() {
        let mut mt = Memtable::new();

        mt.put(b"key1".to_vec(), b"value1".to_vec());
        mt.put(b"key2".to_vec(), b"value2".to_vec());

        assert_eq!(mt.len(), 2);
        assert_eq!(mt.get(b"key1"), Some(Some(b"value1".as_slice())));
        assert_eq!(mt.get(b"key2"), Some(Some(b"value2".as_slice())));
        assert_eq!(mt.get(b"key3"), None);
    }

    #[test]
    fn test_memtable_update() {
        let mut mt = Memtable::new();

        mt.put(b"key".to_vec(), b"value1".to_vec());
        assert_eq!(mt.get(b"key"), Some(Some(b"value1".as_slice())));

        mt.put(b"key".to_vec(), b"value2".to_vec());
        assert_eq!(mt.get(b"key"), Some(Some(b"value2".as_slice())));
        assert_eq!(mt.len(), 1);
    }

    #[test]
    fn test_memtable_delete() {
        let mut mt = Memtable::new();

        mt.put(b"key".to_vec(), b"value".to_vec());
        assert_eq!(mt.get(b"key"), Some(Some(b"value".as_slice())));

        mt.delete(b"key".to_vec());
        // Key exists but is a tombstone
        assert_eq!(mt.get(b"key"), Some(None));
        assert_eq!(mt.len(), 1);
    }

    #[test]
    fn test_memtable_size_tracking() {
        let mut mt = Memtable::new();

        let initial_size = mt.size_bytes();
        mt.put(b"key".to_vec(), b"value".to_vec());

        // Size should have increased
        assert!(mt.size_bytes() > initial_size);
    }

    #[test]
    fn test_memtable_iter_sorted() {
        let mut mt = Memtable::new();

        // Insert in random order
        mt.put(b"c".to_vec(), b"3".to_vec());
        mt.put(b"a".to_vec(), b"1".to_vec());
        mt.put(b"b".to_vec(), b"2".to_vec());

        // Iteration should be sorted
        let keys: Vec<_> = mt.iter().map(|(k, _)| k.clone()).collect();
        assert_eq!(keys, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn test_memtable_sequence() {
        let mut mt = Memtable::with_sequence(100);
        assert_eq!(mt.sequence(), 100);

        mt.put(b"key".to_vec(), b"value".to_vec());
        assert_eq!(mt.sequence(), 101);

        mt.delete(b"key".to_vec());
        assert_eq!(mt.sequence(), 102);
    }

    #[test]
    fn test_memtable_clear() {
        let mut mt = Memtable::new();

        mt.put(b"key1".to_vec(), b"value1".to_vec());
        mt.put(b"key2".to_vec(), b"value2".to_vec());

        assert_eq!(mt.len(), 2);

        mt.clear();

        assert!(mt.is_empty());
        assert_eq!(mt.size_bytes(), 0);
    }
}
