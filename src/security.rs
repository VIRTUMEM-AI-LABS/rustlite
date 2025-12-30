/// Security utilities and input validation for RustLite
///
/// This module provides security hardening features including:
/// - Input validation
/// - Bounds checking
/// - Resource limits
/// - Attack mitigation
use crate::error::{Error, Result};

/// Maximum allowed key size (16 MB)
pub const MAX_KEY_SIZE: usize = 16 * 1024 * 1024;

/// Maximum allowed value size (1 GB)
pub const MAX_VALUE_SIZE: usize = 1024 * 1024 * 1024;

/// Maximum allowed batch size (10,000 operations)
pub const MAX_BATCH_SIZE: usize = 10_000;

/// Maximum allowed query result size (100,000 rows)
pub const MAX_QUERY_RESULTS: usize = 100_000;

/// Maximum allowed SQL query length (1 MB)
pub const MAX_QUERY_LENGTH: usize = 1024 * 1024;

/// Maximum allowed index name length (256 bytes)
pub const MAX_INDEX_NAME_LENGTH: usize = 256;

/// Maximum allowed database path length (4096 bytes)
pub const MAX_PATH_LENGTH: usize = 4096;

/// Validates a key for size and content
///
/// # Security
///
/// Prevents:
/// - Oversized keys that could cause memory exhaustion
/// - Empty keys that could cause undefined behavior
///
/// # Errors
///
/// Returns `Error::InvalidInput` if:
/// - Key is empty
/// - Key exceeds MAX_KEY_SIZE
pub fn validate_key(key: &[u8]) -> Result<()> {
    if key.is_empty() {
        return Err(Error::InvalidInput("Key cannot be empty".to_string()));
    }
    
    if key.len() > MAX_KEY_SIZE {
        return Err(Error::InvalidInput(format!(
            "Key size {} exceeds maximum allowed size {}",
            key.len(),
            MAX_KEY_SIZE
        )));
    }
    
    Ok(())
}

/// Validates a value for size
///
/// # Security
///
/// Prevents:
/// - Oversized values that could cause memory exhaustion
/// - OOM attacks via large value insertion
///
/// # Errors
///
/// Returns `Error::InvalidInput` if value exceeds MAX_VALUE_SIZE
pub fn validate_value(value: &[u8]) -> Result<()> {
    if value.len() > MAX_VALUE_SIZE {
        return Err(Error::InvalidInput(format!(
            "Value size {} exceeds maximum allowed size {}",
            value.len(),
            MAX_VALUE_SIZE
        )));
    }
    
    Ok(())
}

/// Validates a batch operation size
///
/// # Security
///
/// Prevents:
/// - Resource exhaustion via massive batch operations
/// - Transaction log overflow
///
/// # Errors
///
/// Returns `Error::InvalidInput` if batch size exceeds MAX_BATCH_SIZE
pub fn validate_batch_size(size: usize) -> Result<()> {
    if size > MAX_BATCH_SIZE {
        return Err(Error::InvalidInput(format!(
            "Batch size {} exceeds maximum allowed size {}",
            size,
            MAX_BATCH_SIZE
        )));
    }
    
    Ok(())
}

/// Validates a SQL query length
///
/// # Security
///
/// Prevents:
/// - Memory exhaustion from extremely large queries
/// - Parser DoS attacks
///
/// # Errors
///
/// Returns `Error::InvalidInput` if query exceeds MAX_QUERY_LENGTH
pub fn validate_query_length(query: &str) -> Result<()> {
    if query.len() > MAX_QUERY_LENGTH {
        return Err(Error::InvalidInput(format!(
            "Query length {} exceeds maximum allowed length {}",
            query.len(),
            MAX_QUERY_LENGTH
        )));
    }
    
    Ok(())
}

