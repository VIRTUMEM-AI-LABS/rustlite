//! # RustLite Core
//!
//! Core types and implementations for RustLite embedded database.
//!
//! ## ⚠️ Internal Implementation Detail
//!
//! **This crate is an internal implementation detail of RustLite.**
//!
//! Users should depend on the main [`rustlite`](https://crates.io/crates/rustlite) crate
//! instead, which provides the stable public API. This crate's API may change
//! without notice between minor versions.
//!
//! ```toml
//! # In your Cargo.toml - use the main crate, not this one:
//! [dependencies]
//! rustlite = "0.3"
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;
/// File format versioning for SSTable, WAL, and Manifest
pub mod format_version;
pub mod index;
/// SQL-like query engine (v0.4+)
pub mod query;
pub mod storage;
pub mod transaction;

#[cfg(test)]
mod transaction_tests;

pub use error::{Error, Result};

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
    pub fn new() -> Result<Self> {
        Ok(Database {
            inner: Arc::new(DatabaseInner {
                store: RwLock::new(HashMap::new()),
            }),
        })
    }

    /// Inserts or updates a key-value pair.
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let mut store = self.inner.store.write().map_err(|_| Error::LockPoisoned)?;
        store.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    /// Retrieves a value by key.
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let store = self.inner.store.read().map_err(|_| Error::LockPoisoned)?;
        Ok(store.get(key).cloned())
    }

    /// Deletes a key-value pair.
    pub fn delete(&self, key: &[u8]) -> Result<bool> {
        let mut store = self.inner.store.write().map_err(|_| Error::LockPoisoned)?;
        Ok(store.remove(key).is_some())
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new().expect("Failed to create default database")
    }
}
