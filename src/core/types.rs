// Basic types for Bitcoin blockchain

use std::fmt;

/// 256-bit hash type (32 bytes)
/// Used for block hashes, transaction IDs, and Merkle roots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Hash256(pub [u8; 32]);

impl Hash256 {
    /// Create a new Hash256 from a byte array
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create a Hash256 from a slice
    pub fn from_slice(slice: &[u8]) -> Result<Self, String> {
        if slice.len() != 32 {
            return Err(format!("Invalid hash length: expected 32, got {}", slice.len()));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }

    /// Get the hash as a byte slice
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create zero hash (used for genesis block's prev_block_hash)
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    /// Convert to hex string (reversed for display, Bitcoin convention)
    pub fn to_hex(&self) -> String {
        let mut reversed = self.0;
        reversed.reverse();
        hex::encode(reversed)
    }

    /// Create from hex string (expects reversed byte order)
    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| format!("Invalid hex string: {}", e))?;
        if bytes.len() != 32 {
            return Err(format!("Invalid hash length: expected 32, got {}", bytes.len()));
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        hash.reverse(); // Bitcoin uses reversed byte order for display
        Ok(Self(hash))
    }
}

impl fmt::Display for Hash256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash256_creation() {
        let hash = Hash256::new([1u8; 32]);
        assert_eq!(hash.as_bytes(), &[1u8; 32]);
    }

    #[test]
    fn test_hash256_zero() {
        let zero = Hash256::zero();
        assert_eq!(zero.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_hash256_hex() {
        let hash = Hash256::new([0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                                 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
                                 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00,
                                 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        let hex = hash.to_hex();
        let decoded = Hash256::from_hex(&hex).unwrap();
        assert_eq!(hash, decoded);
    }
}
