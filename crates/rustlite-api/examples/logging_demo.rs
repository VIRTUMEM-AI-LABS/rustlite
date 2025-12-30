use rustlite::logging::LogConfig;
use rustlite::Database;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (info level with pretty output to stdout)
    let _guard = LogConfig::info().init();

    println!("=== RustLite Logging Demo ===\n");

    // Create database - this will log "Opening RustLite database"
    let db = Database::in_memory()?;

    // Operations will be logged with debug/info levels
    println!("\n1. Inserting data...");
    db.put(b"user:1", b"Alice")?;
    db.put(b"user:2", b"Bob")?;
    db.put(b"user:3", b"Charlie")?;

    println!("\n2. Reading data...");
    if let Some(value) = db.get(b"user:1")? {
        println!("Found: {}", String::from_utf8_lossy(&value));
    }

    println!("\n3. Deleting data...");
    db.delete(b"user:2")?;

    println!("\n4. Creating index...");
    db.create_index("users", rustlite::IndexType::Hash)?;

    println!("\n5. Beginning transaction...");
    let mut txn = db.begin()?;
    txn.put(b"txn:key".to_vec(), b"txn:value".to_vec())?;
    txn.commit()?;

    println!("\n=== Demo Complete ===");
    println!("Check the logs above to see tracing output!");

    Ok(())
}
