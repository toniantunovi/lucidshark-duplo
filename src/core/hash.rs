//! FNV-1a hash implementation compatible with C++ Duplo
//!
//! This module implements the Fowler-Noll-Vo hash function (FNV-1a variant)
//! with the same parameters as the original C++ Duplo implementation to ensure
//! hash compatibility.

/// FNV-1a offset basis (32-bit)
const FNV_OFFSET_BASIS: u32 = 2_166_136_261;

/// FNV-1a prime (32-bit)
const FNV_PRIME: u32 = 16_777_619;

/// Compute FNV-1a hash for a byte slice
///
/// This implementation matches the C++ Duplo HashUtil::Hash function exactly.
///
/// # Arguments
/// * `data` - The byte slice to hash
///
/// # Returns
/// The 32-bit FNV-1a hash value
#[inline]
pub fn fnv1a_hash(data: &[u8]) -> u32 {
    let mut hash = FNV_OFFSET_BASIS;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Compute hash for a source line with whitespace normalization
///
/// This function filters out whitespace and control characters before hashing,
/// keeping only characters with ASCII value > 32 (space).
///
/// # Arguments
/// * `line` - The source line to hash
///
/// # Returns
/// The 32-bit FNV-1a hash of the whitespace-normalized line
pub fn hash_line(line: &str) -> u32 {
    let clean: Vec<u8> = line.bytes().filter(|&b| b > b' ').collect();
    fnv1a_hash(&clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_hash() {
        // Empty input should return the offset basis
        assert_eq!(fnv1a_hash(&[]), FNV_OFFSET_BASIS);
    }

    #[test]
    fn test_simple_hash() {
        // Test with a simple string
        let hash = fnv1a_hash(b"hello");
        assert_ne!(hash, FNV_OFFSET_BASIS);
        // Verify consistency
        assert_eq!(hash, fnv1a_hash(b"hello"));
    }

    #[test]
    fn test_hash_line_strips_whitespace() {
        // Lines with different whitespace should hash the same
        let hash1 = hash_line("int x = 5;");
        let hash2 = hash_line("int  x  =  5;");
        let hash3 = hash_line("int\tx\t=\t5;");
        let hash4 = hash_line("  int x = 5;  ");

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
        assert_eq!(hash3, hash4);
    }

    #[test]
    fn test_hash_line_different_content() {
        // Different content should hash differently
        let hash1 = hash_line("int x = 5;");
        let hash2 = hash_line("int y = 5;");

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_line_empty() {
        // Empty line (all whitespace) should hash to offset basis
        assert_eq!(hash_line(""), FNV_OFFSET_BASIS);
        assert_eq!(hash_line("   "), FNV_OFFSET_BASIS);
        assert_eq!(hash_line("\t\t"), FNV_OFFSET_BASIS);
    }
}
