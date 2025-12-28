// Integration tests for WAL basic functionality

mod common;

use common::WalTestFixture;
use rustlite_wal::{RecordType, SyncMode, WalConfig, WalManager, WalRecord};

#[test]
fn test_wal_open_and_close() {
    let fixture = WalTestFixture::new();

    let mut config = WalConfig::default();
    config.wal_dir = fixture.wal_dir().clone();

    let mut manager = WalManager::new(config).expect("Failed to create WAL manager");
    manager.open().expect("Failed to open WAL");
    manager.close().expect("Failed to close WAL");
}

#[test]
fn test_wal_append_and_sync() {
    let fixture = WalTestFixture::new();

    let mut config = WalConfig::default();
    config.wal_dir = fixture.wal_dir().clone();
    config.sync_mode = SyncMode::Sync;

    let mut manager = WalManager::new(config).expect("Failed to create WAL manager");
    manager.open().expect("Failed to open WAL");

    // Create a simple PUT record
    let record = WalRecord::new(RecordType::Put, b"key1".to_vec(), b"value1".to_vec());
    let _seq = manager.append(record).expect("Failed to append record");

    manager.sync().expect("Failed to sync WAL");
    manager.close().expect("Failed to close WAL");

    // Verify segment was created
    let segments = fixture.list_segments();
    assert!(!segments.is_empty(), "Expected at least one WAL segment");
}

#[test]
fn test_wal_multiple_records() {
    let fixture = WalTestFixture::new();

    let mut config = WalConfig::default();
    config.wal_dir = fixture.wal_dir().clone();

    let mut manager = WalManager::new(config).expect("Failed to create WAL manager");
    manager.open().expect("Failed to open WAL");

    // Append multiple records
    for i in 0..10 {
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

#[test]
fn test_wal_different_sync_modes() {
    for sync_mode in [SyncMode::Sync, SyncMode::Async, SyncMode::None] {
        let fixture = WalTestFixture::new();

        let mut config = WalConfig::default();
        config.wal_dir = fixture.wal_dir().clone();
        config.sync_mode = sync_mode;

        let mut manager = WalManager::new(config).expect("Failed to create WAL manager");
        manager.open().expect("Failed to open WAL");

        let record = WalRecord::new(RecordType::Put, b"key".to_vec(), b"value".to_vec());
        manager.append(record).expect("Failed to append");

        manager.close().expect("Failed to close");
    }
}
