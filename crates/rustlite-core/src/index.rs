//! Indexing module for RustLite.
//!
//! This module provides B-Tree and Hash index implementations for efficient data retrieval.
//! 
//! ## Index Types
//! 
//! - **B-Tree Index**: Ordered index supporting range queries and prefix scans
//! - **Hash Index**: Fast O(1) exact-match lookups
//! 
//! ## Example
//! 
//! ```rust
//! use rustlite_core::index::{BTreeIndex, HashIndex, Index};
//! 
//! // B-Tree index for ordered access
//! let mut btree = BTreeIndex::new();
//! btree.insert(b"user:001", 100).unwrap();
//! btree.insert(b"user:002", 200).unwrap();
//! 
//! // Range query
//! let range = btree.range(b"user:001", b"user:999").unwrap();
//! 
//! // Hash index for fast lookups
//! let mut hash = HashIndex::new();
//! hash.insert(b"session:abc", 500).unwrap();
//! assert_eq!(hash.find(b"session:abc").unwrap(), vec![500]);
//! ```

use std::collections::{BTreeMap, HashMap};

/// Index type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    /// B-Tree index for ordered data and range queries
    BTree,
    /// Hash index for O(1) exact matches
    Hash,
    /// Full-text search index (planned for future)
    FullText,
}

impl std::fmt::Display for IndexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexType::BTree => write!(f, "BTree"),
            IndexType::Hash => write!(f, "Hash"),
            IndexType::FullText => write!(f, "FullText"),
        }
    }
}

/// Index trait defining the common interface for all index types
pub trait Index: Send + Sync {
    /// Insert a key-value pair into the index.
    /// The value is typically a pointer/offset to the actual data.
    fn insert(&mut self, key: &[u8], value: u64) -> crate::Result<()>;

    /// Find all values matching the exact key.
    fn find(&self, key: &[u8]) -> crate::Result<Vec<u64>>;

    /// Remove all entries for a key from the index.
    /// Returns true if any entries were removed.
    fn remove(&mut self, key: &[u8]) -> crate::Result<bool>;

    /// Returns the number of entries in the index.
    fn len(&self) -> usize;

    /// Returns true if the index is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all entries from the index.
    fn clear(&mut self);

    /// Returns the index type.
    fn index_type(&self) -> IndexType;
}

// ============================================================================
// B-Tree Index Implementation
// ============================================================================

/// B-Tree based index for ordered key lookups and range queries.
///
/// This index maintains keys in sorted order, enabling:
/// - Exact key lookups
/// - Range queries (e.g., all keys between "a" and "z")
/// - Prefix scans (e.g., all keys starting with "user:")
/// - Ordered iteration
///
/// ## Performance
/// - Insert: O(log n)
/// - Lookup: O(log n)
/// - Range query: O(log n + k) where k is the number of results
///
/// ## Example
///
/// ```rust
/// use rustlite_core::index::{BTreeIndex, Index};
///
/// let mut index = BTreeIndex::new();
/// index.insert(b"apple", 1).unwrap();
/// index.insert(b"banana", 2).unwrap();
/// index.insert(b"cherry", 3).unwrap();
///
/// // Exact lookup
/// assert_eq!(index.find(b"banana").unwrap(), vec![2]);
///
/// // Range query
/// let range = index.range(b"apple", b"cherry").unwrap();
/// assert_eq!(range.len(), 3);
/// ```
#[derive(Debug, Clone)]
pub struct BTreeIndex {
    /// The underlying B-Tree map storing key -> list of values
    tree: BTreeMap<Vec<u8>, Vec<u64>>,
    /// Total number of key-value pairs (a key can have multiple values)
    entry_count: usize,
}

impl BTreeIndex {
    /// Create a new empty B-Tree index.
    pub fn new() -> Self {
        Self {
            tree: BTreeMap::new(),
            entry_count: 0,
        }
    }

    /// Range query: find all entries where key is in [start, end] inclusive.
    ///
    /// Returns a vector of (key, values) pairs in sorted order.
    pub fn range(&self, start: &[u8], end: &[u8]) -> crate::Result<Vec<(Vec<u8>, Vec<u64>)>> {
        use std::ops::Bound;
        
        let results: Vec<_> = self
            .tree
            .range((Bound::Included(start.to_vec()), Bound::Included(end.to_vec())))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        
        Ok(results)
    }

