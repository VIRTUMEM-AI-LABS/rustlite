// Common test utilities for WAL integration tests

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test fixture that creates a temporary WAL directory
pub struct WalTestFixture {
    #[allow(dead_code)]
    pub temp_dir: TempDir,
    pub wal_path: PathBuf,
}

impl WalTestFixture {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let wal_path = temp_dir.path().join("wal");
        fs::create_dir_all(&wal_path).expect("Failed to create WAL directory");

        Self { temp_dir, wal_path }
    }

    pub fn wal_dir(&self) -> &PathBuf {
        &self.wal_path
    }

    #[allow(dead_code)]
    pub fn list_segments(&self) -> Vec<String> {
        fs::read_dir(&self.wal_path)
            .expect("Failed to read WAL directory")
            .filter_map(|entry| {
                entry
                    .ok()
                    .and_then(|e| e.file_name().to_str().map(String::from))
            })
            .collect()
    }
}

impl Default for WalTestFixture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_creates_wal_dir() {
        let fixture = WalTestFixture::new();
        assert!(fixture.wal_dir().exists());
        assert!(fixture.wal_dir().is_dir());
    }
}
