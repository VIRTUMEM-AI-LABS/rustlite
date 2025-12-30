#![no_main]

use libfuzzer_sys::fuzz_target;
use rustlite::Database;

fuzz_target!(|data: &[u8]| {
    // Test various key/value size combinations
    if data.len() < 4 {
        return;
    }
    
    let key_len = u16::from_le_bytes([data[0], data[1]]) as usize % 2048;
    let value_len = u16::from_le_bytes([data[2], data[3]]) as usize % 2048;
    
    if data.len() < 4 + key_len + value_len {
        return;
    }
    
    let key = &data[4..4 + key_len];
    let value = &data[4 + key_len..4 + key_len + value_len];
    
    if let Ok(db) = Database::in_memory() {
        // Test put
        let _ = db.put(key, value);
        
        // Test get
        let _ = db.get(key);
        
        // Test delete
        let _ = db.delete(key);
        
        // Test get after delete
        let _ = db.get(key);
    }
    
    // Test edge cases
    if let Ok(db) = Database::in_memory() {
        // Empty key
        let _ = db.put(b"", value);
        let _ = db.get(b"");
        
        // Empty value
        let _ = db.put(key, b"");
        
        // Both empty
        let _ = db.put(b"", b"");
        let _ = db.get(b"");
    }
});
