#![no_main]

use libfuzzer_sys::fuzz_target;
use rustlite::Database;
use arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
enum DbOp {
    Put { key: Vec<u8>, value: Vec<u8> },
    Get { key: Vec<u8> },
    Delete { key: Vec<u8> },
}

fuzz_target!(|ops: Vec<DbOp>| {
    // Create in-memory database for fast fuzzing
    if let Ok(db) = Database::in_memory() {
        for op in ops.iter().take(100) { // Limit operations to prevent timeout
            match op {
                DbOp::Put { key, value } => {
                    // Limit key/value sizes
                    if key.len() <= 1024 && value.len() <= 1024 {
                        let _ = db.put(key, value);
                    }
                }
                DbOp::Get { key } => {
                    if key.len() <= 1024 {
                        let _ = db.get(key);
                    }
                }
                DbOp::Delete { key } => {
                    if key.len() <= 1024 {
                        let _ = db.delete(key);
                    }
                }
            }
        }
    }
});
