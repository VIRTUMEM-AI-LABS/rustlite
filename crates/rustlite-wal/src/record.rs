// WAL record format and encoding/decoding
//
// Record format (binary):
// [length: u32 LE] [type: u8] [payload bytes] [crc32: u32 LE]
//
// Types:
// - PUT (1): key-value insert/update
// - DELETE (2): key deletion
// - BEGIN_TX (3): transaction start marker
// - COMMIT_TX (4): transaction commit marker
// - CHECKPOINT (5): checkpoint marker

use crc32fast::Hasher;
use rustlite_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// WAL record types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RecordType {
    Put = 1,
    Delete = 2,
    BeginTx = 3,
    CommitTx = 4,
    Checkpoint = 5,
}

impl TryFrom<u8> for RecordType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(RecordType::Put),
            2 => Ok(RecordType::Delete),
            3 => Ok(RecordType::BeginTx),
            4 => Ok(RecordType::CommitTx),
            5 => Ok(RecordType::Checkpoint),
            _ => Err(Error::InvalidOperation(format!(
                "Unknown WAL record type: {}",
                value
            ))),
        }
    }
}

/// WAL record payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordPayload {
    Put { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
    BeginTx { tx_id: u64 },
    CommitTx { tx_id: u64 },
    Checkpoint { sequence: u64 },
}

/// A WAL record
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalRecord {
    pub record_type: RecordType,
    pub payload: RecordPayload,
}

impl WalRecord {
    /// Create a new WAL record
    /// This is a convenience method for tests and simple usage
    pub fn new(record_type: RecordType, key: Vec<u8>, value: Vec<u8>) -> Self {
        match record_type {
            RecordType::Put => Self::put(key, value),
            RecordType::Delete => Self::delete(key),
            RecordType::BeginTx => Self::begin_tx(0), // Default tx_id
            RecordType::CommitTx => Self::commit_tx(0),
            RecordType::Checkpoint => Self::checkpoint(0),
        }
    }

    /// Create a PUT record
    pub fn put(key: Vec<u8>, value: Vec<u8>) -> Self {
        Self {
            record_type: RecordType::Put,
            payload: RecordPayload::Put { key, value },
        }
    }

    /// Create a DELETE record
    pub fn delete(key: Vec<u8>) -> Self {
        Self {
            record_type: RecordType::Delete,
            payload: RecordPayload::Delete { key },
        }
    }

    /// Create a BEGIN_TX record
    pub fn begin_tx(tx_id: u64) -> Self {
        Self {
            record_type: RecordType::BeginTx,
            payload: RecordPayload::BeginTx { tx_id },
        }
    }

    /// Create a COMMIT_TX record
    pub fn commit_tx(tx_id: u64) -> Self {
        Self {
            record_type: RecordType::CommitTx,
            payload: RecordPayload::CommitTx { tx_id },
        }
    }

    /// Create a CHECKPOINT record
    pub fn checkpoint(sequence: u64) -> Self {
        Self {
            record_type: RecordType::Checkpoint,
            payload: RecordPayload::Checkpoint { sequence },
        }
    }

    /// Encode record to bytes with framing and CRC
    /// Format: [length: u32 LE] [type: u8] [payload bytes] [crc32: u32 LE]
    pub fn encode(&self) -> Result<Vec<u8>> {
        // Serialize payload
        let payload_bytes = bincode::serialize(&self.payload)
            .map_err(|e| Error::Serialization(format!("Failed to serialize payload: {}", e)))?;

        let type_byte = self.record_type as u8;

        // Calculate length (type byte + payload)
        let content_len = 1 + payload_bytes.len();

        // Calculate CRC over type + payload
        let mut hasher = Hasher::new();
        hasher.update(&[type_byte]);
        hasher.update(&payload_bytes);
        let crc = hasher.finalize();

        // Build frame: [length][type][payload][crc]
        let mut frame = Vec::with_capacity(4 + content_len + 4);
        frame.extend_from_slice(&(content_len as u32).to_le_bytes());
        frame.push(type_byte);
        frame.extend_from_slice(&payload_bytes);
        frame.extend_from_slice(&crc.to_le_bytes());

        Ok(frame)
    }

