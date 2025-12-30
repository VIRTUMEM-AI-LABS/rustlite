/// File format versions for RustLite (v1.0.0+)
///
/// This module defines version constants for all file formats to ensure
/// forward/backward compatibility and safe upgrades.
/// SSTable format version
pub const SSTABLE_FORMAT_VERSION: u16 = 1;

/// WAL format version  
pub const WAL_FORMAT_VERSION: u16 = 1;

/// Manifest format version
pub const MANIFEST_FORMAT_VERSION: u16 = 1;

/// Magic numbers for file validation
pub mod magic {
    /// SSTable magic: "RSTL" (RuSTLite)
    pub const SSTABLE: u32 = 0x5253544C;

    /// WAL magic: "RLWL" (RustLite WAL)
    pub const WAL: u32 = 0x524C574C;

    /// Manifest magic: "RLMF" (RustLite ManiFest)
    pub const MANIFEST: u32 = 0x524C4D46;
}

/// Version compatibility information
pub struct FormatVersion {
    /// Current version of this format
    pub current: u16,
    /// Minimum supported version for reading
    pub min_read: u16,
    /// Minimum supported version for writing
    pub min_write: u16,
}

impl FormatVersion {
    /// Check if a version can be read
    pub fn can_read(&self, version: u16) -> bool {
        version >= self.min_read && version <= self.current
    }

    /// Check if a version can be written
    pub fn can_write(&self, version: u16) -> bool {
        version >= self.min_write && version <= self.current
    }
}

/// SSTable format version info
pub fn sstable_version() -> FormatVersion {
    FormatVersion {
        current: SSTABLE_FORMAT_VERSION,
        min_read: 1,
        min_write: 1,
    }
}

/// WAL format version info
pub fn wal_version() -> FormatVersion {
    FormatVersion {
        current: WAL_FORMAT_VERSION,
        min_read: 1,
        min_write: 1,
    }
}

/// Manifest format version info
pub fn manifest_version() -> FormatVersion {
    FormatVersion {
        current: MANIFEST_FORMAT_VERSION,
        min_read: 1,
        min_write: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        let v = sstable_version();
        assert!(v.can_read(1));
        assert!(v.can_write(1));
        assert!(!v.can_read(0));
        assert!(!v.can_read(999));
    }
}
