# Logging Guide

RustLite provides production-grade structured logging using the `tracing` framework. This guide covers how to configure and use logging for monitoring and debugging.

## Quick Start

```rust
use rustlite::logging::LogConfig;
use rustlite::Database;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (keeps logs for application lifetime)
    let _guard = LogConfig::info().init();
    
    let db = Database::open("./data")?;
    // Operations are now logged automatically
    db.put(b"key", b"value")?;
    
    Ok(())
}
```

## Log Levels

RustLite supports standard log levels from least to most verbose:

- **ERROR**: Critical failures (corrupted data, system errors)
- **WARN**: Degraded performance, non-critical issues
- **INFO**: Important lifecycle events (database open/close, index creation)
- **DEBUG**: Detailed operation logs (each put/get/delete with sizes)
- **TRACE**: Extremely verbose internal details

### Setting Log Level

```rust
// Production: Only show important events
let _guard = LogConfig::info().init();

// Development: Show detailed operations
let _guard = LogConfig::debug().init();

// Troubleshooting: Maximum verbosity
let _guard = LogConfig::with_level("trace").init();
```

## Output Destinations

### Console Output (Default)

```rust
// Logs to stdout
let _guard = LogConfig::info().init();
```

### File Output with Daily Rotation

```rust
// Logs to ./logs/rustlite.log with daily rotation
let _guard = LogConfig::info()
    .with_file("./logs/rustlite.log")
    .init();

// Files are automatically rotated:
// - rustlite.log.2025-01-15
// - rustlite.log.2025-01-16
// - rustlite.log (current)
```

### Both Console and File

```rust
// Logs to both stdout and file
let _guard = LogConfig::info()
    .with_both("./logs/rustlite.log")
    .init();
```

## Log Formats

### Pretty Format (Default)

Human-readable with colors and indentation:

```text
2025-12-28T14:05:28.919878Z  INFO rustlite: Creating in-memory RustLite database
  at crates/rustlite-api/src/lib.rs:232
```

### Compact Format

Single-line format for easier parsing:

```rust
let _guard = LogConfig::info()
    .with_format(LogFormat::Compact)
    .init();
```

Output:
```text
2025-12-28T14:05:28.919878Z INFO rustlite Creating in-memory RustLite database
```

## Structured Logging

RustLite logs include structured context for filtering and analysis:

```text
DEBUG rustlite: Writing key-value pair
  at src/lib.rs:282
  in rustlite::put with key_len: 10, value_len: 22
```

Key structured fields:
- **key_len**: Size of key in bytes
- **value_len**: Size of value in bytes
- **sql_len**: SQL query length
- **isolation**: Transaction isolation level
- **name**: Index name
- **index_type**: Hash or BTree

## What Gets Logged

### INFO Level

```rust
let _guard = LogConfig::info().init();
```

Logs lifecycle events:
- Database open/close
- Index creation
- Transaction begin
- Compaction completion

### DEBUG Level

```rust
let _guard = LogConfig::debug().init();
```

Logs all INFO events plus:
- Every put/get/delete operation with sizes
- Query execution with SQL
- WAL record appends
- Compaction progress

## Environment Variable Override

Set `RUST_LOG` environment variable to override config:

```bash
# Windows PowerShell
$env:RUST_LOG="debug"
cargo run

# Unix/Linux
RUST_LOG=debug cargo run

# Filter by module
RUST_LOG=rustlite=debug,rustlite_wal=trace cargo run
```

## Production Best Practices

### 1. Keep the Guard Alive

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CORRECT: Guard lives for entire application
    let _guard = LogConfig::info()
        .with_file("./logs/app.log")
        .init();
    
    run_application()?;
    Ok(())
}

// INCORRECT: Guard dropped immediately, logging stops
fn bad_example() {
    {
        let _guard = LogConfig::info().init();
    } // Guard dropped here!
    
    // No more logs after this point
}
```

### 2. Use INFO Level in Production

```rust
// Production: Minimal overhead, captures important events
let _guard = LogConfig::info()
    .with_file("/var/log/rustlite/app.log")
    .init();
```

### 3. Rotate Log Files

Daily rotation is automatic with file output:

```rust
let _guard = LogConfig::info()
    .with_file("./logs/rustlite.log")  // Automatically rotates daily
    .init();
```

### 4. Monitor Log Output

```rust
// In monitoring systems, parse structured logs
// Example log line:
// INFO rustlite: Creating index with name: users, index_type: Hash
```

## Performance Impact

Logging overhead by level (approximate):

| Level | Overhead | Use Case |
|-------|----------|----------|
| ERROR/WARN | <1% | Always enabled |
| INFO | 1-2% | Production default |
| DEBUG | 5-10% | Development/staging |
| TRACE | 20%+ | Troubleshooting only |

The `tracing` framework has near-zero overhead when log levels are disabled.

## Examples

### Web Application

```rust
use rustlite::logging::LogConfig;
use rustlite::Database;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Production logging: info to file with rotation
    let _guard = LogConfig::info()
        .with_file("/var/log/myapp/rustlite.log")
        .init();
    
    let db = Database::open("/var/lib/myapp/data")?;
    
    // All operations logged automatically
    start_web_server(db)?;
    
    Ok(())
}
```

### Development Mode

```rust
use rustlite::logging::LogConfig;
use rustlite::Database;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Development: debug to console with pretty colors
    let _guard = LogConfig::debug().init();
    
    let db = Database::in_memory()?;
    
    // Detailed logs for debugging
    db.put(b"test", b"data")?;
    
    Ok(())
}
```

### CI/CD Pipeline

```rust
use rustlite::logging::{LogConfig, LogFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CI: Compact format for log aggregation
    let _guard = LogConfig::debug()
        .with_format(LogFormat::Compact)
        .init();
    
    run_tests()?;
    
    Ok(())
}
```

## Troubleshooting

### No Logs Appearing

1. Check that guard is kept alive:
   ```rust
   let _guard = LogConfig::info().init();  // Don't drop this!
   ```

2. Check log level:
   ```rust
   // DEBUG won't show with info()
   let _guard = LogConfig::debug().init();
   ```

3. Check environment variable:
   ```bash
   echo $RUST_LOG  # Should be empty or match your config
   ```

### Too Many Logs

```rust
// Reduce to INFO level
let _guard = LogConfig::info().init();

// Or use environment variable per-module filtering
// RUST_LOG=rustlite=info,rustlite_storage=warn
```

### File Permissions

```rust
// Ensure directory exists and is writable
std::fs::create_dir_all("./logs")?;

let _guard = LogConfig::info()
    .with_file("./logs/rustlite.log")
    .init();
```

## See Also

- [API Stability Guide](API_STABILITY.md)
- [Production Readiness Plan](V1_PRODUCTION_PLAN.md)
- [Architecture Documentation](ARCHITECTURE.md)
