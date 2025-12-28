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

### Current (v0.1.0)
- ✅ In-memory key-value store
- ✅ Thread-safe concurrent access
- ✅ Simple, ergonomic API

### Roadmap
- 🔄 **v0.2**: Persistent storage with Write-Ahead Logging (WAL)
- 🔄 **v0.3**: B-Tree and Hash indexing
- 🔄 **v0.4**: SQL-like query engine
- 🔄 **v0.5**: Full transaction support with MVCC
- 🔄 **v1.0**: Production-ready with ACID guarantees

See [docs/ROADMAP.md](docs/ROADMAP.md) for detailed plans.

## 🚀 Quick Start

Add RustLite to your `Cargo.toml`:

```toml
[dependencies]
rustlite = "0.1.0"
```

### Basic Usage

```rust
use rustlite::Database;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new database
    let db = Database::new()?;
    
    // Insert data
    db.put(b"user:1:name", b"Alice")?;
    db.put(b"user:1:email", b"alice@example.com")?;
    
    // Retrieve data
    if let Some(name) = db.get(b"user:1:name")? {
        println!("Name: {}", String::from_utf8_lossy(&name));
    }
    
    // Delete data
    db.delete(b"user:1:email")?;
    
    Ok(())
}
```

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

RustLite is built with a modular architecture:

- **Storage Engine**: Pluggable backends (LSM-tree, B-Tree)
- **Transaction Layer**: MVCC-based isolation
- **Query Engine**: SQL-like query compilation and execution
- **WAL**: Write-ahead logging for durability
- **Index System**: Multiple index types for efficient queries

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for technical details and [docs/README.md](docs/README.md) for the full documentation index.

## 🤝 Contributing

We welcome contributions! Please see our [CONTRIBUTING.md](docs/CONTRIBUTING.md) for guidelines.

Key areas where we need help:
- Core storage engine implementation
- Query optimizer
- Performance benchmarking
- Documentation and examples
- Platform-specific optimizations

## 📋 Requirements

- Rust 1.70.0 or later
- Supported platforms: Linux, macOS, Windows

## 🧪 Testing

```bash
# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo test

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

**Current Status**: Early development (v0.1.0)

RustLite is in active development and not yet ready for production use. We're working hard to deliver a stable v1.0 release. Star the repo to follow our progress!

---

Made with ❤️ by the RustLite community
