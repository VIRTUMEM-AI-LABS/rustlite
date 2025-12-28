use crate::transaction::*;
use std::sync::Arc;
use std::thread;

#[test]
fn test_mvcc_basic_read_write() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = TransactionManager::new(Arc::clone(&storage));

    // Start transaction 1
    let mut txn1 = manager.begin(IsolationLevel::RepeatableRead).unwrap();

    // Write data
    txn1.put(b"key1".to_vec(), b"value1".to_vec()).unwrap();
    txn1.put(b"key2".to_vec(), b"value2".to_vec()).unwrap();

    // Read data within same transaction
    assert_eq!(txn1.get(b"key1").unwrap(), Some(b"value1".to_vec()));
    assert_eq!(txn1.get(b"key2").unwrap(), Some(b"value2".to_vec()));

    // Commit
    txn1.commit().unwrap();

    // Start new transaction and read committed data
    let txn2 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    assert_eq!(txn2.get(b"key1").unwrap(), Some(b"value1".to_vec()));
    assert_eq!(txn2.get(b"key2").unwrap(), Some(b"value2".to_vec()));
}

#[test]
fn test_mvcc_snapshot_isolation() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = Arc::new(TransactionManager::new(Arc::clone(&storage)));

    // Transaction 1: Write initial data
    let mut txn1 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn1.put(b"key1".to_vec(), b"v1".to_vec()).unwrap();
    txn1.commit().unwrap();

    // Transaction 2: Start and read (gets snapshot)
    let txn2 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    assert_eq!(txn2.get(b"key1").unwrap(), Some(b"v1".to_vec()));

    // Transaction 3: Update the value
    let mut txn3 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn3.put(b"key1".to_vec(), b"v2".to_vec()).unwrap();
    txn3.commit().unwrap();

    // Transaction 2 should still see old value (snapshot isolation)
    assert_eq!(txn2.get(b"key1").unwrap(), Some(b"v1".to_vec()));

    // New transaction should see updated value
    let txn4 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    assert_eq!(txn4.get(b"key1").unwrap(), Some(b"v2".to_vec()));
}

#[test]
fn test_mvcc_rollback() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = Arc::new(TransactionManager::new(Arc::clone(&storage)));

    // Transaction 1: Write and commit
    let mut txn1 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn1.put(b"key1".to_vec(), b"value1".to_vec()).unwrap();
    txn1.commit().unwrap();

    // Transaction 2: Update but rollback
    let mut txn2 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn2.put(b"key1".to_vec(), b"value2".to_vec()).unwrap();
    txn2.rollback().unwrap();

    // Transaction 3: Should see original value
    let txn3 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    assert_eq!(txn3.get(b"key1").unwrap(), Some(b"value1".to_vec()));
}

#[test]
fn test_mvcc_delete() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = Arc::new(TransactionManager::new(Arc::clone(&storage)));

    // Write data
    let mut txn1 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn1.put(b"key1".to_vec(), b"value1".to_vec()).unwrap();
    txn1.commit().unwrap();

    // Delete data
    let mut txn2 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn2.delete(b"key1").unwrap();
    txn2.commit().unwrap();

    // Verify deleted
    let txn3 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    assert_eq!(txn3.get(b"key1").unwrap(), None);
}

#[test]
fn test_mvcc_concurrent_writes() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = Arc::new(TransactionManager::new(Arc::clone(&storage)));

    // Setup initial data
    let mut txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn.put(b"counter".to_vec(), b"0".to_vec()).unwrap();
    txn.commit().unwrap();

    // Spawn multiple threads doing concurrent writes
    let mut handles = vec![];
    for i in 0..5 {
        let mgr = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            let mut txn = mgr.begin(IsolationLevel::RepeatableRead).unwrap();
            let value = format!("value_{}", i);
            txn.put(b"counter".to_vec(), value.as_bytes().to_vec())
                .unwrap();
            txn.commit().unwrap();
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify final state exists
    let txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    assert!(txn.get(b"counter").unwrap().is_some());
}

#[test]
fn test_mvcc_scan() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = Arc::new(TransactionManager::new(Arc::clone(&storage)));

    // Write multiple keys with prefix
    let mut txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn.put(b"user:1".to_vec(), b"alice".to_vec()).unwrap();
    txn.put(b"user:2".to_vec(), b"bob".to_vec()).unwrap();
    txn.put(b"user:3".to_vec(), b"charlie".to_vec()).unwrap();
    txn.put(b"post:1".to_vec(), b"post1".to_vec()).unwrap();
    txn.commit().unwrap();

    // Scan with prefix
    let txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    let results = txn.scan(b"user:").unwrap();

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].0, b"user:1");
    assert_eq!(results[0].1, b"alice");
    assert_eq!(results[1].0, b"user:2");
    assert_eq!(results[2].0, b"user:3");
}

