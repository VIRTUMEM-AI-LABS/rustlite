use rustlite::logging::LogConfig;
use rustlite::Database;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize debug-level logging
    let _guard = LogConfig::debug().init();

    println!("=== RustLite Debug Logging Demo ===\n");

    let db = Database::in_memory()?;

    println!("\n1. Writing data with debug logs...");
    db.put(b"user:alice", b"Alice Smith - Engineer")?;
    db.put(b"user:bob", b"Bob Jones - Manager")?;

    println!("\n2. Reading data with debug logs...");
    if let Some(value) = db.get(b"user:alice")? {
        println!("Found: {}", String::from_utf8_lossy(&value));
    }

    println!("\n3. Deleting with debug logs...");
    db.delete(b"user:bob")?;

    println!("\n=== Debug Logging Demo Complete ===");

    Ok(())
}
