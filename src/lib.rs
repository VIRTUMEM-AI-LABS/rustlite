//! # RustLite
//!
//! A lightweight, high-performance embedded database written in Rust.
//!
//! RustLite is designed to be:
//! - **Fast**: Optimized for high-throughput operations
//! - **Reliable**: ACID guarantees with write-ahead logging
//! - **Embeddable**: Zero configuration, single-file deployment
//! - **Safe**: Memory-safe by design using Rust's type system
//!
//! ## Features (Roadmap)
//!
//! - v0.1: Core key-value store
//! - v0.2: Persistence and Write-Ahead Logging (WAL)
//! - v0.3: Indexing support (B-Tree, Hash)
//! - v0.4: Query engine with SQL-like syntax
//! - v0.5: Transaction support with MVCC
//! - v1.0: Full ACID compliance and production readiness
//!
//! ## Quick Start
//!
//! ```rust
//! use rustlite::Database;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a new in-memory database
//! let db = Database::new()?;
//!
//! // Insert a key-value pair
//! db.put(b"hello", b"world")?;
//!
//! // Retrieve the value
//! let value = db.get(b"hello")?;
//! assert_eq!(value.as_deref(), Some(&b"world"[..]));
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod error;
pub mod storage;
pub mod transaction;
pub mod index;
pub mod query;
pub mod wal;
pub mod security;

pub use error::{Error, Result};
pub use security::ResourceLimits;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// The main database handle.
///
/// This is the primary interface for interacting with RustLite.
/// It provides thread-safe access to the underlying storage engine.
#[derive(Clone)]
pub struct Database {
    inner: Arc<DatabaseInner>,
}

struct DatabaseInner {
    // Simple in-memory storage for v0.1
    // This will be replaced with proper storage engine in future versions
    store: RwLock<HashMap<Vec<u8>, Vec<u8>>>,
}

impl Database {
    /// Creates a new in-memory database instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustlite::Database;
    ///
    /// let db = Database::new().expect("Failed to create database");
    /// ```
    pub fn new() -> Result<Self> {
        Ok(Database {
            inner: Arc::new(DatabaseInner {
                store: RwLock::new(HashMap::new()),
            }),
        })
    }

    /// Inserts or updates a key-value pair.
    ///
    /// # Arguments
    ///
    /// * `key` - The key as a byte slice
    /// * `value` - The value as a byte slice
    ///
    /// # Examples
    ///
    /// ```
    /// # use rustlite::Database;
    /// # let db = Database::new().unwrap();
    /// db.put(b"name", b"RustLite").expect("Failed to put value");
    /// ```
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let mut store = self.inner.store.write()
            .map_err(|_| Error::LockPoisoned)?;
        store.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    /// Retrieves a value by key.
    ///
    /// Returns `Ok(Some(value))` if the key exists, `Ok(None)` if it doesn't.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    ///
    /// # Examples
    ///
    /// ```
    /// # use rustlite::Database;
    /// # let db = Database::new().unwrap();
    /// # db.put(b"name", b"RustLite").unwrap();
    /// let value = db.get(b"name").expect("Failed to get value");
    /// assert_eq!(value.as_deref(), Some(&b"RustLite"[..]));
    /// ```
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let store = self.inner.store.read()
            .map_err(|_| Error::LockPoisoned)?;
        Ok(store.get(key).cloned())
    }

    /// Deletes a key-value pair.
    ///
    /// Returns `Ok(true)` if the key existed and was deleted, `Ok(false)` otherwise.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to delete
    ///
    /// # Examples
    ///
    /// ```
    /// # use rustlite::Database;
    /// # let db = Database::new().unwrap();
    /// # db.put(b"temp", b"data").unwrap();
    /// let deleted = db.delete(b"temp").expect("Failed to delete");
    /// assert!(deleted);
    /// ```
    pub fn delete(&self, key: &[u8]) -> Result<bool> {
        let mut store = self.inner.store.write()
            .map_err(|_| Error::LockPoisoned)?;
        Ok(store.remove(key).is_some())
    }

    /// Returns the number of key-value pairs in the database.
    pub fn len(&self) -> Result<usize> {
        let store = self.inner.store.read()
            .map_err(|_| Error::LockPoisoned)?;
        Ok(store.len())
    }

    /// Returns `true` if the database contains no key-value pairs.
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new().expect("Failed to create default database")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let db = Database::new().unwrap();
        
        // Test put and get
        db.put(b"key1", b"value1").unwrap();
        assert_eq!(db.get(b"key1").unwrap(), Some(b"value1".to_vec()));
        
        // Test update
        db.put(b"key1", b"value2").unwrap();
        assert_eq!(db.get(b"key1").unwrap(), Some(b"value2".to_vec()));
        
        // Test delete
        assert!(db.delete(b"key1").unwrap());
        assert_eq!(db.get(b"key1").unwrap(), None);
        assert!(!db.delete(b"key1").unwrap());
    }

    #[test]
    fn test_len_and_empty() {
        let db = Database::new().unwrap();
        
        assert!(db.is_empty().unwrap());
        assert_eq!(db.len().unwrap(), 0);
        
        db.put(b"a", b"1").unwrap();
        db.put(b"b", b"2").unwrap();
        
        assert!(!db.is_empty().unwrap());
        assert_eq!(db.len().unwrap(), 2);
    }
}