#[test]
fn test_version_chain_gc() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = Arc::new(TransactionManager::new(Arc::clone(&storage)));

    // Create multiple versions
    for i in 0..5 {
        let mut txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();
        let value = format!("value_{}", i);
        txn.put(b"key1".to_vec(), value.as_bytes().to_vec())
            .unwrap();
        txn.commit().unwrap();
        thread::sleep(std::time::Duration::from_millis(10));
    }

    // Perform GC (should clean up old versions)
    manager.gc().unwrap();

    // Latest value should still be readable
    let txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    assert_eq!(txn.get(b"key1").unwrap(), Some(b"value_4".to_vec()));
}

#[test]
fn test_read_own_writes() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = Arc::new(TransactionManager::new(Arc::clone(&storage)));

    let mut txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();

    // Write and read within same transaction
    txn.put(b"key1".to_vec(), b"value1".to_vec()).unwrap();
    assert_eq!(txn.get(b"key1").unwrap(), Some(b"value1".to_vec()));

    // Update and read again
    txn.put(b"key1".to_vec(), b"value2".to_vec()).unwrap();
    assert_eq!(txn.get(b"key1").unwrap(), Some(b"value2".to_vec()));

    txn.commit().unwrap();

    // Verify committed value
    let txn2 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    assert_eq!(txn2.get(b"key1").unwrap(), Some(b"value2".to_vec()));
}

#[test]
fn test_isolation_levels() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = Arc::new(TransactionManager::new(Arc::clone(&storage)));

    // Test different isolation levels
    let txn1 = manager.begin(IsolationLevel::ReadUncommitted).unwrap();
    assert_eq!(txn1.isolation_level(), IsolationLevel::ReadUncommitted);

    let txn2 = manager.begin(IsolationLevel::ReadCommitted).unwrap();
    assert_eq!(txn2.isolation_level(), IsolationLevel::ReadCommitted);

    let txn3 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    assert_eq!(txn3.isolation_level(), IsolationLevel::RepeatableRead);

    let txn4 = manager.begin(IsolationLevel::Serializable).unwrap();
    assert_eq!(txn4.isolation_level(), IsolationLevel::Serializable);
}

#[test]
fn test_transaction_ids_unique() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = Arc::new(TransactionManager::new(Arc::clone(&storage)));

    let txn1 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    let txn2 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    let txn3 = manager.begin(IsolationLevel::RepeatableRead).unwrap();

    assert_ne!(txn1.id(), txn2.id());
    assert_ne!(txn2.id(), txn3.id());
    assert_ne!(txn1.id(), txn3.id());
}

#[test]
fn test_phantom_read_prevention() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = Arc::new(TransactionManager::new(Arc::clone(&storage)));

    // Transaction 1: Start and scan
    let txn1 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    let initial_scan = txn1.scan(b"user:").unwrap();
    assert_eq!(initial_scan.len(), 0);

    // Transaction 2: Insert new data
    let mut txn2 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn2.put(b"user:1".to_vec(), b"alice".to_vec()).unwrap();
    txn2.commit().unwrap();

    // Transaction 1: Scan again (should still see empty due to snapshot isolation)
    let second_scan = txn1.scan(b"user:").unwrap();
    assert_eq!(second_scan.len(), 0); // Phantom read prevented

    // New transaction should see the insert
    let txn3 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    let final_scan = txn3.scan(b"user:").unwrap();
    assert_eq!(final_scan.len(), 1);
}

#[test]
fn test_write_conflict_detection() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = TransactionManager::new(storage.clone());

    // Initial data
    let mut txn0 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn0.put(b"balance".to_vec(), b"1000".to_vec()).unwrap();
    txn0.commit().unwrap();

    // Two concurrent transactions trying to update same key
    let txn1 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    let mut txn2 = manager.begin(IsolationLevel::RepeatableRead).unwrap();

    // Read in txn1
    let _balance1 = txn1.get(b"balance").unwrap();

    // Txn2 writes and commits
    txn2.put(b"balance".to_vec(), b"1500".to_vec()).unwrap();
    txn2.commit().unwrap();

    // Txn1 should still see old value (snapshot isolation)
    let balance_after = txn1.get(b"balance").unwrap();
    assert_eq!(balance_after, Some(b"1000".to_vec()));
}

#[test]
fn test_long_running_transaction() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = TransactionManager::new(storage.clone());

    // Start a long-running transaction
    let long_txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();

    // Multiple short transactions commit
    for i in 0..10 {
        let mut txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();
        txn.put(format!("key{}", i).into_bytes(), format!("value{}", i).into_bytes())
            .unwrap();
        txn.commit().unwrap();
    }

    // Long transaction should not see any of the new data (snapshot isolation)
    for i in 0..10 {
        let result = long_txn.get(&format!("key{}", i).into_bytes()).unwrap();
        assert_eq!(result, None, "Long transaction should not see key{}", i);
    }
}