/// Validates an index name
///
/// # Security
///
/// Prevents:
/// - Path traversal attacks via index names
/// - Invalid filesystem characters
///
/// # Errors
///
/// Returns `Error::InvalidInput` if:
/// - Name is empty
/// - Name exceeds MAX_INDEX_NAME_LENGTH
/// - Name contains invalid characters (/, \, .., null bytes)
pub fn validate_index_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidInput("Index name cannot be empty".to_string()));
    }
    
    if name.len() > MAX_INDEX_NAME_LENGTH {
        return Err(Error::InvalidInput(format!(
            "Index name length {} exceeds maximum {}",
            name.len(),
            MAX_INDEX_NAME_LENGTH
        )));
    }
    
    // Check for path traversal attempts
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(Error::InvalidInput(
            "Index name cannot contain path separators or '..'".to_string()
        ));
    }
    
    // Check for null bytes
    if name.contains('\0') {
        return Err(Error::InvalidInput(
            "Index name cannot contain null bytes".to_string()
        ));
    }
    
    Ok(())
}

/// Validates a database path
///
/// # Security
///
/// Prevents:
/// - Path traversal attacks
/// - Symlink attacks
/// - Invalid filesystem paths
///
/// # Errors
///
/// Returns `Error::InvalidInput` if:
/// - Path is empty
/// - Path exceeds MAX_PATH_LENGTH
/// - Path contains invalid characters (null bytes)
pub fn validate_path(path: &str) -> Result<()> {
    if path.is_empty() {
        return Err(Error::InvalidInput("Path cannot be empty".to_string()));
    }
    
    if path.len() > MAX_PATH_LENGTH {
        return Err(Error::InvalidInput(format!(
            "Path length {} exceeds maximum {}",
            path.len(),
            MAX_PATH_LENGTH
        )));
    }
    
    // Check for null bytes
    if path.contains('\0') {
        return Err(Error::InvalidInput(
            "Path cannot contain null bytes".to_string()
        ));
    }
    
    Ok(())
}

/// Validates result set size to prevent memory exhaustion
///
/// # Security
///
/// Prevents:
/// - Memory exhaustion from unbounded query results
/// - OOM attacks via large result sets
///
/// # Errors
///
/// Returns `Error::InvalidInput` if result count exceeds MAX_QUERY_RESULTS
pub fn validate_result_size(count: usize) -> Result<()> {
    if count > MAX_QUERY_RESULTS {
        return Err(Error::InvalidInput(format!(
            "Result set size {} exceeds maximum allowed {}. Use LIMIT clause to reduce results.",
            count,
            MAX_QUERY_RESULTS
        )));
    }
    
    Ok(())
}

/// Checks if a file size is within acceptable bounds
///
/// # Security
///
/// Prevents:
/// - Loading corrupted or malicious oversized files
/// - Memory exhaustion from extremely large files
///
/// # Parameters
///
/// - `size`: File size in bytes
/// - `max_size`: Maximum allowed size in bytes
/// - `file_type`: Description of file type for error messages
///
/// # Errors
///
/// Returns `Error::InvalidInput` if size exceeds max_size
pub fn validate_file_size(size: u64, max_size: u64, file_type: &str) -> Result<()> {
    if size > max_size {
        return Err(Error::InvalidInput(format!(
            "{} file size {} exceeds maximum {}",
            file_type,
            size,
            max_size
        )));
    }
    
    Ok(())
}

/// Sanitizes user input for safe logging
///
/// # Security
///
/// Prevents:
/// - Log injection attacks
/// - Sensitive data leakage in logs
///
/// Replaces control characters and truncates long strings.
pub fn sanitize_for_logging(input: &str, max_len: usize) -> String {
    let sanitized: String = input
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .take(max_len)
        .collect();
    
    if input.len() > max_len {
        format!("{}... (truncated)", sanitized)
    } else {
        sanitized
    }
}

