# RustLite Architecture

This document describes the technical architecture of RustLite, a lightweight embedded database written in Rust.

## Design Philosophy

RustLite is built on these core principles:

1. **Safety First**: Leverage Rust's type system to eliminate entire classes of bugs
2. **Performance**: Zero-copy operations, efficient data structures, and minimal overhead
3. **Simplicity**: Clear APIs, minimal configuration, single-file deployment
4. **Reliability**: ACID guarantees, crash recovery, data integrity
5. **Modularity**: Pluggable components for different use cases

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Application Layer                       │
│                    (User Application Code)                   │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                       Public API Layer                       │
│  Database, Transaction, Query, Index Management APIs         │
└───────────────────────────┬─────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
┌───────▼────────┐ ┌────────▼────────┐ ┌───────▼────────┐
│  Query Engine  │ │  Transaction    │ │ Index Manager  │
│                │ │   Coordinator   │ │                │
│  - Parser      │ │  - MVCC         │ │  - B-Tree      │
│  - Planner     │ │  - Isolation    │ │  - Hash        │
│  - Executor    │ │  - Deadlock     │ │  - Full-text   │
└───────┬────────┘ └────────┬────────┘ └───────┬────────┘
        │                   │                   │
        └───────────────────┼───────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                     Storage Engine                           │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Memtable   │  │  SSTable     │  │    Cache     │     │
│  │  (In-Memory) │  │ (On-Disk)    │  │   (LRU/LFU)  │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │          Write-Ahead Log (WAL)                       │  │
│  │          Durability & Crash Recovery                 │  │
│  └──────────────────────────────────────────────────────┘  │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                      File System Layer                       │
│    Memory-mapped I/O, Direct I/O, Fsync, File Management    │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Public API Layer

The primary interface for users of RustLite.

**Key Types:**
- `Database` - Main database handle
- `Transaction` - Transaction handle
- `Query` - Query builder
- `Index` - Index management

**Responsibilities:**
- Validate user input
- Coordinate between subsystems
- Provide ergonomic, safe API
- Handle thread synchronization

### 2. Query Engine (v0.4+)

Parses, plans, and executes queries.

**Components:**

**Parser:**
- Lexical analysis (tokenization)
- Syntax parsing (SQL-like grammar)
- AST (Abstract Syntax Tree) generation

**Planner:**
- Logical query plan generation
- Query optimization
- Index selection
- Cost-based optimization

**Executor:**
- Physical query execution
- Iterator-based processing
- Predicate pushdown
- Result materialization

**Example Query Flow:**
```
SQL → Tokens → AST → Logical Plan → Optimized Plan → Physical Plan → Results
```

### 3. Transaction Coordinator (v0.5+)

Manages concurrent access and ACID properties.

**MVCC (Multi-Version Concurrency Control):**
- Each transaction sees a consistent snapshot
- Writes create new versions
- Old versions garbage collected when no longer needed

**Components:**
- **Transaction Manager**: Begin, commit, rollback
- **Version Manager**: Track object versions
- **Lock Manager**: Detect and resolve conflicts
- **Deadlock Detector**: Timeout-based detection

**Isolation Levels:**
```rust
pub enum IsolationLevel {
    ReadUncommitted,  // Dirty reads allowed
    ReadCommitted,    // No dirty reads
    RepeatableRead,   // No phantom reads
    Serializable,     // Full isolation
}
```

### 4. Index Manager (v0.3+)

Provides fast data access through various index types.

**Index Types:**

**B-Tree Index:**
- Ordered keys
- Range queries
- Prefix scans
- O(log n) lookup

**Hash Index:**
- Unordered keys
- Exact-match queries only
- O(1) average lookup
- Lower memory overhead

**Full-Text Index (v0.6+):**
- Text search
- Relevance ranking
- Phrase queries

### 5. Storage Engine

The core persistent storage layer.

#### Memtable (v0.2+)

In-memory write buffer.

**Implementation:**
- Skip list or B-Tree structure
- Configurable size threshold
- Write-optimized
- Concurrent reads during writes

**Operations:**
- Insert: O(log n)
- Lookup: O(log n)
- Scan: O(log n + k)

#### SSTable (Sorted String Table) (v0.2+)

Immutable on-disk data structure.

**File Format:**
```
┌──────────────────────────────────────┐
│           Data Blocks                 │
│  ┌────────────────────────────────┐  │
│  │  Key-Value Pairs (sorted)      │  │
│  │  Optional compression          │  │
│  └────────────────────────────────┘  │
├──────────────────────────────────────┤
│         Index Block                   │
│  Block offsets and first keys        │
├──────────────────────────────────────┤
│        Bloom Filter                   │
│  Probabilistic membership test        │
├──────────────────────────────────────┤
│          Metadata                     │
│  Version, checksums, stats            │
└──────────────────────────────────────┘
```

**Features:**
- Immutable (never modified after creation)
- Compressed blocks (LZ4, Snappy, Zstd)
- Bloom filters for fast negative lookups
- Block-based organization for efficient I/O

#### LSM-Tree Organization (v0.2+)

Log-Structured Merge Tree for write optimization.

**Levels:**
```
Level 0: [SSTable 1] [SSTable 2] [SSTable 3]  ← Recent, may overlap
Level 1: [SSTable 4] [SSTable 5]              ← Partially merged
Level 2: [SSTable 6]                          ← Fully merged
...
Level N: [SSTable 7]                          ← Largest, oldest
```

**Compaction:**
- Merges SSTables from upper to lower levels
- Removes deleted/overwritten keys
- Maintains sorted order
- Balances read and write amplification

**Compaction Strategies:**
- Size-tiered compaction
- Leveled compaction
- Time-window compaction

