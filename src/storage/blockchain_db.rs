// Blockchain database using sled

use crate::core::{Block, Hash256, Serializable};
use sled::Db;
use std::path::Path;

/// Blockchain database
pub struct BlockchainDB {
    db: Db,
}

impl BlockchainDB {
    /// Create a new blockchain database
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let db = sled::open(path).map_err(|e| format!("Failed to open database: {}", e))?;
        Ok(Self { db })
    }

    /// Create an in-memory database (for testing)
    pub fn memory() -> Result<Self, String> {
        let config = sled::Config::new().temporary(true);
        let db = config.open().map_err(|e| format!("Failed to create memory db: {}", e))?;
        Ok(Self { db })
    }

    /// Store a block
    pub fn store_block(&self, block: &Block) -> Result<(), String> {
        let hash = block.hash();
        let serialized = block.serialize();

        // Store block by hash (flush removed for performance)
        self.db
            .insert(Self::block_key(&hash), serialized.as_slice())
            .map_err(|e| format!("Failed to store block: {}", e))?;

        Ok(())
    }

    /// Get a block by hash
    pub fn get_block(&self, hash: &Hash256) -> Result<Option<Block>, String> {
        let key = Self::block_key(hash);

        match self.db.get(&key).map_err(|e| format!("Database error: {}", e))? {
            Some(data) => {
                let block = Block::deserialize(&data)?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }

    /// Store block height mapping (height -> hash)
    pub fn store_height(&self, height: u32, hash: &Hash256) -> Result<(), String> {
        let key = Self::height_key(height);

        self.db
            .insert(key, hash.as_bytes().as_slice())
            .map_err(|e| format!("Failed to store height: {}", e))?;

        Ok(())
    }

    /// Get block hash by height
    pub fn get_hash_by_height(&self, height: u32) -> Result<Option<Hash256>, String> {
        let key = Self::height_key(height);

        match self.db.get(&key).map_err(|e| format!("Database error: {}", e))? {
            Some(data) => {
                if data.len() != 32 {
                    return Err(format!("Invalid hash length: {}", data.len()));
                }
                let mut hash_bytes = [0u8; 32];
                hash_bytes.copy_from_slice(&data);
                Ok(Some(Hash256::new(hash_bytes)))
            }
            None => Ok(None),
        }
    }

    /// Get block by height
    pub fn get_block_by_height(&self, height: u32) -> Result<Option<Block>, String> {
        match self.get_hash_by_height(height)? {
            Some(hash) => self.get_block(&hash),
            None => Ok(None),
        }
    }

    /// Store the chain tip (best block hash)
    pub fn store_tip(&self, hash: &Hash256) -> Result<(), String> {
        self.db
            .insert(b"tip", hash.as_bytes().as_slice())
            .map_err(|e| format!("Failed to store tip: {}", e))?;

        Ok(())
    }

    /// Get the chain tip (best block hash)
    pub fn get_tip(&self) -> Result<Option<Hash256>, String> {
        match self.db.get(b"tip").map_err(|e| format!("Database error: {}", e))? {
            Some(data) => {
                if data.len() != 32 {
                    return Err(format!("Invalid hash length: {}", data.len()));
                }
                let mut hash_bytes = [0u8; 32];
                hash_bytes.copy_from_slice(&data);
                Ok(Some(Hash256::new(hash_bytes)))
            }
            None => Ok(None),
        }
    }

    /// Store the blockchain height
    pub fn store_chain_height(&self, height: u32) -> Result<(), String> {
        self.db
            .insert(b"height", &height.to_le_bytes())
            .map_err(|e| format!("Failed to store height: {}", e))?;

        Ok(())
    }

    /// Manually flush database (call after batch operations)
    pub fn flush(&self) -> Result<(), String> {
        self.db
            .flush()
            .map_err(|e| format!("Failed to flush: {}", e))?;
        Ok(())
    }

    /// Get the blockchain height
    pub fn get_chain_height(&self) -> Result<u32, String> {
        match self.db.get(b"height").map_err(|e| format!("Database error: {}", e))? {
            Some(data) => {
                if data.len() != 4 {
                    return Err(format!("Invalid height data length: {}", data.len()));
                }
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&data);
                Ok(u32::from_le_bytes(bytes))
            }
            None => Ok(0), // No blocks stored yet
        }
    }

    /// Check if a block exists
    pub fn has_block(&self, hash: &Hash256) -> Result<bool, String> {
        let key = Self::block_key(hash);
        self.db
            .contains_key(&key)
            .map_err(|e| format!("Database error: {}", e))
    }

    // Helper: create key for block storage
    fn block_key(hash: &Hash256) -> Vec<u8> {
        let mut key = Vec::with_capacity(33);
        key.push(b'b'); // 'b' for block
        key.extend_from_slice(hash.as_bytes());
        key
    }

    // Helper: create key for height index
    fn height_key(height: u32) -> Vec<u8> {
        let mut key = Vec::with_capacity(5);
        key.push(b'h'); // 'h' for height
        key.extend_from_slice(&height.to_le_bytes());
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_get_block() {
        let db = BlockchainDB::memory().unwrap();
        let block = Block::genesis();

        // Store block
        db.store_block(&block).unwrap();

        // Retrieve block
        let hash = block.hash();
        let retrieved = db.get_block(&hash).unwrap().unwrap();

        assert_eq!(block, retrieved);
    }

    #[test]
    fn test_height_index() {
        let db = BlockchainDB::memory().unwrap();
        let block = Block::genesis();
        let hash = block.hash();

        // Store block first
        db.store_block(&block).unwrap();

        // Store height mapping
        db.store_height(0, &hash).unwrap();

        // Retrieve by height
        let retrieved_hash = db.get_hash_by_height(0).unwrap().unwrap();
        assert_eq!(hash, retrieved_hash);

        let retrieved_block = db.get_block_by_height(0).unwrap().unwrap();
        assert_eq!(block.hash(), retrieved_block.hash());
    }

    #[test]
    fn test_tip_management() {
        let db = BlockchainDB::memory().unwrap();
        let block = Block::genesis();
        let hash = block.hash();

        // Store tip
        db.store_tip(&hash).unwrap();

        // Get tip
        let tip = db.get_tip().unwrap().unwrap();
        assert_eq!(hash, tip);
    }

    #[test]
    fn test_chain_height() {
        let db = BlockchainDB::memory().unwrap();

        // Initial height should be 0
        assert_eq!(db.get_chain_height().unwrap(), 0);

        // Store height
        db.store_chain_height(10).unwrap();
        assert_eq!(db.get_chain_height().unwrap(), 10);
    }

    #[test]
    fn test_has_block() {
        let db = BlockchainDB::memory().unwrap();
        let block = Block::genesis();
        let hash = block.hash();

        // Block doesn't exist yet
        assert!(!db.has_block(&hash).unwrap());

        // Store block
        db.store_block(&block).unwrap();

        // Block exists now
        assert!(db.has_block(&hash).unwrap());
    }
}
