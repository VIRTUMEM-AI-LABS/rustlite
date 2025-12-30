# Changelog

All notable changes to RustLite will be documented in this file.

The format is based on "Keep a Changelog" and this project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### In Progress
- Future enhancements

## [0.7.0] - 2025-12-30

### Added
- **Aggregate Functions**: Complete SQL aggregate support
  - `COUNT(*)`: Count all rows (including NULLs)
  - `COUNT(column)`: Count non-null values in specified column
  - `SUM(column)`: Sum numeric values
  - `AVG(column)`: Calculate average of numeric values
  - `MIN(column)`: Find minimum value (works with integers, floats, strings)
  - `MAX(column)`: Find maximum value (works with integers, floats, strings)
- **GROUP BY Clause**: Multi-column grouping support
  - Group rows by one or more columns
  - Compute aggregates per group
  - Custom `GroupKey` wrapper for hashable float support
- **HAVING Clause**: Post-aggregation filtering
  - Filter grouped results based on conditions
  - Works with column comparisons (aggregate support in expressions coming soon)
- **Query Planner Improvements**: Optimized physical plan generation
  - Aggregate and GroupBy physical operators
  - Proper column projection for aggregation queries
  - Efficient grouping with HashMap-based implementation
- **Comprehensive Testing**: 9 new aggregate tests
  - Simple aggregates (COUNT, SUM, AVG, MIN, MAX)
  - COUNT(*) vs COUNT(column) with NULL handling
  - GROUP BY with single/multiple columns
  - HAVING clause filtering
  - All 48 workspace tests passing
- **Examples**: `aggregate_demo.rs` demonstrating:
  - All aggregate functions with real data
  - GROUP BY with multiple aggregates
  - HAVING clause usage
  - Real-world analytics use case (customer orders)

### Fixed
- README links now work correctly on crates.io (absolute GitHub URLs)
- Query planner no longer projects aggregate columns prematurely
- `compute_aggregate` robustly finds column indices across all rows

## [0.6.0] - 2025-01-XX

### Added
- Published to crates.io (all 5 crates: rustlite-core, rustlite-wal, rustlite-storage, rustlite-snapshot, rustlite)

## [0.5.0] - 2025-01-XX

### Added
- **Full MVCC Transaction Support**: Complete Multi-Version Concurrency Control implementation
  - `VersionedValue`: Version storage with timestamp-based visibility
  - `VersionChain`: Per-key version history with garbage collection
  - `MVCCStorage`: Thread-safe versioned storage with RwLock
  - `TransactionManager`: Atomic transaction ID and timestamp generation
  - `Transaction`: Snapshot isolation with read-your-own-writes semantics
- **Isolation Levels**: Four standard isolation levels
  - `ReadUncommitted`: Fastest, may see uncommitted changes
  - `ReadCommitted`: See only committed data
  - `RepeatableRead`: Snapshot isolation (default)
  - `Serializable`: Strictest consistency guarantees
- **Transaction API**: New methods on `Database`
  - `begin()`: Start transaction with default RepeatableRead isolation
  - `begin_transaction(isolation)`: Start transaction with custom isolation level
  - `gc()`: Garbage collect old versions
- **Transaction Operations**: Full ACID guarantees
  - `get()`, `put()`, `delete()`: Transactional operations
  - `scan()`: Prefix scanning with snapshot isolation
  - `commit()`: Atomically commit all changes
  - `rollback()`: Discard all changes
- **Comprehensive Testing**: 11 MVCC tests covering:
  - Basic read/write operations
  - Snapshot isolation verification
  - Rollback functionality
  - Concurrent writes (5 threads)
  - Prefix scanning with versions
  - Garbage collection
  - Read-your-own-writes
  - All 4 isolation levels
  - Transaction ID uniqueness
  - Phantom read prevention
- **Examples**: `transaction_demo.rs` demonstrating:
  - Basic transaction workflow
  - Rollback scenarios
  - Snapshot isolation
  - Bank account transfer with ACID
  - Prefix scanning
  - Custom isolation levels
  - Garbage collection

### Changed
- Updated architecture documentation with detailed MVCC design
- Enhanced README with transaction examples
- Improved error handling for transaction operations

### Performance
- Non-blocking reads: Readers never block writers
- Thread-safe concurrent access with RwLock
- Efficient version chain management
- Atomic timestamp generation with AtomicU64

## [0.4.0] - 2025-01-XX

### Added
- **SQL-like Query Engine**: Parse and execute queries
  - SELECT statements with column selection
  - WHERE clause with comparison operators
  - LIMIT clause for result pagination
  - ORDER BY support
- Query planning and optimization
- Execution context for query evaluation
- Comprehensive query tests
- Query examples in documentation

## [0.1.0] - 2025-10-25
### Added
- Initial in-memory key-value store (thread-safe) with basic API: `Database::new`, `put`, `get`, `delete`, `len`.
- In-memory placeholder implementations for WAL, Transactions, and Query engine to enable early testing and packaging.
- Documentation scaffolding for `index` and `storage` modules.
- `CONTRIBUTING.md` and repository `hooks/` scripts to protect against accidental pushes.

### Changed
- Project repository metadata and packaging prepared for crates.io (packaging verified locally and dry-run publish executed).

### Notes
- This release is an early development snapshot. Persistent storage, durable WAL, proper transactions, and query engine are planned for subsequent releases (see ROADMAP.md).

## [Unreleased] -> 0.1.0
- Initial release notes collected.
