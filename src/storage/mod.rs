// Storage layer for blockchain and UTXO set

mod blockchain_db;
mod utxo_set;

pub use blockchain_db::BlockchainDB;
pub use utxo_set::{UtxoSet, Utxo, OutPoint};

use std::path::Path;

/// Storage manager - combines blockchain DB and UTXO set
pub struct Storage {
    pub blockchain: BlockchainDB,
    pub utxo_set: UtxoSet,
}

impl Storage {
    /// Create a new storage instance
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let blockchain = BlockchainDB::new(path.as_ref().join("blocks"))?;
        let utxo_set = UtxoSet::new(path.as_ref().join("utxo"))?;

        Ok(Self {
            blockchain,
            utxo_set,
        })
    }

    /// Create an in-memory storage (for testing)
    pub fn memory() -> Result<Self, String> {
        let blockchain = BlockchainDB::memory()?;
        let utxo_set = UtxoSet::memory()?;

        Ok(Self {
            blockchain,
            utxo_set,
        })
    }
}