    /// Decode record from bytes with validation
    pub fn decode(data: &[u8]) -> Result<(Self, usize)> {
        if data.len() < 9 {
            // Minimum: 4 (length) + 1 (type) + 0 (payload) + 4 (crc)
            return Err(Error::Serialization("Incomplete record frame".to_string()));
        }

        // Read length
        let length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

        // Check if we have the full record
        let total_size = 4 + length + 4; // length field + content + crc
        if data.len() < total_size {
            return Err(Error::Serialization(format!(
                "Incomplete record: expected {} bytes, got {}",
                total_size,
                data.len()
            )));
        }

        // Read type
        let type_byte = data[4];
        let record_type = RecordType::try_from(type_byte)?;

        // Read payload
        let payload_bytes = &data[5..4 + length];

        // Read CRC
        let crc_offset = 4 + length;
        let expected_crc = u32::from_le_bytes([
            data[crc_offset],
            data[crc_offset + 1],
            data[crc_offset + 2],
            data[crc_offset + 3],
        ]);

        // Validate CRC
        let mut hasher = Hasher::new();
        hasher.update(&[type_byte]);
        hasher.update(payload_bytes);
        let actual_crc = hasher.finalize();

        if actual_crc != expected_crc {
            return Err(Error::Storage(format!(
                "CRC mismatch: expected {}, got {}",
                expected_crc, actual_crc
            )));
        }

        // Deserialize payload
        let payload: RecordPayload = bincode::deserialize(payload_bytes)
            .map_err(|e| Error::Serialization(format!("Failed to deserialize payload: {}", e)))?;

        Ok((
            WalRecord {
                record_type,
                payload,
            },
            total_size,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_type_conversion() {
        assert_eq!(RecordType::try_from(1).unwrap(), RecordType::Put);
        assert_eq!(RecordType::try_from(2).unwrap(), RecordType::Delete);
        assert!(RecordType::try_from(99).is_err());
    }

    #[test]
    fn test_put_record_encode_decode() {
        let record = WalRecord::put(b"key1".to_vec(), b"value1".to_vec());

        let encoded = record.encode().unwrap();
        assert!(encoded.len() > 9); // Has minimum framing

        let (decoded, size) = WalRecord::decode(&encoded).unwrap();
        assert_eq!(decoded, record);
        assert_eq!(size, encoded.len());
    }

    #[test]
    fn test_delete_record_encode_decode() {
        let record = WalRecord::delete(b"key1".to_vec());

        let encoded = record.encode().unwrap();
        let (decoded, _) = WalRecord::decode(&encoded).unwrap();

        assert_eq!(decoded, record);
    }

    #[test]
    fn test_tx_records_encode_decode() {
        let begin = WalRecord::begin_tx(42);
        let commit = WalRecord::commit_tx(42);

        let begin_enc = begin.encode().unwrap();
        let commit_enc = commit.encode().unwrap();

        let (begin_dec, _) = WalRecord::decode(&begin_enc).unwrap();
        let (commit_dec, _) = WalRecord::decode(&commit_enc).unwrap();

        assert_eq!(begin_dec, begin);
        assert_eq!(commit_dec, commit);
    }

    #[test]
    fn test_checkpoint_record() {
        let record = WalRecord::checkpoint(1000);

        let encoded = record.encode().unwrap();
        let (decoded, _) = WalRecord::decode(&encoded).unwrap();

        assert_eq!(decoded, record);
    }

    #[test]
    fn test_crc_validation() {
        let record = WalRecord::put(b"key".to_vec(), b"value".to_vec());
        let mut encoded = record.encode().unwrap();

        // Corrupt the payload
        if encoded.len() > 10 {
            encoded[10] ^= 0xFF;
        }

        // Should fail CRC check
        let result = WalRecord::decode(&encoded);
        assert!(result.is_err());
    }

    #[test]
    fn test_incomplete_record() {
        let record = WalRecord::put(b"key".to_vec(), b"value".to_vec());
        let encoded = record.encode().unwrap();

        // Try to decode incomplete data
        let result = WalRecord::decode(&encoded[..5]);
        assert!(result.is_err());
    }
}
