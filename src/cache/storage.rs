//! Cache storage implementation

use crate::config::Config;
use crate::core::SourceLine;
use crate::error::{DuploError, Result};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

/// Current cache format version
const CACHE_VERSION: u32 = 1;

/// Cached source line data
#[derive(Debug, Serialize, Deserialize)]
struct CachedLine {
    /// The cleaned line text
    line: String,
    /// Original line number in the source file
    line_number: usize,
    /// Precomputed hash of the line
    hash: u32,
}

/// Cache entry for a single source file
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    /// Cache format version
    version: u32,
    /// Hash of the original file content
    content_hash: u64,
    /// Hash of the cleaning configuration
    config_hash: u64,
    /// Cached processed lines
    lines: Vec<CachedLine>,
}

/// File cache manager
pub struct FileCache {
    /// Directory where cache files are stored
    cache_dir: PathBuf,
    /// Cleaning config hash (for cache invalidation)
    config_hash: u64,
}

impl FileCache {
    /// Create a new FileCache
    ///
    /// # Arguments
    /// * `config` - Configuration containing cache settings
    ///
    /// # Returns
    /// A FileCache instance, or an error if the cache directory cannot be created
    pub fn new(config: &Config) -> Result<Self> {
        let cache_dir = config
            .cache_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(".duplo-cache"));

        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir).map_err(|e| {
                DuploError::CacheError(format!(
                    "Failed to create cache directory '{}': {}",
                    cache_dir.display(),
                    e
                ))
            })?;
        }

        let config_hash = config.cleaning_config_hash();

        Ok(Self {
            cache_dir,
            config_hash,
        })
    }

    /// Get the cache file path for a source file
    fn cache_path(&self, source_path: &str) -> PathBuf {
        // Create a hash-based filename to avoid path length issues
        let mut hasher = DefaultHasher::new();
        source_path.hash(&mut hasher);
        let path_hash = hasher.finish();
        self.cache_dir.join(format!("{:016x}.cache", path_hash))
    }

    /// Compute content hash of a file
    fn compute_content_hash(path: &str) -> Result<u64> {
        let content = fs::read(path).map_err(|e| DuploError::FileNotFound {
            path: path.to_string(),
            reason: e.to_string(),
        })?;

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Ok(hasher.finish())
    }

    /// Try to load cached lines for a file
    ///
    /// Returns None if the cache is invalid or doesn't exist
    pub fn get(&self, source_path: &str) -> Option<Vec<SourceLine>> {
        let cache_path = self.cache_path(source_path);

        // Check if cache file exists
        if !cache_path.exists() {
            return None;
        }

        // Load cache entry
        let file = File::open(&cache_path).ok()?;
        let reader = BufReader::new(file);
        let entry: CacheEntry = serde_json::from_reader(reader).ok()?;

        // Validate version
        if entry.version != CACHE_VERSION {
            return None;
        }

        // Validate config hash
        if entry.config_hash != self.config_hash {
            return None;
        }

        // Validate content hash
        let current_hash = Self::compute_content_hash(source_path).ok()?;
        if entry.content_hash != current_hash {
            return None;
        }

        // Convert cached lines to SourceLines
        let lines: Vec<SourceLine> = entry
            .lines
            .into_iter()
            .map(|cl| SourceLine::from_cached(cl.line, cl.line_number, cl.hash))
            .collect();

        Some(lines)
    }

    /// Store processed lines in the cache
    pub fn put(&self, source_path: &str, lines: &[SourceLine]) -> Result<()> {
        let cache_path = self.cache_path(source_path);
        let content_hash = Self::compute_content_hash(source_path)?;

        let cached_lines: Vec<CachedLine> = lines
            .iter()
            .map(|sl| CachedLine {
                line: sl.line().to_string(),
                line_number: sl.line_number(),
                hash: sl.hash(),
            })
            .collect();

        let entry = CacheEntry {
            version: CACHE_VERSION,
            content_hash,
            config_hash: self.config_hash,
            lines: cached_lines,
        };

        let file = File::create(&cache_path).map_err(|e| {
            DuploError::CacheError(format!(
                "Failed to create cache file '{}': {}",
                cache_path.display(),
                e
            ))
        })?;

        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &entry)
            .map_err(|e| DuploError::CacheError(format!("Failed to write cache entry: {}", e)))?;

        Ok(())
    }
}

