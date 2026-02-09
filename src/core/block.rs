// Block data structures

use crate::core::{Hash256, hash256, Transaction, Serializable};
use std::io::{Write, Read, Cursor};
use super::serialize::{write_varint, read_varint};

/// Block header - 80 bytes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockHeader {
    /// Block version
    pub version: u32,
    /// Hash of the previous block
    pub prev_block_hash: Hash256,
    /// Merkle root of all transactions in the block
    pub merkle_root: Hash256,
    /// Block timestamp (Unix epoch)
    pub timestamp: u32,
    /// Difficulty target (compact format)
    pub bits: u32,
    /// Nonce for proof-of-work
    pub nonce: u32,
}

impl BlockHeader {
    /// Create a new block header
    pub fn new(
        version: u32,
        prev_block_hash: Hash256,
        merkle_root: Hash256,
        timestamp: u32,
        bits: u32,
        nonce: u32,
    ) -> Self {
        Self {
            version,
            prev_block_hash,
            merkle_root,
            timestamp,
            bits,
            nonce,
        }
    }

    /// Calculate the hash of this block header
    pub fn hash(&self) -> Hash256 {
        let serialized = self.serialize();
        hash256(&serialized)
    }

    /// Serialize the block header (always 80 bytes)
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(80);
        buf.write_all(&self.version.to_le_bytes()).unwrap();
        buf.write_all(self.prev_block_hash.as_bytes()).unwrap();
        buf.write_all(self.merkle_root.as_bytes()).unwrap();
        buf.write_all(&self.timestamp.to_le_bytes()).unwrap();
        buf.write_all(&self.bits.to_le_bytes()).unwrap();
        buf.write_all(&self.nonce.to_le_bytes()).unwrap();
        buf
    }

    /// Deserialize a block header
    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        if data.len() < 80 {
            return Err(format!("Block header too short: {} bytes", data.len()));
        }

        let mut cursor = Cursor::new(data);

        let mut version_bytes = [0u8; 4];
        cursor.read_exact(&mut version_bytes).map_err(|e| e.to_string())?;
        let version = u32::from_le_bytes(version_bytes);

        let mut prev_hash_bytes = [0u8; 32];
        cursor.read_exact(&mut prev_hash_bytes).map_err(|e| e.to_string())?;
        let prev_block_hash = Hash256::new(prev_hash_bytes);

        let mut merkle_bytes = [0u8; 32];
        cursor.read_exact(&mut merkle_bytes).map_err(|e| e.to_string())?;
        let merkle_root = Hash256::new(merkle_bytes);

        let mut timestamp_bytes = [0u8; 4];
        cursor.read_exact(&mut timestamp_bytes).map_err(|e| e.to_string())?;
        let timestamp = u32::from_le_bytes(timestamp_bytes);

        let mut bits_bytes = [0u8; 4];
        cursor.read_exact(&mut bits_bytes).map_err(|e| e.to_string())?;
        let bits = u32::from_le_bytes(bits_bytes);

        let mut nonce_bytes = [0u8; 4];
        cursor.read_exact(&mut nonce_bytes).map_err(|e| e.to_string())?;
        let nonce = u32::from_le_bytes(nonce_bytes);

        Ok(Self {
            version,
            prev_block_hash,
            merkle_root,
            timestamp,
            bits,
            nonce,
        })
    }
}

