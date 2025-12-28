// Recovery and crash scenario tests for WAL
#![allow(clippy::field_reassign_with_default)]

mod common;

use common::WalTestFixture;
use rustlite_wal::{RecordType, RecoveryManager, SyncMode, WalConfig, WalManager, WalRecord};

#[test]
fn test_recovery_from_clean_shutdown() {
    let fixture = WalTestFixture::new();

    let mut config = WalConfig::default();
    config.wal_dir = fixture.wal_dir().clone();

    // Write some records and close cleanly
    {
        let mut manager = WalManager::new(config.clone()).expect("Failed to create WAL manager");
        manager.open().expect("Failed to open WAL");

        for i in 0..5 {
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            let record = WalRecord::new(
                RecordType::Put,
                key.as_bytes().to_vec(),
                value.as_bytes().to_vec(),
            );
            manager.append(record).expect("Failed to append record");
        }

        manager.sync().expect("Failed to sync");
        manager.close().expect("Failed to close");
    }

    // Recover from WAL
    let recovery = RecoveryManager::new(config).expect("Failed to create recovery manager");
    let records = recovery.recover().expect("Failed to recover records");

    assert_eq!(records.len(), 5, "Expected 5 recovered records");
}

#[test]
fn test_recovery_from_partial_write() {
    // This test will simulate a crash scenario by:
    // 1. Writing some complete records
    // 2. Simulating a partial write (incomplete record at end)
    // 3. Verifying recovery handles it gracefully

    let fixture = WalTestFixture::new();

    let mut config = WalConfig::default();
    config.wal_dir = fixture.wal_dir().clone();

    // TODO: Implement when we have manual segment manipulation utilities
    // For now, just verify basic recovery works
    let mut manager = WalManager::new(config.clone()).expect("Failed to create WAL manager");
    manager.open().expect("Failed to open WAL");

    let record = WalRecord::new(RecordType::Put, b"key".to_vec(), b"value".to_vec());
    manager.append(record).expect("Failed to append");
    manager.sync().expect("Failed to sync");
    manager.close().expect("Failed to close");

    let recovery = RecoveryManager::new(config).expect("Failed to create recovery manager");
    let records = recovery.recover().expect("Recovery should succeed");
    assert_eq!(records.len(), 1);
}

#[test]
fn test_recovery_with_transactions() {
    let fixture = WalTestFixture::new();

    let mut config = WalConfig::default();
    config.wal_dir = fixture.wal_dir().clone();

    // Write a transaction sequence
    {
        let mut manager = WalManager::new(config.clone()).expect("Failed to create WAL manager");
        manager.open().expect("Failed to open WAL");

        // Transaction 1: Complete (tx_id = 1)
        manager
            .append(WalRecord::begin_tx(1))
            .expect("Failed to append");
        manager
            .append(WalRecord::put(b"tx_key1".to_vec(), b"tx_val1".to_vec()))
            .expect("Failed to append");
        manager
            .append(WalRecord::commit_tx(1))
            .expect("Failed to append");

        // Transaction 2: Incomplete (tx_id = 2, no commit)
        manager
            .append(WalRecord::begin_tx(2))
            .expect("Failed to append");
        manager
            .append(WalRecord::put(b"tx_key2".to_vec(), b"tx_val2".to_vec()))
            .expect("Failed to append");

        manager.sync().expect("Failed to sync");
        manager.close().expect("Failed to close");
    }

    // Recover - should only include committed transaction data records
    // recover() filters out transaction markers, only returning PUT/DELETE records
    let recovery = RecoveryManager::new(config).expect("Failed to create recovery manager");
    let records = recovery.recover().expect("Failed to recover");

    // Should have 1 PUT record from complete transaction
    // (BEGIN_TX and COMMIT_TX markers are filtered out by recover())
    // Incomplete tx (tx_key2) should be rolled back
    assert_eq!(
        records.len(),
        1,
        "Expected 1 data record from committed transaction"
    );
}

#[test]
fn test_recovery_with_corrupted_crc() {
    // Test that recovery gracefully handles corrupted records
    // by stopping at the first corruption

    let fixture = WalTestFixture::new();

    let mut config = WalConfig::default();
    config.wal_dir = fixture.wal_dir().clone();

    // Write valid records
    {
        let mut manager = WalManager::new(config.clone()).expect("Failed to create WAL manager");
        manager.open().expect("Failed to open WAL");

        for i in 0..3 {
            let record = WalRecord::new(
                RecordType::Put,
                format!("key{}", i).as_bytes().to_vec(),
                format!("value{}", i).as_bytes().to_vec(),
            );
            manager.append(record).expect("Failed to append");
        }

        manager.sync().expect("Failed to sync");
        manager.close().expect("Failed to close");
    }

    // TODO: Manually corrupt a record in the segment file
    // For now, just verify basic recovery works

    let recovery = RecoveryManager::new(config).expect("Failed to create recovery manager");
    let records = recovery
        .recover()
        .expect("Recovery should succeed even with corruption");

    // Should recover at least some records before corruption
    assert!(!records.is_empty(), "Should recover some valid records");
}

#[test]
fn test_recovery_empty_wal() {
    let fixture = WalTestFixture::new();

    let config = WalConfig {
        wal_dir: fixture.wal_dir().clone(),
        sync_mode: SyncMode::Sync,
        max_segment_size: 1024 * 1024,
    };

    // No WAL segments exist yet
    let recovery = RecoveryManager::new(config).expect("Failed to create recovery manager");
    let records = recovery
        .recover()
        .expect("Recovery from empty WAL should succeed");

    assert!(records.is_empty(), "Empty WAL should return no records");
}
