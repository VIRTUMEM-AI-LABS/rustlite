#![no_main]

use libfuzzer_sys::fuzz_target;
use rustlite_core::query::parser::Parser;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string (ignore invalid UTF-8)
    if let Ok(sql) = std::str::from_utf8(data) {
        // Limit query length to prevent timeout
        if sql.len() > 10_000 {
            return;
        }
        
        // Try to parse the SQL - should never panic
        if let Ok(mut parser) = Parser::new(sql) {
            let _ = parser.parse();
        }
    }
});
