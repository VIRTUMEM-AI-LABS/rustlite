//! Relational data example demonstrating Users and Orders tables
//! with foreign key relationships using indexes.

use rustlite::{Database, IndexType, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Order {
    id: u64,
    user_id: u64, // Foreign key to User
    product: String,
    amount: f64,
}

fn main() -> Result<()> {
    println!("=== RustLite Relational Demo ===\n");

    // Create in-memory database
    let db = Database::in_memory()?;

    // =========================================================================
    // Setup: Create indexes for Users table
    // =========================================================================
    println!("Setting up Users table indexes...");

    // Primary key index (Hash for O(1) lookup by user_id)
    db.create_index("users_pk", IndexType::Hash)?;

    // Secondary index for email lookups
    db.create_index("users_by_email", IndexType::Hash)?;

    // Secondary index for name lookups (BTree for range queries)
    db.create_index("users_by_name", IndexType::BTree)?;

    // =========================================================================
    // Setup: Create indexes for Orders table
    // =========================================================================
    println!("Setting up Orders table indexes...");

    // Primary key index
    db.create_index("orders_pk", IndexType::Hash)?;

    // Foreign key index (BTree to query all orders for a user)
    db.create_index("orders_by_user", IndexType::BTree)?;

    println!("✓ Indexes created\n");

    // =========================================================================
    // Insert Users
    // =========================================================================
    println!("Inserting users...");

    let users = vec![
        User {
            id: 1,
            name: "Alice Johnson".to_string(),
            email: "alice@example.com".to_string(),
        },
        User {
            id: 2,
            name: "Bob Smith".to_string(),
            email: "bob@example.com".to_string(),
        },
        User {
            id: 3,
            name: "Charlie Brown".to_string(),
            email: "charlie@example.com".to_string(),
        },
    ];

    for user in &users {
        // Store user data
        let key = format!("user:{}", user.id);
        let value = bincode::serialize(user).unwrap();
        db.put(key.as_bytes(), &value)?;

        // Update indexes
        db.index_insert("users_pk", &user.id.to_le_bytes(), user.id)?;
        db.index_insert("users_by_email", user.email.as_bytes(), user.id)?;
        db.index_insert("users_by_name", user.name.as_bytes(), user.id)?;

        println!("  ✓ Inserted user: {} (ID: {})", user.name, user.id);
    }
    println!();

    // =========================================================================
    // Insert Orders
    // =========================================================================
    println!("Inserting orders...");

    let orders = vec![
        Order {
            id: 101,
            user_id: 1,
            product: "Laptop".to_string(),
            amount: 1200.00,
        },
        Order {
            id: 102,
            user_id: 1,
            product: "Mouse".to_string(),
            amount: 25.00,
        },
        Order {
            id: 103,
            user_id: 2,
            product: "Keyboard".to_string(),
            amount: 75.00,
        },
        Order {
            id: 104,
            user_id: 3,
            product: "Monitor".to_string(),
            amount: 350.00,
        },
        Order {
            id: 105,
            user_id: 1,
            product: "Headphones".to_string(),
            amount: 150.00,
        },
    ];

    for order in &orders {
        // Validate foreign key constraint
        let user_exists = !db
            .index_find("users_pk", &order.user_id.to_le_bytes())?
            .is_empty();
        if !user_exists {
            eprintln!(
                "ERROR: Invalid user_id {} for order {}",
                order.user_id, order.id
            );
            continue;
        }

        // Store order data
        let key = format!("order:{}", order.id);
        let value = bincode::serialize(order).unwrap();
        db.put(key.as_bytes(), &value)?;

        // Update indexes
        db.index_insert("orders_pk", &order.id.to_le_bytes(), order.id)?;
        db.index_insert("orders_by_user", &order.user_id.to_le_bytes(), order.id)?;

        println!(
            "  ✓ Inserted order: {} for user {} (${:.2})",
            order.product, order.user_id, order.amount
        );
    }
    println!();

    // =========================================================================
    // Query 1: Lookup user by ID (Primary Key)
    // =========================================================================
    println!("=== Query 1: Find user by ID ===");
    let user_id: u64 = 2;
    let user_ids = db.index_find("users_pk", &user_id.to_le_bytes())?;

    if let Some(&found_id) = user_ids.first() {
        let key = format!("user:{}", found_id);
        if let Some(data) = db.get(key.as_bytes())? {
            let user: User = bincode::deserialize(&data).unwrap();
            println!("Found user: {} <{}>", user.name, user.email);
        }
    }
    println!();

    // =========================================================================
    // Query 2: Lookup user by email
    // =========================================================================
    println!("=== Query 2: Find user by email ===");
    let email = b"alice@example.com";
    let user_ids = db.index_find("users_by_email", email)?;

    if let Some(&found_id) = user_ids.first() {
        let key = format!("user:{}", found_id);
        if let Some(data) = db.get(key.as_bytes())? {
            let user: User = bincode::deserialize(&data).unwrap();
            println!("Found user: {} (ID: {})", user.name, user.id);
        }
    }
    println!();

    // =========================================================================
    // Query 3: Get all orders for a specific user (Foreign Key Join)
    // =========================================================================
    println!("=== Query 3: Get all orders for user 1 (Alice) ===");
    let user_id: u64 = 1;

    // Get user info
    let user_ids = db.index_find("users_pk", &user_id.to_le_bytes())?;
    if let Some(&found_id) = user_ids.first() {
        let key = format!("user:{}", found_id);
        if let Some(data) = db.get(key.as_bytes())? {
            let user: User = bincode::deserialize(&data).unwrap();
            println!("User: {}", user.name);
        }
    }

    // Get all orders for this user
    let order_ids = db.index_find("orders_by_user", &user_id.to_le_bytes())?;
    println!("Found {} orders:", order_ids.len());

    let mut total = 0.0;
    for &order_id in &order_ids {
        let key = format!("order:{}", order_id);
        if let Some(data) = db.get(key.as_bytes())? {
            let order: Order = bincode::deserialize(&data).unwrap();
            println!(
                "  - Order #{}: {} (${:.2})",
                order.id, order.product, order.amount
            );
            total += order.amount;
        }
    }
    println!("Total: ${:.2}", total);
    println!();

    // =========================================================================
    // Query 4: List all users (using name index for sorted order)
    // =========================================================================
    println!("=== Query 4: List all users (sorted by name) ===");
    let info = db.index_info()?;
    for idx in &info {
        if idx.name == "users_by_name" {
            println!("Users in database: {}", idx.entry_count);
        }
    }
    println!();

    // =========================================================================
    // Query 5: Get order count per user
    // =========================================================================
    println!("=== Query 5: Order summary by user ===");
    for user in &users {
        let order_ids = db.index_find("orders_by_user", &user.id.to_le_bytes())?;
        let mut total = 0.0;

        for &order_id in &order_ids {
            let key = format!("order:{}", order_id);
            if let Some(data) = db.get(key.as_bytes())? {
                let order: Order = bincode::deserialize(&data).unwrap();
                total += order.amount;
            }
        }

        println!(
            "{}: {} orders, ${:.2} total",
            user.name,
            order_ids.len(),
            total
        );
    }
    println!();

    // =========================================================================
    // Demonstrate DELETE with foreign key cascade
    // =========================================================================
    println!("=== Delete user 3 and cascade to orders ===");
    let user_id_to_delete: u64 = 3;

    // Find and delete all orders for this user
    let order_ids = db.index_find("orders_by_user", &user_id_to_delete.to_le_bytes())?;
    println!(
        "Deleting {} orders for user {}...",
        order_ids.len(),
        user_id_to_delete
    );

    for &order_id in &order_ids {
        let key = format!("order:{}", order_id);
        db.delete(key.as_bytes())?;
        db.index_remove("orders_pk", &order_id.to_le_bytes())?;
        db.index_remove("orders_by_user", &user_id_to_delete.to_le_bytes())?;
        println!("  ✓ Deleted order {}", order_id);
    }

    // Delete the user
    let key = format!("user:{}", user_id_to_delete);
    if let Some(data) = db.get(key.as_bytes())? {
        let user: User = bincode::deserialize(&data).unwrap();
        db.delete(key.as_bytes())?;
        db.index_remove("users_pk", &user.id.to_le_bytes())?;
        db.index_remove("users_by_email", user.email.as_bytes())?;
        db.index_remove("users_by_name", user.name.as_bytes())?;
        println!("  ✓ Deleted user: {}", user.name);
    }
    println!();

    // =========================================================================
    // Final statistics
    // =========================================================================
    println!("=== Final Database Statistics ===");
    let index_info = db.index_info()?;
    for info in index_info {
        println!(
            "Index '{}': {} entries ({:?})",
            info.name, info.entry_count, info.index_type
        );
    }

    Ok(())
}
