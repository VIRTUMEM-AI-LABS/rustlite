#![no_main]

use libfuzzer_sys::fuzz_target;
use rustlite_wal::record::WalRecord;

fuzz_target!(|data: &[u8]| {
    // Limit input size to prevent timeout
    if data.len() > 1_000_000 {
        return;
    }
    
    // Try to decode WAL record - should never panic
    let _ = WalRecord::decode(data);
});
