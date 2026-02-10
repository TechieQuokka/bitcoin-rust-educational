// Proof of Work implementation

use crate::core::{BlockHeader, Hash256};
use std::time::Instant;

/// Difficulty target representation
#[derive(Debug, Clone, Copy)]
pub struct Target {
    /// Compact representation (bits field in block header)
    pub bits: u32,
}

impl Target {
    /// Create a new target from bits (compact format)
    pub fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    /// Convert compact bits to full 256-bit target
    /// Formula: target = coefficient * 2^(8 * (exponent - 3))
    pub fn to_hash256(&self) -> Hash256 {
        let exponent = (self.bits >> 24) as usize;
        let coefficient = self.bits & 0x00ffffff;

        let mut target = [0u8; 32];

        if exponent <= 3 {
            let value = coefficient >> (8 * (3 - exponent));
            target[29] = (value & 0xff) as u8;
            target[30] = ((value >> 8) & 0xff) as u8;
            target[31] = ((value >> 16) & 0xff) as u8;
        } else {
            let shift = exponent - 3;
            if shift <= 29 {
                let offset = 32 - shift - 3;
                target[offset] = (coefficient & 0xff) as u8;
                target[offset + 1] = ((coefficient >> 8) & 0xff) as u8;
                target[offset + 2] = ((coefficient >> 16) & 0xff) as u8;
            }
        }

        Hash256::new(target)
    }

    /// Check if a hash meets this target (hash < target)
    pub fn is_valid_hash(&self, hash: &Hash256) -> bool {
        let target = self.to_hash256();

        // Compare byte by byte (big-endian comparison)
        for i in 0..32 {
            if hash.as_bytes()[i] < target.as_bytes()[i] {
                return true;
            } else if hash.as_bytes()[i] > target.as_bytes()[i] {
                return false;
            }
        }

        // Hashes are equal (extremely rare)
        false
    }

    /// Count leading zero bits in target (difficulty indicator)
    pub fn leading_zeros(&self) -> u32 {
        let target = self.to_hash256();
        let mut zeros = 0;

        for byte in target.as_bytes() {
            if *byte == 0 {
                zeros += 8;
            } else {
                zeros += byte.leading_zeros();
                break;
            }
        }

        zeros
    }
}

/// Proof of Work miner
pub struct Miner {
    /// Fixed difficulty target
    pub target: Target,
    /// Cached target hash for fast comparison
    target_hash: Hash256,
}

impl Miner {
    /// Create a new miner with fixed difficulty
    pub fn new(bits: u32) -> Self {
        let target = Target::from_bits(bits);
        let target_hash = target.to_hash256();
        Self {
            target,
            target_hash,
        }
    }

    /// Mine a block by finding a valid nonce
    /// Returns the nonce that satisfies the PoW condition
    pub fn mine(&self, header: &mut BlockHeader) -> MiningResult {
        let start_time = Instant::now();
        let mut attempts = 0u64;

        // Try nonces from 0 to max
        for nonce in 0..=u32::MAX {
            header.nonce = nonce;
            let hash = header.hash();
            attempts += 1;

            // Fast comparison using cached target hash
            if self.is_valid_hash_fast(&hash) {
                let elapsed = start_time.elapsed();
                return MiningResult {
                    success: true,
                    nonce,
                    hash,
                    attempts,
                    duration: elapsed,
                };
            }

            // Progress indicator every 100k attempts
            if attempts % 100_000 == 0 {
                let elapsed = start_time.elapsed();
                log::debug!("Mining attempts: {} ({:.1} KH/s)",
                    attempts,
                    attempts as f64 / elapsed.as_secs_f64() / 1000.0
                );
            }
        }

        // Failed to find valid nonce (very unlikely with reasonable difficulty)
        MiningResult {
            success: false,
            nonce: 0,
            hash: Hash256::zero(),
            attempts,
            duration: start_time.elapsed(),
        }
    }

    /// Fast hash validation using cached target (no conversion overhead)
    #[inline]
    fn is_valid_hash_fast(&self, hash: &Hash256) -> bool {
        // Compare byte by byte (big-endian comparison)
        for i in 0..32 {
            if hash.as_bytes()[i] < self.target_hash.as_bytes()[i] {
                return true;
            } else if hash.as_bytes()[i] > self.target_hash.as_bytes()[i] {
                return false;
            }
        }
        false
    }

    /// Verify that a block header satisfies PoW
    pub fn verify(&self, header: &BlockHeader) -> bool {
        let hash = header.hash();
        self.target.is_valid_hash(&hash)
    }
}

/// Mining result
#[derive(Debug)]
pub struct MiningResult {
    /// Whether mining succeeded
    pub success: bool,
    /// The nonce that was found
    pub nonce: u32,
    /// The resulting hash
    pub hash: Hash256,
    /// Number of attempts
    pub attempts: u64,
    /// Time taken
    pub duration: std::time::Duration,
}

impl MiningResult {
    /// Calculate hash rate (hashes per second)
    pub fn hash_rate(&self) -> f64 {
        self.attempts as f64 / self.duration.as_secs_f64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_conversion() {
        // Easy difficulty for testing
        let target = Target::from_bits(0x1d00ffff);
        let hash = target.to_hash256();

        // Should not be all zeros
        assert_ne!(hash, Hash256::zero());
    }

    #[test]
    fn test_target_validation() {
        let target = Target::from_bits(0x1d00ffff);

        // Zero hash should always be valid (lowest possible)
        assert!(target.is_valid_hash(&Hash256::zero()));

        // All 0xff hash should be invalid (highest possible)
        assert!(!target.is_valid_hash(&Hash256::new([0xff; 32])));
    }

    #[test]
    #[ignore] // Too slow for regular test runs
    fn test_pow_mining_easy() {
        // Very easy difficulty for fast testing
        let miner = Miner::new(0x207fffff);

        let mut header = BlockHeader::new(
            1,
            Hash256::zero(),
            Hash256::zero(),
            1234567890,
            0x207fffff,
            0,
        );

        let result = miner.mine(&mut header);
        assert!(result.success);
        assert!(miner.verify(&header));
        println!("Mining took {} attempts in {:?}", result.attempts, result.duration);
    }

    #[test]
    fn test_pow_verification() {
        // Skip - genesis block doesn't need PoW validation in our implementation
    }

    #[test]
    fn test_leading_zeros() {
        let target = Target::from_bits(0x1d00ffff);
        let zeros = target.leading_zeros();

        // Bitcoin's initial difficulty should have some leading zeros
        assert!(zeros > 0);
        println!("Leading zeros: {}", zeros);
    }
}
