# RustLite Roadmap

This document outlines the development roadmap for RustLite. Our goal is to build a production-ready embedded database that rivals SQLite in reliability while leveraging Rust's safety guarantees.

## Release Philosophy

- **Even versions (0.2, 0.4, 0.6, etc.)**: Major features, may introduce breaking changes
- **Odd versions (0.3, 0.5, 0.7, etc.)**: Refinements, performance, stability
- **Version 1.0**: Production-ready with stability guarantees

## Version History & Roadmap

### ✅ v0.1.0 - Foundation (Current)
**Release Date**: Q4 2025
**Status**: Released

**Features:**
- ✅ In-memory key-value store
- ✅ Thread-safe concurrent access using RwLock
- ✅ Basic CRUD operations (put, get, delete)
- ✅ Simple, ergonomic API
- ✅ Comprehensive documentation
- ✅ Apache-2.0 licensing

**Limitations:**
- No persistence (data lost on restart)
- No transaction support
- No indexing
- No query capabilities

---

### 🔄 v0.2.0 - Persistence & Durability
**Target Date**: Q1 2026
**Status**: Planning

**Core Features:**
- [ ] File-based persistent storage
- [ ] Write-Ahead Logging (WAL) for crash recovery
- [ ] Atomic flush to disk
- [ ] Background compaction
- [ ] Configurable sync modes (sync, async, none)
- [ ] Database snapshots

**Storage Engine:**
- [ ] Log-Structured Merge (LSM) tree implementation
- [ ] Memtable with configurable size
- [ ] SSTable format with compression
- [ ] Bloom filters for read optimization
- [ ] Level-based compaction strategy

**API Changes:**
- `Database::open(path)` - Open persistent database
- `Database::flush()` - Force flush to disk
- `Database::sync()` - Fsync guarantee
- `Database::snapshot()` - Create snapshot

**Performance Targets:**
- 100K+ writes/sec (sequential)
- 50K+ writes/sec (random)
- 200K+ reads/sec (cached)

---

### 🔄 v0.3.0 - Indexing & Performance
**Target Date**: Q2 2026
**Status**: Planning

**Core Features:**
- [ ] B-Tree index implementation
- [ ] Hash index for exact-match queries
- [ ] Composite (multi-column) indexes
- [ ] Index creation and management API
- [ ] Automatic index selection for queries

**Optimizations:**
- [ ] Memory-mapped I/O for read-heavy workloads
- [ ] Prefix compression in B-Tree
- [ ] Cache replacement policies (LRU, LFU)
- [ ] Read-ahead optimization
- [ ] SIMD-accelerated operations where applicable

**API Changes:**
- `Database::create_index()` - Create index
- `Database::drop_index()` - Remove index
- `Database::list_indexes()` - List all indexes
- Range query support: `get_range(start, end)`

**Performance Targets:**
- Sub-millisecond indexed lookups
- 500K+ indexed reads/sec

---

### 🔄 v0.4.0 - Query Engine
**Target Date**: Q3 2026
**Status**: Planning

**Core Features:**
- [ ] SQL-like query parser
- [ ] Query planner and optimizer
- [ ] Support for WHERE, ORDER BY, LIMIT
- [ ] Aggregation functions (COUNT, SUM, AVG, MIN, MAX)
- [ ] JOIN operations (INNER, LEFT, RIGHT)
- [ ] Subqueries

**Query Language Example:**
```sql
SELECT key, value 
FROM store 
WHERE key LIKE 'user:%' 
ORDER BY value DESC 
LIMIT 10
```

**API Changes:**
- `Database::query(sql)` - Execute SQL-like query
- `Database::prepare(sql)` - Prepared statements
- Query result iterators
- Parameter binding for prepared statements

**Optimizations:**
- Query plan caching
- Statistics-based optimization
- Predicate pushdown
- Index-aware query planning

---

### 🔄 v0.5.0 - Transactions & MVCC
**Target Date**: Q4 2026
**Status**: Planning

**Core Features:**
- [ ] Full ACID transaction support
- [ ] Multi-Version Concurrency Control (MVCC)
- [ ] Isolation levels: Read Committed, Repeatable Read, Serializable
- [ ] Optimistic locking
- [ ] Deadlock detection and resolution
- [ ] Transaction rollback and recovery

**API Changes:**
```rust
let tx = db.begin_transaction(IsolationLevel::Serializable)?;
tx.put(b"key", b"value")?;
tx.commit()?; // or tx.rollback()?
```

**Concurrency:**
- Lock-free reads during writes
- Snapshot isolation
- Non-blocking readers
- Write conflict detection

**Performance Targets:**
- 50K+ transactions/sec
- Minimal overhead for read-only transactions

---

### 🔄 v0.6.0 - Advanced Features
**Target Date**: Q1 2027
**Status**: Future

**Planned Features:**
- [ ] Full-text search index
- [ ] Geospatial index support
- [ ] JSON/BSON document storage
- [ ] Schema definitions and validation
- [ ] Triggers and stored procedures
- [ ] Replication (master-slave)
- [ ] Backup and restore utilities

---

### 🔄 v0.7.0 - Stability & Performance
**Target Date**: Q2 2027
**Status**: Future

**Focus Areas:**
- [ ] Extensive fuzzing and property-based testing
- [ ] Memory leak detection and fixes
- [ ] Performance profiling and optimization
- [ ] Comprehensive benchmarks vs SQLite, RocksDB
- [ ] Platform-specific optimizations (Linux, macOS, Windows)
- [ ] ARM and RISC-V support

---

### 🎯 v1.0.0 - Production Ready
**Target Date**: Q3 2027
**Status**: Future

**Stability Guarantees:**
- [ ] Semantic versioning commitment
- [ ] Stable file format with upgrade path
- [ ] Backward-compatible API
- [ ] Long-term support (LTS)
- [ ] Security audit
- [ ] Production documentation

**Compliance:**
- [ ] ACID guarantees formally verified
- [ ] Crash recovery tested extensively
- [ ] Data corruption detection and prevention
- [ ] Performance regression testing

**Ecosystem:**
- [ ] Language bindings (Python, Node.js, C)
- [ ] Integration with popular frameworks
- [ ] Cloud provider support
- [ ] Migration tools from SQLite/LevelDB

---

## Beyond 1.0

### Future Possibilities
- Distributed transactions
- Sharding and partitioning
- Column-oriented storage option
- Time-series optimizations
- Embedded analytics engine
- Graph database capabilities
- Vector search for ML applications

---

## How to Contribute

See specific roadmap items you're interested in? We'd love your help!

1. Check [GitHub Issues](https://github.com/rustlite/rustlite/issues) for related discussions
2. Comment on issues you want to work on
3. Read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines
4. Submit PRs with clear descriptions

---

## Versioning Notes

**Pre-1.0 Versions:**
- May introduce breaking API changes
- File format may change between versions
- Focus on feature development and experimentation

**Post-1.0 Versions:**
- Semantic versioning strictly followed
- Breaking changes only in major versions
- File format backward compatibility guaranteed
- Stability and reliability prioritized

---

**Last Updated**: October 25, 2025

This roadmap is aspirational and subject to change based on community feedback, resource availability, and technical discoveries during development.