/// Block - contains header and transactions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    /// Transactions in this block
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Create a new block
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self {
            header,
            transactions,
        }
    }

    /// Create the genesis block
    pub fn genesis() -> Self {
        // Genesis block coinbase
        let coinbase_sig = b"The Times 03/Jan/2009 Chancellor on brink of second bailout for banks".to_vec();
        let coinbase_output = crate::core::TxOutput::new(
            5000000000, // 50 BTC in satoshis
            vec![], // Empty scriptPubKey for now
        );
        let coinbase_tx = Transaction::coinbase(coinbase_sig, coinbase_output, 0);

        let merkle_root = Self::calculate_merkle_root(&[coinbase_tx.clone()]);

        // Use very easy difficulty for educational purposes
        let header = BlockHeader::new(
            1,                    // version
            Hash256::zero(),     // no previous block
            merkle_root,
            1231006505,          // Jan 3, 2009 timestamp
            0x20ffffff,          // very easy difficulty for testing
            2,                   // genesis nonce
        );

        Self {
            header,
            transactions: vec![coinbase_tx],
        }
    }

    /// Calculate Merkle root from transactions
    pub fn calculate_merkle_root(transactions: &[Transaction]) -> Hash256 {
        if transactions.is_empty() {
            return Hash256::zero();
        }

        // Get transaction IDs
        let mut hashes: Vec<Hash256> = transactions
            .iter()
            .map(|tx| tx.txid())
            .collect();

        // Build Merkle tree
        while hashes.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in hashes.chunks(2) {
                let left = chunk[0];
                let right = if chunk.len() == 2 { chunk[1] } else { chunk[0] };

                // Concatenate and hash
                let mut combined = Vec::new();
                combined.extend_from_slice(left.as_bytes());
                combined.extend_from_slice(right.as_bytes());
                next_level.push(hash256(&combined));
            }

            hashes = next_level;
        }

        hashes[0]
    }

    /// Get the block hash
    pub fn hash(&self) -> Hash256 {
        self.header.hash()
    }

    /// Get block height (not stored in block, determined by chain position)
    /// This is a placeholder - actual height is tracked by the blockchain
    pub fn height(&self) -> u32 {
        // Will be implemented when we have the blockchain structure
        0
    }

    /// Check if this is the genesis block
    pub fn is_genesis(&self) -> bool {
        self.header.prev_block_hash == Hash256::zero()
    }
}

impl Serializable for Block {
    fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Serialize header
        buf.write_all(&self.header.serialize()).unwrap();

        // Transaction count
        write_varint(&mut buf, self.transactions.len() as u64).unwrap();

        // Serialize transactions
        for tx in &self.transactions {
            buf.write_all(&tx.serialize()).unwrap();
        }

        buf
    }

    fn deserialize(data: &[u8]) -> Result<Self, String> {
        let mut cursor = Cursor::new(data);

        // Deserialize header (80 bytes)
        let mut header_bytes = [0u8; 80];
        cursor.read_exact(&mut header_bytes).map_err(|e| e.to_string())?;
        let header = BlockHeader::deserialize(&header_bytes)?;

        // Transaction count
        let tx_count = read_varint(&mut cursor).map_err(|e| e.to_string())? as usize;

        // Deserialize transactions
        let mut transactions = Vec::with_capacity(tx_count);
        for _ in 0..tx_count {
            // Read remaining bytes for transaction
            let mut remaining = Vec::new();
            cursor.read_to_end(&mut remaining).map_err(|e| e.to_string())?;

            let tx = Transaction::deserialize(&remaining)?;
            let consumed = tx.serialize().len();
            transactions.push(tx);

            // Reset cursor position for next transaction
            let new_pos = cursor.position() + consumed as u64;
            cursor.set_position(new_pos);
        }

        Ok(Self {
            header,
            transactions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_header_serialization() {
        let header = BlockHeader::new(
            1,
            Hash256::zero(),
            Hash256::zero(),
            1234567890,
            0x1d00ffff,
            0,
        );

        let serialized = header.serialize();
        assert_eq!(serialized.len(), 80);

        let deserialized = BlockHeader::deserialize(&serialized).unwrap();
        assert_eq!(header, deserialized);
    }

    #[test]
    fn test_block_hash() {
        let header = BlockHeader::new(
            1,
            Hash256::zero(),
            Hash256::zero(),
            1234567890,
            0x1d00ffff,
            0,
        );

        let hash = header.hash();
        assert_eq!(hash.as_bytes().len(), 32);

        // Same header should produce same hash
        let hash2 = header.hash();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_genesis_block() {
        let genesis = Block::genesis();
        assert!(genesis.is_genesis());
        assert_eq!(genesis.transactions.len(), 1);
        assert!(genesis.transactions[0].is_coinbase());
    }

    #[test]
    fn test_merkle_root_single_tx() {
        let output = crate::core::TxOutput::new(5000000000, vec![1, 2, 3]);
        let tx = Transaction::coinbase(vec![4, 5, 6], output, 0);

        let merkle = Block::calculate_merkle_root(&[tx.clone()]);
        let expected = tx.txid();
        assert_eq!(merkle, expected);
    }

    #[test]
    fn test_merkle_root_multiple_tx() {
        let tx1 = Transaction::coinbase(
            vec![1],
            crate::core::TxOutput::new(1000, vec![]),
            0
        );
        let tx2 = Transaction::coinbase(
            vec![2],
            crate::core::TxOutput::new(2000, vec![]),
            0
        );

        let merkle = Block::calculate_merkle_root(&[tx1, tx2]);
        assert_eq!(merkle.as_bytes().len(), 32);
    }
}