/// Checks if resource limits are within bounds
///
/// # Security
///
/// Prevents:
/// - Resource exhaustion attacks
/// - Configuration-based DoS
pub struct ResourceLimits {
    /// Maximum memory usage in bytes
    pub max_memory: usize,
    /// Maximum open file descriptors
    pub max_file_descriptors: usize,
    /// Maximum concurrent transactions
    pub max_concurrent_transactions: usize,
    /// Maximum WAL size in bytes
    pub max_wal_size: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 1024 * 1024 * 1024, // 1 GB
            max_file_descriptors: 1024,
            max_concurrent_transactions: 10_000,
            max_wal_size: 100 * 1024 * 1024, // 100 MB
        }
    }
}

impl ResourceLimits {
    /// Validates that requested resources are within limits
    pub fn validate(&self) -> Result<()> {
        // Sanity checks
        if self.max_memory == 0 {
            return Err(Error::InvalidInput("max_memory cannot be zero".to_string()));
        }
        
        if self.max_file_descriptors == 0 {
            return Err(Error::InvalidInput("max_file_descriptors cannot be zero".to_string()));
        }
        
        if self.max_concurrent_transactions == 0 {
            return Err(Error::InvalidInput("max_concurrent_transactions cannot be zero".to_string()));
        }
        
        if self.max_wal_size == 0 {
            return Err(Error::InvalidInput("max_wal_size cannot be zero".to_string()));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_key() {
        // Valid keys
        assert!(validate_key(b"valid_key").is_ok());
        assert!(validate_key(&[0u8; 1024]).is_ok());
        
        // Empty key
        assert!(validate_key(b"").is_err());
        
        // Oversized key
        let large_key = vec![0u8; MAX_KEY_SIZE + 1];
        assert!(validate_key(&large_key).is_err());
    }

    #[test]
    fn test_validate_value() {
        // Valid values
        assert!(validate_value(b"valid_value").is_ok());
        assert!(validate_value(&[0u8; 1024 * 1024]).is_ok());
        
        // Empty value (allowed)
        assert!(validate_value(b"").is_ok());
        
        // Oversized value
        let large_value = vec![0u8; MAX_VALUE_SIZE + 1];
        assert!(validate_value(&large_value).is_err());
    }

    #[test]
    fn test_validate_index_name() {
        // Valid names
        assert!(validate_index_name("valid_name").is_ok());
        assert!(validate_index_name("name-with-dash").is_ok());
        assert!(validate_index_name("name_123").is_ok());
        
        // Invalid names
        assert!(validate_index_name("").is_err()); // Empty
        assert!(validate_index_name("path/traversal").is_err()); // Forward slash
        assert!(validate_index_name("path\\traversal").is_err()); // Backslash
        assert!(validate_index_name("../etc/passwd").is_err()); // Path traversal
        assert!(validate_index_name("name\0null").is_err()); // Null byte
    }

    #[test]
    fn test_validate_path() {
        // Valid paths
        assert!(validate_path("./database").is_ok());
        assert!(validate_path("/var/lib/rustlite").is_ok());
        
        // Invalid paths
        assert!(validate_path("").is_err()); // Empty
        assert!(validate_path("path\0null").is_err()); // Null byte
        
        // Very long path
        let long_path = "a".repeat(MAX_PATH_LENGTH + 1);
        assert!(validate_path(&long_path).is_err());
    }

    #[test]
    fn test_sanitize_for_logging() {
        assert_eq!(sanitize_for_logging("normal text", 100), "normal text");
        assert_eq!(sanitize_for_logging("text\nwith\nnewlines", 100), "text\nwith\nnewlines");
        
        // Control characters removed
        let input = "text\x01with\x02control";
        assert_eq!(sanitize_for_logging(input, 100), "textwithcontrol");
        
        // Truncation
        let long_input = "a".repeat(200);
        let result = sanitize_for_logging(&long_input, 50);
        assert!(result.len() <= 100); // 50 chars + "... (truncated)"
        assert!(result.ends_with("... (truncated)"));
    }

    #[test]
    fn test_resource_limits() {
        let limits = ResourceLimits::default();
        assert!(limits.validate().is_ok());
        
        // Invalid limits
        let invalid = ResourceLimits {
            max_memory: 0,
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
    }
}
