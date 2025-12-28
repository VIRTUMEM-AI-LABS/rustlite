//! # RustLite
//!
//! A lightweight, high-performance embedded database written in Rust with ACID guarantees.
//!
//! ## Quick Start
//!
//! ```rust
//! use rustlite::Database;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a new database
//! let db = Database::new()?;
//!
//! // Insert data
//! db.put(b"user:1:name", b"Alice")?;
//! db.put(b"user:1:email", b"alice@example.com")?;
//!
//! // Retrieve data
//! if let Some(name) = db.get(b"user:1:name")? {
//!     println!("Name: {}", String::from_utf8_lossy(&name));
//! }
//!
//! // Delete data
//! db.delete(b"user:1:email")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Features
//!
//! - **v0.1.0**: In-memory key-value store with thread-safe concurrent access
//! - **v0.2.0** (planned): Persistent storage with WAL and crash recovery
//! - **v0.3.0** (planned): Indexing and performance optimizations
//! - **v0.4.0** (planned): SQL-like query engine
//! - **v1.0.0** (planned): Production-ready with full ACID guarantees
//!
//! See [ROADMAP.md](https://github.com/VIRTUMEM-AI-LABS/rustlite/blob/main/ROADMAP.md) for details.

// Re-export core types and traits
pub use rustlite_core::{Database, Error, Result};

// WAL components (v0.2+ - skeleton only)
pub use rustlite_wal::WalManager;

// Storage components (v0.2+ - skeleton only)
pub use rustlite_storage::StorageEngine;

// Snapshot components (v0.2+ - skeleton only)
pub use rustlite_snapshot::{SnapshotManager, SnapshotMeta};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.1.0");
    }
}
