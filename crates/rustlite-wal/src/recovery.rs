// WAL recovery module - handles crash recovery logic
//
// Recovery is responsible for:
// 1. Reading all WAL records from disk
// 2. Tracking transaction boundaries (BEGIN/COMMIT)
// 3. Only returning committed records (incomplete transactions are rolled back)
// 4. Handling corrupted or truncated records gracefully

use crate::{WalConfig, WalReader, WalRecord};
use crate::record::RecordPayload;
use rustlite_core::{Error, Result};
use std::collections::{HashMap, HashSet};

/// Manages WAL recovery after crash or restart
pub struct RecoveryManager {
    config: WalConfig,
}

/// Represents a transaction's state during recovery
#[derive(Debug, Clone)]
struct TransactionState {
    /// Records belonging to this transaction
    records: Vec<WalRecord>,
    /// Whether the transaction was committed
    committed: bool,
}

impl RecoveryManager {
    /// Create a new recovery manager with the given configuration
    pub fn new(config: WalConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Recover records from WAL
    ///
    /// This method:
    /// 1. Reads all records from all WAL segments
    /// 2. Tracks transaction boundaries
    /// 3. Only returns records from committed transactions
    /// 4. For records outside transactions, returns them directly
    ///
    /// Returns a vector of recovered records in order
    pub fn recover(&self) -> Result<Vec<WalRecord>> {
        let mut reader = WalReader::new(&self.config.wal_dir)?;

        if reader.segment_count() == 0 {
            return Ok(Vec::new());
        }

        // Track active transactions
        let mut transactions: HashMap<u64, TransactionState> = HashMap::new();
        let mut committed_tx_ids: HashSet<u64> = HashSet::new();

        // Records outside of any transaction
        let mut standalone_records: Vec<WalRecord> = Vec::new();

        // Current transaction context (for records that don't specify tx_id)
        let mut current_tx_id: Option<u64> = None;

        // Read all records
        loop {
            match reader.next_record() {
                Ok(Some(record)) => {
                    match &record.payload {
                        RecordPayload::BeginTx { tx_id } => {
                            // Start tracking a new transaction
                            transactions.insert(
                                *tx_id,
                                TransactionState {
                                    records: Vec::new(),
                                    committed: false,
                                },
                            );
                            current_tx_id = Some(*tx_id);
                        }
                        RecordPayload::CommitTx { tx_id } => {
                            // Mark transaction as committed
                            if let Some(tx_state) = transactions.get_mut(tx_id) {
                                tx_state.committed = true;
                                committed_tx_ids.insert(*tx_id);
                            }
                            // Clear current tx if it matches
                            if current_tx_id == Some(*tx_id) {
                                current_tx_id = None;
                            }
                        }
                        RecordPayload::Put { .. } | RecordPayload::Delete { .. } => {
                            // Data records - add to current transaction or standalone
                            if let Some(tx_id) = current_tx_id {
                                if let Some(tx_state) = transactions.get_mut(&tx_id) {
                                    tx_state.records.push(record);
                                } else {
                                    // Transaction not found, treat as standalone
                                    standalone_records.push(record);
                                }
                            } else {
                                // No active transaction
                                standalone_records.push(record);
                            }
                        }
                        RecordPayload::Checkpoint { .. } => {
                            // Checkpoint records can be used for optimization
                            // For now, we just skip them during recovery
                        }
                    }
                }
                Ok(None) => {
                    // End of WAL
                    break;
                }
                Err(e) => {
                    // Handle errors gracefully
                    // CRC errors or truncation means we stop here
                    // Records up to this point are still valid
                    if Self::is_recoverable_error(&e) {
                        break;
                    }
                    return Err(e);
                }
            }
        }

        // Collect results: standalone records + committed transaction records
        let mut result = standalone_records;

        // Add records from committed transactions in order
        // Sort by tx_id for deterministic ordering
        let mut committed_txs: Vec<_> = transactions
            .into_iter()
            .filter(|(_, state)| state.committed)
            .collect();
        committed_txs.sort_by_key(|(tx_id, _)| *tx_id);

        for (_, tx_state) in committed_txs {
            result.extend(tx_state.records);
        }

        Ok(result)
    }

    /// Recover records with transaction markers included
    ///
    /// Unlike `recover()`, this method returns all records including
    /// BEGIN_TX and COMMIT_TX markers for committed transactions.
    /// This is useful for replaying the exact WAL state.
    pub fn recover_with_markers(&self) -> Result<Vec<WalRecord>> {
        let mut reader = WalReader::new(&self.config.wal_dir)?;

        if reader.segment_count() == 0 {
            return Ok(Vec::new());
        }

        // First pass: identify committed transactions
        let mut committed_tx_ids: HashSet<u64> = HashSet::new();
        let mut all_records: Vec<WalRecord> = Vec::new();

        loop {
            match reader.next_record() {
                Ok(Some(record)) => {
                    if let RecordPayload::CommitTx { tx_id } = &record.payload {
                        committed_tx_ids.insert(*tx_id);
                    }
                    all_records.push(record);
                }
                Ok(None) => break,
                Err(e) => {
                    if Self::is_recoverable_error(&e) {
                        break;
                    }
                    return Err(e);
                }
            }
        }

        // Second pass: filter to only include committed transactions and standalone records
        let mut result: Vec<WalRecord> = Vec::new();
        let mut current_tx_id: Option<u64> = None;
        let mut in_committed_tx = false;

        for record in all_records {
            let payload = &record.payload;
            let should_include = match payload {
                RecordPayload::BeginTx { tx_id } => {
                    in_committed_tx = committed_tx_ids.contains(tx_id);
                    current_tx_id = Some(*tx_id);
                    in_committed_tx
                }
                RecordPayload::CommitTx { tx_id } => {
                    let include = committed_tx_ids.contains(tx_id);
                    if current_tx_id == Some(*tx_id) {
                        current_tx_id = None;
                        in_committed_tx = false;
                    }
                    include
                }
                RecordPayload::Put { .. } | RecordPayload::Delete { .. } => {
                    if current_tx_id.is_some() {
                        // In a transaction
                        in_committed_tx
                    } else {
                        // Standalone record
                        true
                    }
                }
                RecordPayload::Checkpoint { .. } => {
                    // Include checkpoint markers
                    true
                }
            };

            if should_include {
                result.push(record);
            }
        }

        Ok(result)
    }

    /// Check if an error is recoverable (we can continue without the corrupted data)
    fn is_recoverable_error(err: &Error) -> bool {
        match err {
            Error::Storage(msg) => msg.contains("CRC mismatch"),
            Error::Serialization(msg) => {
                msg.contains("Incomplete") || msg.contains("truncated")
            }
            _ => false,
        }
    }

    /// Get statistics about the WAL
    pub fn get_stats(&self) -> Result<RecoveryStats> {
        let mut reader = WalReader::new(&self.config.wal_dir)?;

        let mut stats = RecoveryStats {
            segment_count: reader.segment_count(),
            total_records: 0,
            put_records: 0,
            delete_records: 0,
            transactions_started: 0,
            transactions_committed: 0,
            transactions_incomplete: 0,
            checkpoints: 0,
        };

        let mut active_transactions: HashSet<u64> = HashSet::new();

        loop {
            match reader.next_record() {
                Ok(Some(record)) => {
                    stats.total_records += 1;
                    match &record.payload {
                        RecordPayload::Put { .. } => stats.put_records += 1,
                        RecordPayload::Delete { .. } => stats.delete_records += 1,
                        RecordPayload::BeginTx { tx_id } => {
                            stats.transactions_started += 1;
                            active_transactions.insert(*tx_id);
                        }
                        RecordPayload::CommitTx { tx_id } => {
                            stats.transactions_committed += 1;
                            active_transactions.remove(tx_id);
                        }
                        RecordPayload::Checkpoint { .. } => stats.checkpoints += 1,
                    }
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }

        stats.transactions_incomplete = active_transactions.len();

        Ok(stats)
    }
}

/// Statistics about the WAL state
#[derive(Debug, Clone, Default)]
pub struct RecoveryStats {
    /// Number of segment files
    pub segment_count: usize,
    /// Total number of records
    pub total_records: usize,
    /// Number of PUT records
    pub put_records: usize,
    /// Number of DELETE records
    pub delete_records: usize,
    /// Number of transactions started
    pub transactions_started: usize,
    /// Number of transactions committed
    pub transactions_committed: usize,
    /// Number of incomplete transactions (started but not committed)
    pub transactions_incomplete: usize,
    /// Number of checkpoint records
    pub checkpoints: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RecordType, SyncMode, WalWriter};
    use tempfile::TempDir;

    fn setup_test_wal() -> (TempDir, WalConfig) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let wal_path = temp_dir.path().join("wal");
        std::fs::create_dir_all(&wal_path).expect("Failed to create WAL dir");

        let config = WalConfig {
            wal_dir: wal_path,
            sync_mode: SyncMode::Sync,
            max_segment_size: 64 * 1024 * 1024,
        };

        (temp_dir, config)
    }

    #[test]
    fn test_recovery_empty_wal() {
        let (_temp_dir, config) = setup_test_wal();

        let recovery = RecoveryManager::new(config).expect("Failed to create recovery manager");
        let records = recovery.recover().expect("Failed to recover");

        assert!(records.is_empty());
    }

    #[test]
    fn test_recovery_standalone_records() {
        let (_temp_dir, config) = setup_test_wal();

        // Write standalone records (no transaction)
        {
            let mut writer = WalWriter::new(&config.wal_dir, config.max_segment_size, config.sync_mode)
                .expect("Failed to create writer");

            for i in 0..5 {
                let record = WalRecord::put(
                    format!("key{}", i).into_bytes(),
                    format!("value{}", i).into_bytes(),
                );
                writer.append(record).expect("Failed to append");
            }
            writer.sync().expect("Failed to sync");
        }

        let recovery = RecoveryManager::new(config).expect("Failed to create recovery manager");
        let records = recovery.recover().expect("Failed to recover");

        assert_eq!(records.len(), 5);
    }

    #[test]
    fn test_recovery_committed_transaction() {
        let (_temp_dir, config) = setup_test_wal();

        // Write a complete transaction
        {
            let mut writer = WalWriter::new(&config.wal_dir, config.max_segment_size, config.sync_mode)
                .expect("Failed to create writer");

            writer.append(WalRecord::begin_tx(1)).expect("Failed to append");
            writer
                .append(WalRecord::put(b"key1".to_vec(), b"val1".to_vec()))
                .expect("Failed to append");
            writer
                .append(WalRecord::put(b"key2".to_vec(), b"val2".to_vec()))
                .expect("Failed to append");
            writer.append(WalRecord::commit_tx(1)).expect("Failed to append");
            writer.sync().expect("Failed to sync");
        }

        let recovery = RecoveryManager::new(config).expect("Failed to create recovery manager");
        let records = recovery.recover().expect("Failed to recover");

        // Should have 2 PUT records (BEGIN and COMMIT are filtered out by recover())
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].record_type, RecordType::Put);
        assert_eq!(records[1].record_type, RecordType::Put);
    }

    #[test]
    fn test_recovery_incomplete_transaction_rollback() {
        let (_temp_dir, config) = setup_test_wal();

        // Write an incomplete transaction (no COMMIT)
        {
            let mut writer = WalWriter::new(&config.wal_dir, config.max_segment_size, config.sync_mode)
                .expect("Failed to create writer");

            writer.append(WalRecord::begin_tx(1)).expect("Failed to append");
            writer
                .append(WalRecord::put(b"key1".to_vec(), b"val1".to_vec()))
                .expect("Failed to append");
            writer
                .append(WalRecord::put(b"key2".to_vec(), b"val2".to_vec()))
                .expect("Failed to append");
            // NO COMMIT - simulating crash
            writer.sync().expect("Failed to sync");
        }

        let recovery = RecoveryManager::new(config).expect("Failed to create recovery manager");
        let records = recovery.recover().expect("Failed to recover");

        // Incomplete transaction should be rolled back - no records recovered
        assert_eq!(records.len(), 0);
    }

    #[test]
    fn test_recovery_mixed_committed_and_incomplete() {
        let (_temp_dir, config) = setup_test_wal();

        // Write one complete and one incomplete transaction
        {
            let mut writer = WalWriter::new(&config.wal_dir, config.max_segment_size, config.sync_mode)
                .expect("Failed to create writer");

            // Transaction 1: Complete
            writer.append(WalRecord::begin_tx(1)).expect("Failed to append");
            writer
                .append(WalRecord::put(b"key1".to_vec(), b"val1".to_vec()))
                .expect("Failed to append");
            writer.append(WalRecord::commit_tx(1)).expect("Failed to append");

            // Transaction 2: Incomplete
            writer.append(WalRecord::begin_tx(2)).expect("Failed to append");
            writer
                .append(WalRecord::put(b"key2".to_vec(), b"val2".to_vec()))
                .expect("Failed to append");
            // NO COMMIT
            writer.sync().expect("Failed to sync");
        }

        let recovery = RecoveryManager::new(config).expect("Failed to create recovery manager");
        let records = recovery.recover().expect("Failed to recover");

        // Only records from committed transaction
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn test_recovery_with_markers() {
        let (_temp_dir, config) = setup_test_wal();

        // Write a complete transaction
        {
            let mut writer = WalWriter::new(&config.wal_dir, config.max_segment_size, config.sync_mode)
                .expect("Failed to create writer");

            writer.append(WalRecord::begin_tx(1)).expect("Failed to append");
            writer
                .append(WalRecord::put(b"key1".to_vec(), b"val1".to_vec()))
                .expect("Failed to append");
            writer.append(WalRecord::commit_tx(1)).expect("Failed to append");
            writer.sync().expect("Failed to sync");
        }

        let recovery = RecoveryManager::new(config).expect("Failed to create recovery manager");
        let records = recovery.recover_with_markers().expect("Failed to recover");

        // Should have all 3 records including markers
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].record_type, RecordType::BeginTx);
        assert_eq!(records[1].record_type, RecordType::Put);
        assert_eq!(records[2].record_type, RecordType::CommitTx);
    }

    #[test]
    fn test_recovery_stats() {
        let (_temp_dir, config) = setup_test_wal();

        // Write various records
        {
            let mut writer = WalWriter::new(&config.wal_dir, config.max_segment_size, config.sync_mode)
                .expect("Failed to create writer");

            // Complete transaction
            writer.append(WalRecord::begin_tx(1)).expect("Failed to append");
            writer
                .append(WalRecord::put(b"k1".to_vec(), b"v1".to_vec()))
                .expect("Failed to append");
            writer.append(WalRecord::commit_tx(1)).expect("Failed to append");

            // Incomplete transaction
            writer.append(WalRecord::begin_tx(2)).expect("Failed to append");
            writer
                .append(WalRecord::delete(b"k2".to_vec()))
                .expect("Failed to append");

            writer.sync().expect("Failed to sync");
        }

        let recovery = RecoveryManager::new(config).expect("Failed to create recovery manager");
        let stats = recovery.get_stats().expect("Failed to get stats");

        assert_eq!(stats.total_records, 5);
        assert_eq!(stats.put_records, 1);
        assert_eq!(stats.delete_records, 1);
        assert_eq!(stats.transactions_started, 2);
        assert_eq!(stats.transactions_committed, 1);
        assert_eq!(stats.transactions_incomplete, 1);
    }
}
