//! Transaction Demo
//!
//! This example demonstrates RustLite's MVCC transaction support (v0.5.0+).
//! It shows:
//! - Basic transaction operations (begin, put, commit)
//! - Rollback functionality
//! - Snapshot isolation between concurrent transactions
//! - A bank account transfer example demonstrating ACID guarantees

use rustlite::{Database, IsolationLevel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== RustLite Transaction Demo ===\n");

    // Create an in-memory database with transaction support
    let db = Database::in_memory()?;

    // Demo 1: Basic Transaction
    println!("1. Basic Transaction:");
    {
        let mut txn = db.begin()?;
        txn.put(b"user:alice".to_vec(), b"Alice Smith".to_vec())?;
        txn.put(b"user:bob".to_vec(), b"Bob Jones".to_vec())?;
        txn.commit()?;
        println!("   ✓ Committed 2 users");

        let txn = db.begin()?;
        let alice = txn.get(b"user:alice")?;
        println!("   ✓ Read back: {:?}", String::from_utf8_lossy(&alice.unwrap()));
    }

    // Demo 2: Rollback
    println!("\n2. Rollback Demo:");
    {
        let mut txn = db.begin()?;
        txn.put(b"temp:data".to_vec(), b"temporary".to_vec())?;
        println!("   ✓ Wrote temporary data");
        txn.rollback()?;
        println!("   ✓ Rolled back transaction");

        let txn = db.begin()?;
        let result = txn.get(b"temp:data")?;
        println!("   ✓ Data after rollback: {:?}", result);
        assert!(result.is_none(), "Data should not exist after rollback");
    }

    // Demo 3: Snapshot Isolation
    println!("\n3. Snapshot Isolation:");
    {
        // Initial state
        let mut txn = db.begin()?;
        txn.put(b"counter".to_vec(), b"100".to_vec())?;
        txn.commit()?;
        println!("   ✓ Initial counter value: 100");

        // Start transaction 1 (reads counter)
        let txn1 = db.begin()?;
        let counter_bytes1 = txn1.get(b"counter")?.unwrap();
        let value1 = String::from_utf8_lossy(&counter_bytes1);
        println!("   ✓ Transaction 1 reads: {}", value1);

        // Transaction 2 updates counter
        let mut txn2 = db.begin()?;
        txn2.put(b"counter".to_vec(), b"200".to_vec())?;
        txn2.commit()?;
        println!("   ✓ Transaction 2 commits new value: 200");

        // Transaction 1 still sees old value (snapshot isolation)
        let counter_bytes2 = txn1.get(b"counter")?.unwrap();
        let value1_again = String::from_utf8_lossy(&counter_bytes2);
        println!("   ✓ Transaction 1 still sees: {} (snapshot isolation)", value1_again);
        assert_eq!(value1, value1_again, "Transaction should see its snapshot");
    }

    // Demo 4: Bank Account Transfer (ACID guarantees)
    println!("\n4. Bank Account Transfer (ACID Demo):");
    {
        // Initialize accounts
        let mut txn = db.begin()?;
        txn.put(b"account:alice".to_vec(), b"1000".to_vec())?;
        txn.put(b"account:bob".to_vec(), b"500".to_vec())?;
        txn.commit()?;
        println!("   ✓ Initial balances: Alice=$1000, Bob=$500");

        // Transfer $200 from Alice to Bob
        let mut txn = db.begin()?;
        
        // Read current balances
        let alice_balance: i32 = String::from_utf8_lossy(
            &txn.get(b"account:alice")?.unwrap()
        ).parse()?;
        let bob_balance: i32 = String::from_utf8_lossy(
            &txn.get(b"account:bob")?.unwrap()
        ).parse()?;

        // Perform transfer
        let transfer_amount = 200;
        if alice_balance >= transfer_amount {
            let new_alice = alice_balance - transfer_amount;
            let new_bob = bob_balance + transfer_amount;
            
            txn.put(b"account:alice".to_vec(), new_alice.to_string().into_bytes())?;
            txn.put(b"account:bob".to_vec(), new_bob.to_string().into_bytes())?;
            txn.commit()?;
            
            println!("   ✓ Transfer successful: Alice -${}, Bob +${}", transfer_amount, transfer_amount);
        } else {
            txn.rollback()?;
            println!("   ✗ Insufficient funds, rolled back");
        }

        // Verify final balances
        let txn = db.begin()?;
        let alice_bytes = txn.get(b"account:alice")?.unwrap();
        let bob_bytes = txn.get(b"account:bob")?.unwrap();
        let alice_final = String::from_utf8_lossy(&alice_bytes);
        let bob_final = String::from_utf8_lossy(&bob_bytes);
        println!("   ✓ Final balances: Alice=${}, Bob=${}", alice_final, bob_final);
    }

    // Demo 5: Scan with Transactions
    println!("\n5. Prefix Scan:");
    {
        let mut txn = db.begin()?;
        txn.put(b"product:laptop".to_vec(), b"$1200".to_vec())?;
        txn.put(b"product:mouse".to_vec(), b"$25".to_vec())?;
        txn.put(b"product:keyboard".to_vec(), b"$80".to_vec())?;
        txn.put(b"order:001".to_vec(), b"laptop".to_vec())?;
        txn.commit()?;

        let txn = db.begin()?;
        let products = txn.scan(b"product:")?;
        println!("   ✓ Found {} products:", products.len());
        for (key, value) in products {
            let name = String::from_utf8_lossy(&key).replace("product:", "");
            let price = String::from_utf8_lossy(&value);
            println!("     - {}: {}", name, price);
        }
    }

    // Demo 6: Isolation Levels
    println!("\n6. Custom Isolation Level:");
    {
        let txn = db.begin_transaction(IsolationLevel::Serializable)?;
        println!("   ✓ Created transaction with Serializable isolation");
        println!("   ✓ Transaction ID: {}", txn.id());
        println!("   ✓ Isolation Level: {:?}", txn.isolation_level());
    }

    // Demo 7: Garbage Collection
    println!("\n7. Garbage Collection:");
    {
        // Create multiple versions
        for i in 0..5 {
            let mut txn = db.begin()?;
            txn.put(b"versioned:key".to_vec(), format!("version{}", i).into_bytes())?;
            txn.commit()?;
        }
        println!("   ✓ Created 5 versions of a key");

        db.gc()?;
        println!("   ✓ Ran garbage collection to clean up old versions");
    }

    println!("\n=== Demo Complete ===");
    println!("All transaction features demonstrated successfully!");

    Ok(())
}
