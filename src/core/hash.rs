// Hashing utilities for Bitcoin

use sha2::{Sha256, Digest};
use crate::core::Hash256;

/// SHA256 double hash (Bitcoin convention)
/// hash256 = SHA256(SHA256(data))
pub fn hash256(data: &[u8]) -> Hash256 {
    let first_hash = Sha256::digest(data);
    let second_hash = Sha256::digest(&first_hash);
    Hash256::from_slice(&second_hash).expect("SHA256 always returns 32 bytes")
}

/// Single SHA256 hash
pub fn sha256_hash(data: &[u8]) -> [u8; 32] {
    let hash = Sha256::digest(data);
    let mut result = [0u8; 32];
    result.copy_from_slice(&hash);
    result
}

/// RIPEMD160(SHA256(data)) - used for address generation
pub fn hash160(data: &[u8]) -> [u8; 20] {
    use ripemd::{Ripemd160, Digest as RipemdDigest};
    let sha = Sha256::digest(data);
    let ripemd = Ripemd160::digest(&sha);
    let mut result = [0u8; 20];
    result.copy_from_slice(&ripemd);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash256() {
        let data = b"hello world";
        let hash = hash256(data);
        assert_eq!(hash.as_bytes().len(), 32);

        // Same data should produce same hash
        let hash2 = hash256(data);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hash256_empty() {
        let data = b"";
        let hash = hash256(data);
        assert_eq!(hash.as_bytes().len(), 32);
    }

    #[test]
    fn test_hash160() {
        let data = b"test data";
        let hash = hash160(data);
        assert_eq!(hash.len(), 20);
    }
}
