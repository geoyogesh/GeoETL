//! Object store integration for reading CSV files
//!
//! This module provides utilities for reading CSV files from various
//! object store backends (local filesystem, S3, cloud storage, etc.)
//! through `DataFusion`'s object store abstraction.

use datafusion_common::Result;

/// Metadata about a CSV file in an object store
#[derive(Debug, Clone)]
pub struct CsvFileMetadata {
    /// File path/location
    pub location: String,
    /// File size in bytes
    pub size: usize,
    /// Last modified timestamp (if available)
    pub last_modified: Option<i64>,
}

impl CsvFileMetadata {
    /// Create new metadata
    pub fn new(location: impl Into<String>, size: usize) -> Self {
        Self {
            location: location.into(),
            size,
            last_modified: None,
        }
    }

    /// Set last modified timestamp
    #[must_use]
    pub fn with_last_modified(mut self, timestamp: i64) -> Self {
        self.last_modified = Some(timestamp);
        self
    }

    /// Check if file is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

/// Helper to construct object store URLs
#[allow(dead_code)]
pub fn make_url(scheme: &str, bucket: &str, path: &str) -> String {
    if scheme.is_empty() {
        // Local filesystem
        path.to_string()
    } else {
        // Remote object store (s3, gs, etc.)
        let trimmed = path.trim_start_matches('/');
        format!("{scheme}://{bucket}/{trimmed}")
    }
}

/// Parse an object store URL into components
#[allow(dead_code)]
pub fn parse_url(url: &str) -> Result<(Option<String>, String)> {
    if url.contains("://") {
        let parts: Vec<&str> = url.splitn(2, "://").collect();
        if parts.len() == 2 {
            Ok((Some(parts[0].to_string()), parts[1].to_string()))
        } else {
            Err(datafusion_common::DataFusionError::Plan(format!(
                "Invalid URL format: {url}"
            )))
        }
    } else {
        // Local path
        Ok((None, url.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_file_metadata() {
        let meta = CsvFileMetadata::new("test.csv", 1024).with_last_modified(1_234_567_890);

        assert_eq!(meta.location, "test.csv");
        assert_eq!(meta.size, 1024);
        assert_eq!(meta.last_modified, Some(1_234_567_890));
        assert!(!meta.is_empty());

        let empty_meta = CsvFileMetadata::new("empty.csv", 0);
        assert!(empty_meta.is_empty());
    }

    #[test]
    fn test_make_url() {
        assert_eq!(make_url("", "", "data/test.csv"), "data/test.csv");
        assert_eq!(
            make_url("s3", "my-bucket", "data/test.csv"),
            "s3://my-bucket/data/test.csv"
        );
        assert_eq!(
            make_url("s3", "my-bucket", "/data/test.csv"),
            "s3://my-bucket/data/test.csv"
        );
    }

    #[test]
    fn test_parse_url() {
        let (scheme, path) = parse_url("s3://bucket/path/file.csv").unwrap();
        assert_eq!(scheme, Some("s3".to_string()));
        assert_eq!(path, "bucket/path/file.csv");

        let (scheme, path) = parse_url("local/path/file.csv").unwrap();
        assert_eq!(scheme, None);
        assert_eq!(path, "local/path/file.csv");
    }
}
