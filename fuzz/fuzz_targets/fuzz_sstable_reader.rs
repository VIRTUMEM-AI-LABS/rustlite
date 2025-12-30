#![no_main]

use libfuzzer_sys::fuzz_target;
use rustlite_storage::sstable::SSTableReader;
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    // Limit input size
    if data.len() > 10_000_000 {
        return;
    }
    
    // Write to temporary file and try to open as SSTable
    if let Ok(mut temp_file) = tempfile::NamedTempFile::new() {
        if temp_file.write_all(data).is_ok() {
            let path = temp_file.path();
            let _ = SSTableReader::open(path);
        }
    }
});