#### Write-Ahead Log (WAL) (v0.2+)

Ensures durability and enables crash recovery.

**Log Entry Format:**
```rust
struct WalEntry {
    sequence_number: u64,
    entry_type: WalEntryType,  // Put, Delete, Commit, Rollback
    key: Vec<u8>,
    value: Option<Vec<u8>>,
    checksum: u32,
}
```

**Recovery Process:**
1. Open WAL file
2. Replay entries in sequence
3. Rebuild memtable
4. Resume normal operations

**Optimization:**
- Group commit (batch multiple transactions)
- Parallel WAL writes
- Periodic checkpointing

### 6. Cache Layer (v0.3+)

Reduces disk I/O for frequently accessed data.

**Cache Types:**

**Block Cache:**
- Caches SSTable data blocks
- LRU or LFU eviction policy
- Configurable size limit

**Row Cache:**
- Caches individual key-value pairs
- Useful for read-heavy workloads

**Implementation:**
```rust
struct Cache<K, V> {
    map: HashMap<K, CacheEntry<V>>,
    policy: EvictionPolicy,
    max_size: usize,
}
```

### 7. File System Layer

Abstracts OS-specific file operations.

**Features:**
- Memory-mapped files for read-heavy workloads
- Direct I/O to bypass OS cache
- Fsync/fdatasync for durability
- File lock for single-writer guarantee
- Cross-platform support (Linux, macOS, Windows)

## Data Flow Examples

### Write Path

```
1. Application calls db.put(key, value)
2. Append to WAL (for durability)
3. Insert into Memtable
4. Return success to application
5. [Background] When Memtable full:
   a. Create immutable Memtable
   b. Start new Memtable for writes
   c. Flush immutable Memtable to SSTable
   d. Trigger compaction if needed
```

### Read Path

```
1. Application calls db.get(key)
2. Check Memtable (newest data)
3. Check Immutable Memtables
4. Check Block Cache
5. For each SSTable (Level 0 → Level N):
   a. Check Bloom filter (fast negative lookup)
   b. If might exist, check index
   c. If found, read data block
   d. Return value if found
6. Return None if not found
```

### Query Path (v0.4+)

```
1. Application calls db.query(sql)
2. Parse SQL → AST
3. Generate logical plan
4. Optimize plan (index selection, predicate pushdown)
5. Generate physical plan
6. Execute plan:
   a. Use indexes if available
   b. Scan SSTables/Memtable
   c. Apply filters and transformations
   d. Return result iterator
```

## Concurrency Model

### Thread Safety

**Read Concurrency:**
- Multiple concurrent readers
- Lock-free or read-write lock based on component
- MVCC for transaction isolation

**Write Serialization:**
- Single writer to WAL and Memtable
- Background threads for compaction
- Atomic operations for metadata updates

### Synchronization Primitives

```rust
// Database-level
Arc<RwLock<Database>>  // Multiple readers, single writer

// Memtable
Arc<SkipList>          // Lock-free reads

// WAL
Mutex<WalWriter>       // Serialized writes

// Transaction
Arc<TransactionState>  // MVCC versioning
```

## Memory Management

### Buffer Pool

Manages fixed-size memory budget across components:

```rust
struct BufferPool {
    memtable_budget: usize,
    block_cache_budget: usize,
    write_buffer_budget: usize,
}
```

### Memory Estimation

- Memtable: ~1-64 MB default
- Block cache: ~8-256 MB default
- Write buffer: ~4-32 MB default

Total: Typically 16-256 MB for embedded use cases

## File Format Versioning

**Version Header:**
```
Magic Number: 0x52_55_53_54_4C_49_54_45 ("RUSTLITE")
Version: u16 (major.minor)
Features: u64 (feature flags)
```

**Upgrade Path:**
- Detect older versions on open
- Auto-upgrade if possible
- Reject incompatible versions

## Error Handling

**Error Categories:**
- I/O errors (disk full, permission denied)
- Corruption errors (checksum mismatch)
- Transaction errors (deadlock, conflict)
- API misuse (invalid parameters)

**Recovery Strategy:**
- Detect errors early
- Fail fast on corruption
- Provide detailed error context
- Enable graceful degradation where possible

## Performance Characteristics

### Time Complexity

| Operation | Best Case | Average Case | Worst Case |
|-----------|-----------|--------------|------------|
| Put       | O(1)*     | O(log n)     | O(n)**     |
| Get       | O(1)*     | O(log n × L) | O(n × L)   |
| Delete    | O(1)*     | O(log n)     | O(n)**     |
| Range     | O(k)      | O(log n + k) | O(n)       |

\* Memtable hit, amortized  
\*\* Compaction trigger  
L = Number of LSM levels  
k = Number of results

### Space Amplification

- Write amplification: 5-10x (LSM-tree characteristic)
- Space amplification: 1.2-2x (depending on compaction)
- Read amplification: O(log n) (number of levels checked)

## Security Considerations (v1.0+)

- File permissions validation
- Input sanitization
- Buffer overflow protection (Rust's safety)
- Denial of service mitigations
- Encryption at rest (optional)
- Audit logging

## Testing Strategy

- Unit tests for each module
- Integration tests for end-to-end flows
- Property-based testing (quickcheck, proptest)
- Fuzzing for parser and file formats
- Stress testing and chaos engineering
- Performance regression tests

## Benchmarking

Compare against:
- SQLite
- RocksDB
- LMDB
- sled

Metrics:
- Throughput (ops/sec)
- Latency (p50, p99, p99.9)
- Memory usage
- Disk I/O
- CPU utilization

---

**Last Updated**: October 25, 2025

This architecture is aspirational and will evolve as development progresses. Early versions will implement simplified versions of these components.
