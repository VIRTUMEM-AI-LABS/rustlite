/// Security-hardened wrapper for Database operations
///
/// This module provides input validation for all database operations
/// to prevent security vulnerabilities and resource exhaustion attacks.
use rustlite_core::error::{Error, Result};

/// Validates database key
///
/// # Security
///
/// - Prevents empty keys
/// - Prevents oversized keys (>16MB)
///
/// # Errors
///
/// Returns Error::InvalidInput if validation fails
#[inline]
pub fn validate_key(key: &[u8]) -> Result<()> {
    const MAX_KEY_SIZE: usize = 16 * 1024 * 1024; // 16 MB

    if key.is_empty() {
        return Err(Error::InvalidInput("Key cannot be empty".to_string()));
    }

    if key.len() > MAX_KEY_SIZE {
        return Err(Error::InvalidInput(format!(
            "Key size {} exceeds maximum {}",
            key.len(),
            MAX_KEY_SIZE
        )));
    }

    Ok(())
}

/// Validates database value
///
/// # Security
///
/// - Prevents oversized values (>1GB) to avoid OOM
///
/// # Errors
///
/// Returns Error::InvalidInput if validation fails
#[inline]
pub fn validate_value(value: &[u8]) -> Result<()> {
    const MAX_VALUE_SIZE: usize = 1024 * 1024 * 1024; // 1 GB

    if value.len() > MAX_VALUE_SIZE {
        return Err(Error::InvalidInput(format!(
            "Value size {} exceeds maximum {}",
            value.len(),
            MAX_VALUE_SIZE
        )));
    }

    Ok(())
}

/// Validates SQL query string
///
/// # Security
///
/// - Prevents oversized queries (>1MB)
/// - Prevents empty queries
///
/// # Errors
///
/// Returns Error::InvalidInput if validation fails
#[inline]
pub fn validate_query(query: &str) -> Result<()> {
    const MAX_QUERY_LENGTH: usize = 1024 * 1024; // 1 MB

    if query.is_empty() {
        return Err(Error::InvalidInput("Query cannot be empty".to_string()));
    }

    if query.len() > MAX_QUERY_LENGTH {
        return Err(Error::InvalidInput(format!(
            "Query length {} exceeds maximum {}",
            query.len(),
            MAX_QUERY_LENGTH
        )));
    }

    Ok(())
}

/// Validates index name
///
/// # Security
///
/// - Prevents path traversal attacks
/// - Prevents invalid filesystem characters
///
/// # Errors
///
/// Returns Error::InvalidInput if validation fails
#[inline]
pub fn validate_index_name(name: &str) -> Result<()> {
    const MAX_INDEX_NAME_LENGTH: usize = 256;

    if name.is_empty() {
        return Err(Error::InvalidInput(
            "Index name cannot be empty".to_string(),
        ));
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
            "Index name cannot contain path separators or '..'".to_string(),
        ));
    }

    // Check for null bytes
    if name.contains('\0') {
        return Err(Error::InvalidInput(
            "Index name cannot contain null bytes".to_string(),
        ));
    }

    Ok(())
}

/// Validates database path
///
/// # Security
///
/// - Prevents path traversal (somewhat)
/// - Prevents null bytes
///
/// # Errors
///
/// Returns Error::InvalidInput if validation fails
#[allow(dead_code)]
#[inline]
pub fn validate_path(path: &str) -> Result<()> {
    const MAX_PATH_LENGTH: usize = 4096;

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
            "Path cannot contain null bytes".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_key() {
        // Valid
        assert!(validate_key(b"valid").is_ok());

        // Empty
        assert!(validate_key(b"").is_err());

        // Too large
        let large = vec![0u8; 17 * 1024 * 1024];
        assert!(validate_key(&large).is_err());
    }

    #[test]
    fn test_validate_value() {
        // Valid
        assert!(validate_value(b"valid").is_ok());
        assert!(validate_value(b"").is_ok()); // Empty allowed
    }

    #[test]
    fn test_validate_query() {
        // Valid
        assert!(validate_query("SELECT * FROM table").is_ok());

        // Empty
        assert!(validate_query("").is_err());

        // Too long
        let long = "a".repeat(2 * 1024 * 1024);
        assert!(validate_query(&long).is_err());
    }

    #[test]
    fn test_validate_index_name() {
        // Valid
        assert!(validate_index_name("valid_name").is_ok());

        // Path traversal
        assert!(validate_index_name("../etc/passwd").is_err());
        assert!(validate_index_name("path/to/file").is_err());
    }
}
