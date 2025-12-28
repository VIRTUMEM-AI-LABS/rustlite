# RustLite


<p align="right">
    <img src="assets/logo-wordmark.svg" alt="RustLite logo" width="100" />
</p>

[![Crates.io](https://img.shields.io/crates/v/rustlite.svg)](https://crates.io/crates/rustlite)
[![Documentation](https://docs.rs/rustlite/badge.svg)](https://docs.rs/rustlite)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Build Status](https://github.com/VIRTUMEM-AI-LABS/rustlite/workflows/CI/badge.svg)](https://github.com/VIRTUMEM-AI-LABS/rustlite/actions)

[![Changelog](https://img.shields.io/badge/Changelog-docs/CHANGELOG.md-blue.svg)](docs/CHANGELOG.md)

**RustLite** is a lightweight, high-performance embedded database written entirely in Rust. Designed for applications that need a fast, reliable, and embeddable storage solution with ACID guarantees.

## 🎯 Vision

RustLite aims to be the go-to embedded database for Rust applications, combining:

- **Performance**: Zero-copy operations, memory-mapped I/O, and efficient data structures
- **Reliability**: Full ACID compliance with write-ahead logging and crash recovery
- **Simplicity**: Single-file deployment, zero configuration, intuitive API
- **Safety**: Memory-safe by design using Rust's type system and ownership model

## ✨ Features

### Current (v0.3.0)
- ✅ **Persistent storage** with LSM-tree architecture
- ✅ **Write-Ahead Logging (WAL)** for crash recovery
- ✅ **SSTable compaction** for optimized disk usage
- ✅ **Snapshot backups** for point-in-time recovery
- ✅ **B-Tree indexing** for range queries and ordered lookups
- ✅ **Hash indexing** for O(1) exact-match lookups
- ✅ Thread-safe concurrent access
- ✅ Simple, ergonomic API

### Roadmap
- 🔄 **v0.4**: SQL-like query engine
- 🔄 **v0.5**: Full transaction support with MVCC
- 🔄 **v1.0**: Production-ready with ACID guarantees

See [docs/ROADMAP.md](docs/ROADMAP.md) for detailed plans.

## 🚀 Quick Start

Add RustLite to your `Cargo.toml`:

```toml
[dependencies]
rustlite = "0.3"
```

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

See [examples/relational_demo.rs](crates/rustlite-api/examples/relational_demo.rs) for a complete example showing:
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

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for technical details and [docs/README.md](docs/README.md) for the full documentation index.

## 🤝 Contributing

We welcome contributions! Please see our [CONTRIBUTING.md](docs/CONTRIBUTING.md) for guidelines.

Key areas where we need help:
- Query optimizer and query planner
- Performance benchmarking and optimization
- Documentation and examples
- Platform-specific optimizations
- Advanced indexing (full-text search, spatial indexes)

## 📋 Requirements

- Rust 1.70.0 or later
- Supported platforms: Linux, macOS, Windows

## 🧪 Testing

```bash
# Run all tests (126+ tests)
cargo test --workspace

# Run with logging
RUST_LOG=debug cargo test

# Run examples
cargo run --example persistent_demo
cargo run --example relational_demo

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

This project is licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0).

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
