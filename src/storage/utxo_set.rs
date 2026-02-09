// UTXO (Unspent Transaction Output) set management

use crate::core::{Hash256, TxOutput};
use sled::Db;
use std::path::Path;

/// UTXO identifier - transaction hash + output index
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutPoint {
    pub txid: Hash256,
    pub vout: u32,
}

impl OutPoint {
    pub fn new(txid: Hash256, vout: u32) -> Self {
        Self { txid, vout }
    }

    /// Serialize to bytes for database key
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(36);
        bytes.extend_from_slice(self.txid.as_bytes());
        bytes.extend_from_slice(&self.vout.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != 36 {
            return Err(format!("Invalid outpoint bytes length: {}", bytes.len()));
        }

        let mut txid_bytes = [0u8; 32];
        txid_bytes.copy_from_slice(&bytes[0..32]);
        let txid = Hash256::new(txid_bytes);

        let mut vout_bytes = [0u8; 4];
        vout_bytes.copy_from_slice(&bytes[32..36]);
        let vout = u32::from_le_bytes(vout_bytes);

        Ok(Self { txid, vout })
    }
}

/// UTXO - contains the output and metadata
#[derive(Debug, Clone)]
pub struct Utxo {
    pub output: TxOutput,
    pub height: u32,      // Block height where this UTXO was created
    pub is_coinbase: bool, // Whether this is a coinbase output
}

impl Utxo {
    pub fn new(output: TxOutput, height: u32, is_coinbase: bool) -> Self {
        Self {
            output,
            height,
            is_coinbase,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Serialize output
        bytes.extend_from_slice(&self.output.serialize());

        // Add metadata
        bytes.extend_from_slice(&self.height.to_le_bytes());
        bytes.push(if self.is_coinbase { 1 } else { 0 });

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 13 {
            // Minimum: 8 (value) + 1 (script len) + 4 (height) + 1 (coinbase flag)
            return Err("UTXO data too short".to_string());
        }

        // Deserialize output (all except last 5 bytes)
        let output_bytes = &bytes[..bytes.len() - 5];
        let mut cursor = std::io::Cursor::new(output_bytes);
        let output = TxOutput::deserialize(&mut cursor)?;

        // Deserialize metadata (last 5 bytes)
        let metadata_start = bytes.len() - 5;
        let mut height_bytes = [0u8; 4];
        height_bytes.copy_from_slice(&bytes[metadata_start..metadata_start + 4]);
        let height = u32::from_le_bytes(height_bytes);

        let is_coinbase = bytes[bytes.len() - 1] != 0;

        Ok(Self {
            output,
            height,
            is_coinbase,
        })
    }
}

/// UTXO set database
pub struct UtxoSet {
    db: Db,
}

impl UtxoSet {
    /// Create a new UTXO set
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let db = sled::open(path).map_err(|e| format!("Failed to open UTXO db: {}", e))?;
        Ok(Self { db })
    }

    /// Create an in-memory UTXO set (for testing)
    pub fn memory() -> Result<Self, String> {
        let config = sled::Config::new().temporary(true);
        let db = config.open().map_err(|e| format!("Failed to create memory UTXO db: {}", e))?;
        Ok(Self { db })
    }

    /// Add a UTXO
    pub fn add_utxo(&self, outpoint: &OutPoint, utxo: &Utxo) -> Result<(), String> {
        let key = outpoint.to_bytes();
        let value = utxo.to_bytes();

        self.db
            .insert(key, value)
            .map_err(|e| format!("Failed to add UTXO: {}", e))?;

        self.db
            .flush()
            .map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(())
    }

    /// Get a UTXO
    pub fn get_utxo(&self, outpoint: &OutPoint) -> Result<Option<Utxo>, String> {
        let key = outpoint.to_bytes();

        match self.db.get(&key).map_err(|e| format!("Database error: {}", e))? {
            Some(data) => {
                let utxo = Utxo::from_bytes(&data)?;
                Ok(Some(utxo))
            }
            None => Ok(None),
        }
    }

    /// Remove a UTXO (spent)
    pub fn remove_utxo(&self, outpoint: &OutPoint) -> Result<bool, String> {
        let key = outpoint.to_bytes();

        let existed = self
            .db
            .remove(&key)
            .map_err(|e| format!("Failed to remove UTXO: {}", e))?
            .is_some();

        self.db
            .flush()
            .map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(existed)
    }

    /// Check if a UTXO exists
    pub fn has_utxo(&self, outpoint: &OutPoint) -> Result<bool, String> {
        let key = outpoint.to_bytes();
        self.db
            .contains_key(&key)
            .map_err(|e| format!("Database error: {}", e))
    }

    /// Get all UTXOs (for balance calculation)
    pub fn get_all_utxos(&self) -> Result<Vec<(OutPoint, Utxo)>, String> {
        let mut utxos = Vec::new();

        for item in self.db.iter() {
            let (key, value) = item.map_err(|e| format!("Iterator error: {}", e))?;

            let outpoint = OutPoint::from_bytes(&key)?;
            let utxo = Utxo::from_bytes(&value)?;

            utxos.push((outpoint, utxo));
        }

        Ok(utxos)
    }