    /// Prefix scan: find all entries where key starts with the given prefix.
    ///
    /// Returns a vector of (key, values) pairs in sorted order.
    pub fn prefix_scan(&self, prefix: &[u8]) -> crate::Result<Vec<(Vec<u8>, Vec<u64>)>> {
        let results: Vec<_> = self
            .tree
            .range(prefix.to_vec()..)
            .take_while(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        
        Ok(results)
    }

    /// Get the minimum key in the index, if any.
    pub fn min_key(&self) -> Option<&[u8]> {
        self.tree.keys().next().map(|k| k.as_slice())
    }

    /// Get the maximum key in the index, if any.
    pub fn max_key(&self) -> Option<&[u8]> {
        self.tree.keys().next_back().map(|k| k.as_slice())
    }

    /// Iterate over all entries in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = (&[u8], &[u64])> {
        self.tree.iter().map(|(k, v)| (k.as_slice(), v.as_slice()))
    }
}

impl Default for BTreeIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl Index for BTreeIndex {
    fn insert(&mut self, key: &[u8], value: u64) -> crate::Result<()> {
        self.tree
            .entry(key.to_vec())
            .or_insert_with(Vec::new)
            .push(value);
        self.entry_count += 1;
        Ok(())
    }

    fn find(&self, key: &[u8]) -> crate::Result<Vec<u64>> {
        Ok(self.tree.get(key).cloned().unwrap_or_default())
    }

    fn remove(&mut self, key: &[u8]) -> crate::Result<bool> {
        if let Some(values) = self.tree.remove(key) {
            self.entry_count -= values.len();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn len(&self) -> usize {
        self.entry_count
    }

    fn clear(&mut self) {
        self.tree.clear();
        self.entry_count = 0;
    }

    fn index_type(&self) -> IndexType {
        IndexType::BTree
    }
}

// ============================================================================
// Hash Index Implementation
// ============================================================================

/// Hash-based index for fast O(1) exact-match lookups.
///
/// This index uses a hash map for constant-time key lookups, making it ideal for:
/// - Primary key lookups
/// - Session/token lookups
/// - Any scenario where you only need exact matches
///
/// ## Performance
/// - Insert: O(1) average
/// - Lookup: O(1) average
/// - Delete: O(1) average
///
/// ## Limitations
/// - Does not support range queries (use BTreeIndex for that)
/// - Does not maintain order
///
/// ## Example
///
/// ```rust
/// use rustlite_core::index::{HashIndex, Index};
///
/// let mut index = HashIndex::new();
/// index.insert(b"session:abc123", 42).unwrap();
/// index.insert(b"session:def456", 43).unwrap();
///
/// // Fast O(1) lookup
/// assert_eq!(index.find(b"session:abc123").unwrap(), vec![42]);
/// assert!(index.find(b"nonexistent").unwrap().is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct HashIndex {
    /// The underlying hash map storing key -> list of values
    map: HashMap<Vec<u8>, Vec<u64>>,
    /// Total number of key-value pairs
    entry_count: usize,
}

impl HashIndex {
    /// Create a new empty Hash index.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            entry_count: 0,
        }
    }

    /// Create a new Hash index with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            entry_count: 0,
        }
    }

    /// Check if the index contains a key.
    pub fn contains_key(&self, key: &[u8]) -> bool {
        self.map.contains_key(key)
    }

    /// Get the number of unique keys in the index.
    pub fn key_count(&self) -> usize {
        self.map.len()
    }

    /// Iterate over all entries (unordered).
    pub fn iter(&self) -> impl Iterator<Item = (&[u8], &[u64])> {
        self.map.iter().map(|(k, v)| (k.as_slice(), v.as_slice()))
    }
}

impl Default for HashIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl Index for HashIndex {
    fn insert(&mut self, key: &[u8], value: u64) -> crate::Result<()> {
        self.map
            .entry(key.to_vec())
            .or_insert_with(Vec::new)
            .push(value);
        self.entry_count += 1;
        Ok(())
    }

    fn find(&self, key: &[u8]) -> crate::Result<Vec<u64>> {
        Ok(self.map.get(key).cloned().unwrap_or_default())
    }

