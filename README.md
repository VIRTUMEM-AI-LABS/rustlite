# RustLite

<p align="center">
    <img src="https://raw.githubusercontent.com/VIRTUMEM-AI-LABS/rustlite/main/assets/logo-wordmark.svg" alt="RustLite Logo" width="300" />
</p>

<p align="center">

[![Crates.io](https://img.shields.io/crates/v/rustlite.svg)](https://crates.io/crates/rustlite)
[![Documentation](https://docs.rs/rustlite/badge.svg)](https://docs.rs/rustlite)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Build Status](https://github.com/VIRTUMEM-AI-LABS/rustlite/workflows/CI/badge.svg)](https://github.com/VIRTUMEM-AI-LABS/rustlite/actions)
[![Changelog](https://img.shields.io/badge/Changelog-docs/CHANGELOG.md-blue.svg)](https://github.com/VIRTUMEM-AI-LABS/rustlite/blob/main/docs/CHANGELOG.md)

</p>

**RustLite** is a lightweight, high-performance embedded database written entirely in Rust. Designed for applications that need a fast, reliable, and embeddable storage solution with ACID guarantees.

> **⚠️ Important**: RustLite is a **key-value store with MVCC transactions**, not a full relational database. It's similar to LevelDB or RocksDB (but with transactions), NOT SQLite or PostgreSQL. While it includes a basic SQL-like query engine for simple SELECT operations, it lacks schemas, CREATE TABLE, data types, functions, and constraints. For full SQL support, use [rusqlite](https://docs.rs/rusqlite) or SQLite directly.

## 🎯 Vision

RustLite aims to be the go-to embedded database for Rust applications, combining:

- **Performance**: Zero-copy operations, memory-mapped I/O, and efficient data structures
- **Reliability**: Full ACID compliance with write-ahead logging and crash recovery
- **Simplicity**: Single-file deployment, zero configuration, intuitive API
- **Safety**: Memory-safe by design using Rust's type system and ownership model

## 💡 Ideal Use Cases

RustLite excels in scenarios where you need fast, transactional key-value storage:

- **📱 Embedded Applications**: Mobile/desktop apps needing local data storage
- **🔧 Application State**: Configuration, settings, and application metadata
- **💾 Caching Layer**: High-performance caching with persistence
- **🎫 Session Storage**: Web session management with ACID guarantees
- **📊 Time-Series Data**: Event logs, metrics, and analytics data
- **🔄 Event Sourcing**: Append-only event stores with snapshot isolation
- **📨 Message Queues**: Lightweight job queues and task schedulers
- **🎮 Game State**: Player progress, inventory, and game world persistence
- **📝 Document Storage**: Key-based document retrieval (JSON, MessagePack, etc.)
- **🔐 Credential Vaults**: Secure local storage for API keys and secrets

**Not Ideal For:**
- ❌ Complex relational queries with JOINs across multiple tables
- ❌ Applications requiring SQL compatibility (use SQLite/PostgreSQL)
- ❌ Full-text search (no FTS support yet)
- ❌ Large-scale distributed systems (single-node only)

## ✨ Features

### Current (v0.7.0)
- ✅ **Persistent storage** with LSM-tree architecture
- ✅ **Write-Ahead Logging (WAL)** for crash recovery
- ✅ **SSTable compaction** for optimized disk usage
- ✅ **Snapshot backups** for point-in-time recovery
- ✅ **B-Tree indexing** for range queries and ordered lookups
- ✅ **Hash indexing** for O(1) exact-match lookups
- ✅ **SQL-like query engine** with SELECT, WHERE, LIMIT support
- ✅ **Aggregate functions**: COUNT(*), COUNT(column), SUM, AVG, MIN, MAX
- ✅ **GROUP BY** with multiple columns and HAVING clauses
- ✅ **Full MVCC transactions** with snapshot isolation
- ✅ **JOIN operations** (INNER, LEFT, RIGHT, FULL OUTER)
- ✅ **Hash join** and **nested loop join** algorithms
- ✅ **Production logging** with structured tracing
- ✅ Thread-safe concurrent access
- ✅ Simple, ergonomic API

## 🚀 Quick Start

Add RustLite to your `Cargo.toml`:

```toml
[dependencies]
rustlite = "0.5"
```

### Transactions with MVCC (v0.5.0+)

```rust
use rustlite::{Database, IsolationLevel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open("./my_database")?;
    
    // Begin a transaction with snapshot isolation (default)
    let mut txn = db.begin()?;
    
    // All reads see a consistent snapshot
    txn.put(b"account:alice".to_vec(), b"1000".to_vec())?;
    txn.put(b"account:bob".to_vec(), b"500".to_vec())?;
    
    // Transfer money atomically
    let alice = txn.get(b"account:alice")?.unwrap();
    let bob = txn.get(b"account:bob")?.unwrap();
    
    let alice_bal: i32 = String::from_utf8_lossy(&alice).parse()?;
    let bob_bal: i32 = String::from_utf8_lossy(&bob).parse()?;
    
    txn.put(b"account:alice".to_vec(), (alice_bal - 200).to_string().into_bytes())?;
    txn.put(b"account:bob".to_vec(), (bob_bal + 200).to_string().into_bytes())?;
    
    // Commit (or rollback on error)
    txn.commit()?;
    
    Ok(())
}
```

**Isolation Levels:**
- `ReadUncommitted`: Fastest, may see uncommitted changes
- `ReadCommitted`: See only committed data
- `RepeatableRead`: Snapshot isolation (default)
- `Serializable`: Strictest consistency

See [examples/transaction_demo.rs](https://github.com/VIRTUMEM-AI-LABS/rustlite/blob/main/crates/rustlite-api/examples/transaction_demo.rs) for comprehensive examples.

### Persistent Database (Recommended)

```rust
use rustlite::Database;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open a persistent database (creates directory if needed)
    let db = Database::open("./my_database")?;
    
    // Insert data - automatically persisted to disk
    db.put(b"user:1:name", b"Alice")?;
    db.put(b"user:1:email", b"alice@example.com")?;
    
    // Retrieve data
    if let Some(name) = db.get(b"user:1:name")? {
        println!("Name: {}", String::from_utf8_lossy(&name));
    }
    
    // Delete data
    db.delete(b"user:1:email")?;
    
    // Force sync to disk (optional - data is already durable via WAL)
    db.sync()?;
    
    Ok(())
}
```

### Data Persists Across Restarts

```rust
use rustlite::Database;

// First run - write data
let db = Database::open("./my_database")?;
db.put(b"counter", b"42")?;
drop(db);

// Later - data is still there!
let db = Database::open("./my_database")?;
assert_eq!(db.get(b"counter")?, Some(b"42".to_vec()));
```

### In-Memory Database (For Testing)

```rust
use rustlite::Database;

// Fast in-memory storage (data lost when program exits)
let db = Database::in_memory()?;
db.put(b"temp", b"data")?;
```

### Indexing for Fast Lookups (v0.3.0+)

```rust
use rustlite::{Database, IndexType};

let db = Database::in_memory()?;

// Create indexes
db.create_index("users_by_email", IndexType::Hash)?;  // O(1) lookups
db.create_index("users_by_name", IndexType::BTree)?;  // Range queries

// Index your data
db.put(b"user:1", b"alice@example.com")?;
db.index_insert("users_by_email", b"alice@example.com", 1)?;
db.index_insert("users_by_name", b"Alice", 1)?;

// Fast lookups
let user_ids = db.index_find("users_by_email", b"alice@example.com")?;
println!("Found user: {}", user_ids[0]); // Output: 1
```

### Relational Data with Foreign Keys

See [examples/relational_demo.rs](https://github.com/VIRTUMEM-AI-LABS/rustlite/blob/main/crates/rustlite-api/examples/relational_demo.rs) for a complete example showing:
- Users and Orders tables
- Foreign key relationships
- Primary and secondary indexes
- Join queries and cascade deletes

## 📦 Installation

### From crates.io

```bash
cargo add rustlite
```

### From source

```bash
git clone https://github.com/VIRTUMEM-AI-LABS/rustlite.git
cd rustlite
cargo build --release
```

## 🏗️ Architecture

RustLite is built with a modular LSM-tree architecture:

```
┌──────────────────────────────────────────────────────────┐
│                      Database API                        │
│                   (rustlite crate)                       │
├──────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │
│  │  Indexing   │  │   Memtable  │  │     WAL         │   │
│  │  B-Tree +   │  │  (BTreeMap) │  │ (Write Log)     │   │
│  │  Hash       │  └─────────────┘  └─────────────────┘   │
│  └─────────────┘                                          │
│  ┌─────────────────────────────────────────────────────┐ │
│  │           SSTable Storage + Compaction              │ │
│  │        (Sorted String Tables on Disk)               │ │
│  └─────────────────────────────────────────────────────┘ │
│  ┌─────────────┐                                          │
│  │  Snapshot   │  Point-in-time backups                   │
│  │  Manager    │                                          │
│  └─────────────┘                                          │
└──────────────────────────────────────────────────────────┘
```

**Key Components:**
- **Indexing**: B-Tree for range queries, Hash for O(1) lookups
- **Memtable**: In-memory sorted buffer for fast writes
- **WAL**: Write-ahead log for crash recovery and durability
- **SSTable**: Immutable on-disk sorted files
- **Compaction**: Background merging to reduce read amplification
- **Snapshot**: Point-in-time backups for disaster recovery

See [docs/ARCHITECTURE.md](https://github.com/VIRTUMEM-AI-LABS/rustlite/blob/main/docs/ARCHITECTURE.md) for technical details and [docs/README.md](https://github.com/VIRTUMEM-AI-LABS/rustlite/blob/main/docs/README.md) for the full documentation index.

## 📝 Production Logging (v0.6+)

RustLite includes production-grade structured logging:

```rust
use rustlite::logging::LogConfig;
use rustlite::Database;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (info level to file with daily rotation)
    let _guard = LogConfig::info()
        .with_file("./logs/rustlite.log")
        .init();
    
    let db = Database::open("./data")?;
    // All operations are now logged with structured context
    db.put(b"key", b"value")?;  // Logs: "Writing key-value pair" with sizes
    
    Ok(())
}
```

See [docs/LOGGING.md](https://github.com/VIRTUMEM-AI-LABS/rustlite/blob/main/docs/LOGGING.md) for comprehensive logging guide.

## 🤝 Contributing

We welcome contributions! Please see our [CONTRIBUTING.md](https://github.com/VIRTUMEM-AI-LABS/rustlite/blob/main/docs/CONTRIBUTING.md) for guidelines.

Key areas where we need help:
- Query optimizer and query planner
- Performance benchmarking and optimization
- Documentation and examples
- Platform-specific optimizations
- Advanced indexing (full-text search, spatial indexes)

## 📋 Requirements

- Rust 1.81.0 or later
- Supported platforms: Linux, macOS, Windows

## 🧪 Testing

```bash
# Run all tests (48 tests: 39 lib + 9 aggregate)
cargo test --workspace

# Run with logging
RUST_LOG=debug cargo test

# Run examples
cargo run --example persistent_demo
cargo run --example relational_demo
cargo run --example aggregate_demo     # NEW: GROUP BY and aggregates

# Run benchmarks
cargo bench
```

## 📊 Benchmarks

Performance benchmarks will be published as the project matures. Early benchmarks show:

- Sequential writes: TBD
- Random reads: TBD
- Concurrent operations: TBD

## 🔒 Security

RustLite takes security seriously. Please report any security vulnerabilities to [security@rustlite.dev](mailto:security@rustlite.dev).

## 📜 License

This project is licensed under the Apache License, Version 2.0 ([LICENSE](https://github.com/VIRTUMEM-AI-LABS/rustlite/blob/main/LICENSE) or http://www.apache.org/licenses/LICENSE-2.0).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in RustLite by you shall be under the terms and conditions of the Apache License, Version 2.0, without any additional terms or conditions.

## 🌟 Acknowledgments

RustLite is inspired by excellent databases like SQLite, LevelDB, and RocksDB.

## 📞 Contact & Community

- **GitHub**: [github.com/VIRTUMEM-AI-LABS/rustlite](https://github.com/VIRTUMEM-AI-LABS/rustlite)
- **Crates.io**: [crates.io/crates/rustlite](https://crates.io/crates/rustlite)
- **Documentation**: [docs.rs/rustlite](https://docs.rs/rustlite)
- **Discord**: Coming soon
- **Website**: [rustlite.dev](https://rustlite.dev) (planned)

## 🗺️ Status

**Current Status**: Active development (v0.3.0)

RustLite is in active development with persistent storage, WAL, and indexing capabilities. Not yet production-ready, but suitable for experimentation and development. Star the repo to follow our progress toward v1.0!

---

Made with ❤️ by the RustLite community