    /// Get balance for a script pubkey
    pub fn get_balance(&self, script_pubkey: &[u8]) -> Result<u64, String> {
        let mut balance = 0u64;

        for item in self.db.iter() {
            let (_, value) = item.map_err(|e| format!("Iterator error: {}", e))?;
            let utxo = Utxo::from_bytes(&value)?;

            if utxo.output.script_pubkey == script_pubkey {
                balance += utxo.output.value;
            }
        }

        Ok(balance)
    }

    /// Get all UTXOs for a script pubkey
    pub fn get_utxos_for_script(&self, script_pubkey: &[u8]) -> Result<Vec<(OutPoint, Utxo)>, String> {
        let mut utxos = Vec::new();

        for item in self.db.iter() {
            let (key, value) = item.map_err(|e| format!("Iterator error: {}", e))?;

            let utxo = Utxo::from_bytes(&value)?;

            if utxo.output.script_pubkey == script_pubkey {
                let outpoint = OutPoint::from_bytes(&key)?;
                utxos.push((outpoint, utxo));
            }
        }

        Ok(utxos)
    }

    /// Count total UTXOs
    pub fn count(&self) -> Result<usize, String> {
        self.db
            .len()
            .try_into()
            .map_err(|e| format!("Failed to get UTXO count: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outpoint_serialization() {
        let outpoint = OutPoint::new(Hash256::new([1; 32]), 42);

        let bytes = outpoint.to_bytes();
        assert_eq!(bytes.len(), 36);

        let decoded = OutPoint::from_bytes(&bytes).unwrap();
        assert_eq!(outpoint, decoded);
    }

    #[test]
    fn test_utxo_serialization() {
        let output = TxOutput::new(1000, vec![1, 2, 3, 4]);
        let utxo = Utxo::new(output.clone(), 10, false);

        let bytes = utxo.to_bytes();
        let decoded = Utxo::from_bytes(&bytes).unwrap();

        assert_eq!(utxo.output.value, decoded.output.value);
        assert_eq!(utxo.output.script_pubkey, decoded.output.script_pubkey);
        assert_eq!(utxo.height, decoded.height);
        assert_eq!(utxo.is_coinbase, decoded.is_coinbase);
    }

    #[test]
    fn test_add_and_get_utxo() {
        let utxo_set = UtxoSet::memory().unwrap();

        let outpoint = OutPoint::new(Hash256::new([1; 32]), 0);
        let output = TxOutput::new(5000, vec![1, 2, 3]);
        let utxo = Utxo::new(output, 1, false);

        // Add UTXO
        utxo_set.add_utxo(&outpoint, &utxo).unwrap();

        // Get UTXO
        let retrieved = utxo_set.get_utxo(&outpoint).unwrap().unwrap();
        assert_eq!(utxo.output.value, retrieved.output.value);
    }

    #[test]
    fn test_remove_utxo() {
        let utxo_set = UtxoSet::memory().unwrap();

        let outpoint = OutPoint::new(Hash256::new([1; 32]), 0);
        let output = TxOutput::new(5000, vec![1, 2, 3]);
        let utxo = Utxo::new(output, 1, false);

        // Add UTXO
        utxo_set.add_utxo(&outpoint, &utxo).unwrap();
        assert!(utxo_set.has_utxo(&outpoint).unwrap());

        // Remove UTXO
        let existed = utxo_set.remove_utxo(&outpoint).unwrap();
        assert!(existed);
        assert!(!utxo_set.has_utxo(&outpoint).unwrap());
    }

    #[test]
    fn test_get_balance() {
        let utxo_set = UtxoSet::memory().unwrap();

        let script_pubkey = vec![1, 2, 3, 4];

        // Add multiple UTXOs for the same script
        let outpoint1 = OutPoint::new(Hash256::new([1; 32]), 0);
        let utxo1 = Utxo::new(TxOutput::new(1000, script_pubkey.clone()), 1, false);
        utxo_set.add_utxo(&outpoint1, &utxo1).unwrap();

        let outpoint2 = OutPoint::new(Hash256::new([2; 32]), 0);
        let utxo2 = Utxo::new(TxOutput::new(2000, script_pubkey.clone()), 2, false);
        utxo_set.add_utxo(&outpoint2, &utxo2).unwrap();

        // Check balance
        let balance = utxo_set.get_balance(&script_pubkey).unwrap();
        assert_eq!(balance, 3000);
    }

    #[test]
    fn test_count() {
        let utxo_set = UtxoSet::memory().unwrap();

        assert_eq!(utxo_set.count().unwrap(), 0);

        // Add UTXOs
        let outpoint1 = OutPoint::new(Hash256::new([1; 32]), 0);
        let utxo1 = Utxo::new(TxOutput::new(1000, vec![1, 2, 3]), 1, false);
        utxo_set.add_utxo(&outpoint1, &utxo1).unwrap();

        assert_eq!(utxo_set.count().unwrap(), 1);

        let outpoint2 = OutPoint::new(Hash256::new([2; 32]), 0);
        let utxo2 = Utxo::new(TxOutput::new(2000, vec![4, 5, 6]), 2, false);
        utxo_set.add_utxo(&outpoint2, &utxo2).unwrap();

        assert_eq!(utxo_set.count().unwrap(), 2);
    }
}