#[test]
fn test_transaction_abort_cleanup() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = TransactionManager::new(storage.clone());

    // Create and rollback multiple transactions
    for i in 0..5 {
        let mut txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();
        txn.put(format!("temp{}", i).into_bytes(), b"data".to_vec())
            .unwrap();
        txn.rollback().unwrap();
    }

    // Verify none of the data exists
    let verify_txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    for i in 0..5 {
        let result = verify_txn.get(&format!("temp{}", i).into_bytes()).unwrap();
        assert_eq!(result, None);
    }
}

#[test]
fn test_interleaved_reads_writes() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = TransactionManager::new(storage.clone());

    // Setup initial state
    let mut setup = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    setup.put(b"counter".to_vec(), b"0".to_vec()).unwrap();
    setup.commit().unwrap();

    // Transaction 1: Read
    let txn1 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    let val1 = txn1.get(b"counter").unwrap();
    assert_eq!(val1, Some(b"0".to_vec()));

    // Transaction 2: Write
    let mut txn2 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn2.put(b"counter".to_vec(), b"5".to_vec()).unwrap();

    // Transaction 1: Read again (should still see 0)
    let val1_again = txn1.get(b"counter").unwrap();
    assert_eq!(val1_again, Some(b"0".to_vec()));

    // Transaction 2: Commit
    txn2.commit().unwrap();

    // Transaction 1: Still sees old value
    let val1_final = txn1.get(b"counter").unwrap();
    assert_eq!(val1_final, Some(b"0".to_vec()));

    // New transaction sees new value
    let txn3 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    let val3 = txn3.get(b"counter").unwrap();
    assert_eq!(val3, Some(b"5".to_vec()));
}

#[test]
fn test_delete_then_recreate() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = TransactionManager::new(storage.clone());

    // Create key
    let mut txn1 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn1.put(b"key".to_vec(), b"v1".to_vec()).unwrap();
    txn1.commit().unwrap();

    // Delete key
    let mut txn2 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn2.delete(b"key").unwrap();
    txn2.commit().unwrap();

    // Verify deleted
    let txn3 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    assert_eq!(txn3.get(b"key").unwrap(), None);

    // Recreate key
    let mut txn4 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn4.put(b"key".to_vec(), b"v2".to_vec()).unwrap();
    txn4.commit().unwrap();

    // Verify recreated
    let txn5 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    assert_eq!(txn5.get(b"key").unwrap(), Some(b"v2".to_vec()));
}

#[test]
fn test_massive_concurrent_transactions() {
    use std::thread;

    let storage = Arc::new(MVCCStorage::new());
    let manager = TransactionManager::new(storage.clone());

    // Spawn 20 threads, each doing 50 transactions
    let mut handles = vec![];
    for thread_id in 0..20 {
        let mgr = manager.clone();
        let handle = thread::spawn(move || {
            for i in 0..50 {
                let mut txn = mgr.begin(IsolationLevel::RepeatableRead).unwrap();
                let key = format!("thread{}:item{}", thread_id, i);
                txn.put(key.into_bytes(), b"data".to_vec()).unwrap();
                txn.commit().unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all 1000 keys exist
    let verify = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    let mut found = 0;
    for thread_id in 0..20 {
        for i in 0..50 {
            let key = format!("thread{}:item{}", thread_id, i);
            if verify.get(&key.into_bytes()).unwrap().is_some() {
                found += 1;
            }
        }
    }
    assert_eq!(found, 1000);
}

#[test]
fn test_empty_transaction() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = TransactionManager::new(storage.clone());

    // Transaction with no operations
    let txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn.commit().unwrap(); // Should succeed

    // Rollback empty transaction
    let txn2 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    txn2.rollback().unwrap(); // Should succeed
}

#[test]
fn test_scan_with_deletes() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = TransactionManager::new(storage.clone());

    // Insert multiple keys
    let mut txn1 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    for i in 0..10 {
        txn1.put(format!("item:{}", i).into_bytes(), b"value".to_vec())
            .unwrap();
    }
    txn1.commit().unwrap();

    // Delete every other key
    let mut txn2 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    for i in (0..10).step_by(2) {
        txn2.delete(&format!("item:{}", i).into_bytes()).unwrap();
    }
    txn2.commit().unwrap();

    // Scan should only return non-deleted keys
    let txn3 = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    let results = txn3.scan(b"item:").unwrap();
    assert_eq!(results.len(), 5); // Only odd numbered items remain
}

#[test]
fn test_version_chain_ordering() {
    let storage = Arc::new(MVCCStorage::new());
    let manager = TransactionManager::new(storage.clone());

    // Create multiple versions rapidly
    for i in 0..10 {
        let mut txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();
        txn.put(b"key".to_vec(), format!("v{}", i).into_bytes())
            .unwrap();
        txn.commit().unwrap();
    }

    // Latest transaction should see latest version
    let txn = manager.begin(IsolationLevel::RepeatableRead).unwrap();
    let value = txn.get(b"key").unwrap();
    assert_eq!(value, Some(b"v9".to_vec()));
}