/// Clear the cache directory
pub fn clear_cache(config: &Config) -> Result<()> {
    let cache_dir = config
        .cache_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from(".duplo-cache"));

    if cache_dir.exists() {
        // Remove all .cache files in the directory
        for entry in fs::read_dir(&cache_dir).map_err(|e| {
            DuploError::CacheError(format!(
                "Failed to read cache directory '{}': {}",
                cache_dir.display(),
                e
            ))
        })? {
            let entry = entry.map_err(|e| {
                DuploError::CacheError(format!("Failed to read cache entry: {}", e))
            })?;

            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "cache") {
                fs::remove_file(&path).map_err(|e| {
                    DuploError::CacheError(format!(
                        "Failed to remove cache file '{}': {}",
                        path.display(),
                        e
                    ))
                })?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_config(cache_dir: &Path) -> Config {
        let mut config = Config::default();
        config.cache_enabled = true;
        config.cache_dir = Some(cache_dir.to_path_buf());
        config
    }

    #[test]
    fn test_cache_roundtrip() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        let config = create_test_config(&cache_dir);

        // Create a test source file
        let source_path = temp.path().join("test.c");
        let mut f = File::create(&source_path).unwrap();
        writeln!(f, "int x = 5;").unwrap();
        writeln!(f, "int y = 10;").unwrap();

        let cache = FileCache::new(&config).unwrap();

        // Initially, cache should be empty
        assert!(cache.get(source_path.to_str().unwrap()).is_none());

        // Store some lines
        let lines = vec![
            SourceLine::new("int x = 5;".to_string(), 1),
            SourceLine::new("int y = 10;".to_string(), 2),
        ];
        cache.put(source_path.to_str().unwrap(), &lines).unwrap();

        // Should be able to retrieve them
        let retrieved = cache.get(source_path.to_str().unwrap()).unwrap();
        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved[0].line(), "int x = 5;");
        assert_eq!(retrieved[0].line_number(), 1);
    }

    #[test]
    fn test_cache_invalidation_on_content_change() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        let config = create_test_config(&cache_dir);

        let source_path = temp.path().join("test.c");
        {
            let mut f = File::create(&source_path).unwrap();
            writeln!(f, "original content").unwrap();
        }

        let cache = FileCache::new(&config).unwrap();
        let lines = vec![SourceLine::new("original content".to_string(), 1)];
        cache.put(source_path.to_str().unwrap(), &lines).unwrap();

        // Verify cache hit
        assert!(cache.get(source_path.to_str().unwrap()).is_some());

        // Modify the file
        {
            let mut f = File::create(&source_path).unwrap();
            writeln!(f, "modified content").unwrap();
        }

        // Cache should be invalidated
        assert!(cache.get(source_path.to_str().unwrap()).is_none());
    }

    #[test]
    fn test_cache_invalidation_on_config_change() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");

        let source_path = temp.path().join("test.c");
        {
            let mut f = File::create(&source_path).unwrap();
            writeln!(f, "test content").unwrap();
        }

        // Create cache with min_chars = 3
        let mut config1 = create_test_config(&cache_dir);
        config1.min_chars = 3;
        let cache1 = FileCache::new(&config1).unwrap();
        let lines = vec![SourceLine::new("test content".to_string(), 1)];
        cache1.put(source_path.to_str().unwrap(), &lines).unwrap();

        // Verify cache hit with same config
        assert!(cache1.get(source_path.to_str().unwrap()).is_some());

        // Create cache with different min_chars
        let mut config2 = create_test_config(&cache_dir);
        config2.min_chars = 5;
        let cache2 = FileCache::new(&config2).unwrap();

        // Cache should be invalidated due to config change
        assert!(cache2.get(source_path.to_str().unwrap()).is_none());
    }

    #[test]
    fn test_clear_cache() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        let config = create_test_config(&cache_dir);

        let source_path = temp.path().join("test.c");
        {
            let mut f = File::create(&source_path).unwrap();
            writeln!(f, "test").unwrap();
        }

        let cache = FileCache::new(&config).unwrap();
        let lines = vec![SourceLine::new("test".to_string(), 1)];
        cache.put(source_path.to_str().unwrap(), &lines).unwrap();

        // Verify cache exists
        assert!(cache.get(source_path.to_str().unwrap()).is_some());

        // Clear the cache
        clear_cache(&config).unwrap();

        // Cache should be empty
        assert!(cache.get(source_path.to_str().unwrap()).is_none());
    }
}
