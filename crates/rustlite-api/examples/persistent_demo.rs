//! Demonstrates RustLite's persistent storage capabilities.
//!
//! Run with: cargo run -p rustlite --example persistent_demo

use rustlite::Database;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = "./demo_database";
    
    println!("=== RustLite Persistent Database Demo ===\n");
    
    // Clean up any previous demo data
    if Path::new(db_path).exists() {
        std::fs::remove_dir_all(db_path)?;
        println!("🧹 Cleaned up previous demo data\n");
    }
    
    // PART 1: Write data
    println!("📝 PART 1: Writing data to database...");
    {
        let db = Database::open(db_path)?;
        
        // Store some user data
        db.put(b"user:1:name", b"Alice")?;
        db.put(b"user:1:email", b"alice@example.com")?;
        db.put(b"user:1:role", b"admin")?;
        
        db.put(b"user:2:name", b"Bob")?;
        db.put(b"user:2:email", b"bob@example.com")?;
        db.put(b"user:2:role", b"user")?;
        
        // Store a counter
        db.put(b"stats:total_users", b"2")?;
        
        println!("   ✅ Stored 2 users and 1 counter");
        println!("   📁 Data written to: {}", db_path);
        
        // Database goes out of scope and closes
    }
    println!("   🔒 Database closed\n");
    
    // PART 2: Reopen and verify data persisted
    println!("🔓 PART 2: Reopening database and verifying data...");
    {
        let db = Database::open(db_path)?;
        
        // Read back the data
        let name1 = db.get(b"user:1:name")?.unwrap();
        let email1 = db.get(b"user:1:email")?.unwrap();
        
        println!("   User 1: {} <{}>", 
            String::from_utf8_lossy(&name1),
            String::from_utf8_lossy(&email1));
        
        let name2 = db.get(b"user:2:name")?.unwrap();
        let email2 = db.get(b"user:2:email")?.unwrap();
        
        println!("   User 2: {} <{}>", 
            String::from_utf8_lossy(&name2),
            String::from_utf8_lossy(&email2));
        
        let count = db.get(b"stats:total_users")?.unwrap();
        println!("   Total users: {}", String::from_utf8_lossy(&count));
        
        println!("   ✅ All data successfully retrieved!");
    }
    println!();
    
    // PART 3: Update and delete operations
    println!("🔄 PART 3: Updating and deleting data...");
    {
        let db = Database::open(db_path)?;
        
        // Update Alice's role
        db.put(b"user:1:role", b"superadmin")?;
        println!("   📝 Updated Alice's role to 'superadmin'");
        
        // Delete Bob's email
        db.delete(b"user:2:email")?;
        println!("   🗑️  Deleted Bob's email");
        
        // Update counter
        db.put(b"stats:total_users", b"2")?;
        
        // Force sync to disk
        db.sync()?;
        println!("   💾 Synced to disk");
    }
    println!();
    
    // PART 4: Final verification
    println!("✅ PART 4: Final verification...");
    {
        let db = Database::open(db_path)?;
        
        let role = db.get(b"user:1:role")?.unwrap();
        println!("   Alice's role: {}", String::from_utf8_lossy(&role));
        
        let bob_email = db.get(b"user:2:email")?;
        println!("   Bob's email: {:?}", bob_email.as_ref().map(|v| String::from_utf8_lossy(v).to_string()));
        
        assert_eq!(role, b"superadmin");
        assert_eq!(bob_email, None);
        
        println!("   ✅ All assertions passed!");
    }
    
    // Clean up demo
    std::fs::remove_dir_all(db_path)?;
    println!("\n🧹 Cleaned up demo database");
    println!("\n=== Demo Complete! ===");
    
    Ok(())
}