    fn remove(&mut self, key: &[u8]) -> crate::Result<bool> {
        if let Some(values) = self.map.remove(key) {
            self.entry_count -= values.len();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn len(&self) -> usize {
        self.entry_count
    }

    fn clear(&mut self) {
        self.map.clear();
        self.entry_count = 0;
    }

    fn index_type(&self) -> IndexType {
        IndexType::Hash
    }
}

// ============================================================================
// Index Manager
// ============================================================================

/// Manages multiple indexes for a database.
///
/// The IndexManager provides a centralized way to create, access, and manage
/// indexes across the database. It supports both B-Tree and Hash indexes.
///
/// ## Example
///
/// ```rust
/// use rustlite_core::index::{IndexManager, IndexType};
///
/// let mut manager = IndexManager::new();
///
/// // Create indexes
/// manager.create_index("users_by_id", IndexType::Hash).unwrap();
/// manager.create_index("users_by_name", IndexType::BTree).unwrap();
///
/// // Use indexes
/// manager.insert("users_by_id", b"user:1", 100).unwrap();
/// manager.insert("users_by_name", b"alice", 100).unwrap();
/// ```
pub struct IndexManager {
    /// Named indexes
    indexes: HashMap<String, Box<dyn Index>>,
}

impl IndexManager {
    /// Create a new empty index manager.
    pub fn new() -> Self {
        Self {
            indexes: HashMap::new(),
        }
    }

    /// Create a new index with the given name and type.
    pub fn create_index(&mut self, name: &str, index_type: IndexType) -> crate::Result<()> {
        if self.indexes.contains_key(name) {
            return Err(crate::Error::InvalidOperation(format!(
                "Index '{}' already exists",
                name
            )));
        }

        let index: Box<dyn Index> = match index_type {
            IndexType::BTree => Box::new(BTreeIndex::new()),
            IndexType::Hash => Box::new(HashIndex::new()),
            IndexType::FullText => {
                return Err(crate::Error::InvalidOperation(
                    "FullText index not yet implemented".to_string(),
                ))
            }
        };

        self.indexes.insert(name.to_string(), index);
        Ok(())
    }

    /// Drop an index by name.
    pub fn drop_index(&mut self, name: &str) -> crate::Result<bool> {
        Ok(self.indexes.remove(name).is_some())
    }

    /// Get a reference to an index by name.
    pub fn get_index(&self, name: &str) -> Option<&dyn Index> {
        self.indexes.get(name).map(|b| b.as_ref())
    }

    /// Get a mutable reference to an index by name.
    pub fn get_index_mut(&mut self, name: &str) -> Option<&mut (dyn Index + 'static)> {
        self.indexes.get_mut(name).map(|b| b.as_mut())
    }

    /// Insert a key-value pair into a named index.
    pub fn insert(&mut self, name: &str, key: &[u8], value: u64) -> crate::Result<()> {
        let index = self.indexes.get_mut(name).ok_or_else(|| {
            crate::Error::NotFound
        })?;
        index.insert(key, value)
    }

    /// Find values in a named index.
    pub fn find(&self, name: &str, key: &[u8]) -> crate::Result<Vec<u64>> {
        let index = self.indexes.get(name).ok_or_else(|| {
            crate::Error::NotFound
        })?;
        index.find(key)
    }

    /// Remove a key from a named index.
    pub fn remove(&mut self, name: &str, key: &[u8]) -> crate::Result<bool> {
        let index = self.indexes.get_mut(name).ok_or_else(|| {
            crate::Error::NotFound
        })?;
        index.remove(key)
    }

    /// List all index names.
    pub fn list_indexes(&self) -> Vec<&str> {
        self.indexes.keys().map(|s| s.as_str()).collect()
    }

    /// Get information about all indexes.
    pub fn index_info(&self) -> Vec<IndexInfo> {
        self.indexes
            .iter()
            .map(|(name, index)| IndexInfo {
                name: name.clone(),
                index_type: index.index_type(),
                entry_count: index.len(),
            })
            .collect()
    }
}

impl Default for IndexManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about an index.
#[derive(Debug, Clone)]
pub struct IndexInfo {
    /// The name of the index.
    pub name: String,
    /// The type of the index.
    pub index_type: IndexType,
    /// The number of entries in the index.
    pub entry_count: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btree_index_basic_operations() {
        let mut index = BTreeIndex::new();
        
        // Insert
        index.insert(b"key1", 100).unwrap();
        index.insert(b"key2", 200).unwrap();
        index.insert(b"key1", 101).unwrap(); // Duplicate key with different value
        
        // Find
        assert_eq!(index.find(b"key1").unwrap(), vec![100, 101]);
        assert_eq!(index.find(b"key2").unwrap(), vec![200]);
        assert!(index.find(b"key3").unwrap().is_empty());
        
        // Length
        assert_eq!(index.len(), 3);
        
        // Remove
        assert!(index.remove(b"key1").unwrap());
        assert!(!index.remove(b"key1").unwrap());
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_btree_index_range_query() {
        let mut index = BTreeIndex::new();
        
        index.insert(b"a", 1).unwrap();
        index.insert(b"b", 2).unwrap();
        index.insert(b"c", 3).unwrap();
        index.insert(b"d", 4).unwrap();
        index.insert(b"e", 5).unwrap();
        
        let range = index.range(b"b", b"d").unwrap();
        assert_eq!(range.len(), 3);
        assert_eq!(range[0].0, b"b");
        assert_eq!(range[1].0, b"c");
        assert_eq!(range[2].0, b"d");
    }

    #[test]
    fn test_btree_index_prefix_scan() {
        let mut index = BTreeIndex::new();
        
        index.insert(b"user:001", 1).unwrap();
        index.insert(b"user:002", 2).unwrap();
        index.insert(b"user:003", 3).unwrap();
        index.insert(b"order:001", 10).unwrap();
        index.insert(b"order:002", 20).unwrap();
        
        let users = index.prefix_scan(b"user:").unwrap();
        assert_eq!(users.len(), 3);
        
        let orders = index.prefix_scan(b"order:").unwrap();
        assert_eq!(orders.len(), 2);
    }

    #[test]
    fn test_btree_index_min_max() {
        let mut index = BTreeIndex::new();
        
        assert!(index.min_key().is_none());
        assert!(index.max_key().is_none());
        
        index.insert(b"middle", 2).unwrap();
        index.insert(b"first", 1).unwrap();
        index.insert(b"last", 3).unwrap();
        
        assert_eq!(index.min_key(), Some(b"first".as_slice()));
        assert_eq!(index.max_key(), Some(b"middle".as_slice()));
    }

    #[test]
    fn test_hash_index_basic_operations() {
        let mut index = HashIndex::new();
        
        // Insert
        index.insert(b"session:abc", 100).unwrap();
        index.insert(b"session:def", 200).unwrap();
        index.insert(b"session:abc", 101).unwrap();
        
        // Find
        assert_eq!(index.find(b"session:abc").unwrap(), vec![100, 101]);
        assert_eq!(index.find(b"session:def").unwrap(), vec![200]);
        assert!(index.find(b"session:xyz").unwrap().is_empty());
        
        // Contains
        assert!(index.contains_key(b"session:abc"));
        assert!(!index.contains_key(b"session:xyz"));
        
        // Length
        assert_eq!(index.len(), 3);
        assert_eq!(index.key_count(), 2);
    }

    #[test]
    fn test_hash_index_with_capacity() {
        let index = HashIndex::with_capacity(100);
        assert!(index.is_empty());
    }

    #[test]
    fn test_index_manager() {
        let mut manager = IndexManager::new();
        
        // Create indexes
        manager.create_index("users", IndexType::Hash).unwrap();
        manager.create_index("names", IndexType::BTree).unwrap();
        
        // Duplicate name should fail
        assert!(manager.create_index("users", IndexType::Hash).is_err());
        
        // Insert into indexes
        manager.insert("users", b"user:1", 100).unwrap();
        manager.insert("names", b"alice", 100).unwrap();
        manager.insert("names", b"bob", 101).unwrap();
        
        // Find
        assert_eq!(manager.find("users", b"user:1").unwrap(), vec![100]);
        assert_eq!(manager.find("names", b"alice").unwrap(), vec![100]);
        
        // List indexes
        let names = manager.list_indexes();
        assert_eq!(names.len(), 2);
        
        // Index info
        let info = manager.index_info();
        assert_eq!(info.len(), 2);
        
        // Drop index
        assert!(manager.drop_index("users").unwrap());
        assert!(!manager.drop_index("users").unwrap());
        assert_eq!(manager.list_indexes().len(), 1);
    }

    #[test]
    fn test_index_clear() {
        let mut btree = BTreeIndex::new();
        let mut hash = HashIndex::new();
        
        btree.insert(b"key", 1).unwrap();
        hash.insert(b"key", 1).unwrap();
        
        assert!(!btree.is_empty());
        assert!(!hash.is_empty());
        
        btree.clear();
        hash.clear();
        
        assert!(btree.is_empty());
        assert!(hash.is_empty());
    }
}
