// Transaction data structures

use crate::core::{Hash256, hash256, Serializable};
use std::io::{Write, Read, Cursor};
use super::serialize::{write_varint, read_varint, write_var_bytes, read_var_bytes};

/// Transaction input - references a previous transaction output
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxInput {
    /// Hash of the previous transaction
    pub prev_tx_hash: Hash256,
    /// Index of the output in the previous transaction
    pub prev_index: u32,
    /// Signature script (scriptSig) - proves ownership
    pub script_sig: Vec<u8>,
    /// Sequence number (used for locktime, usually 0xffffffff)
    pub sequence: u32,
}

impl TxInput {
    /// Create a new transaction input
    pub fn new(prev_tx_hash: Hash256, prev_index: u32, script_sig: Vec<u8>) -> Self {
        Self {
            prev_tx_hash,
            prev_index,
            script_sig,
            sequence: 0xffffffff,
        }
    }

    /// Create a coinbase input (for mining rewards)
    pub fn coinbase(script_sig: Vec<u8>, _height: u32) -> Self {
        // Coinbase uses zero hash and max index
        Self {
            prev_tx_hash: Hash256::zero(),
            prev_index: 0xffffffff,
            script_sig,
            sequence: 0xffffffff,
        }
    }

    /// Check if this is a coinbase input
    pub fn is_coinbase(&self) -> bool {
        self.prev_tx_hash == Hash256::zero() && self.prev_index == 0xffffffff
    }

    /// Serialize the input
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_all(self.prev_tx_hash.as_bytes()).unwrap();
        buf.write_all(&self.prev_index.to_le_bytes()).unwrap();
        write_var_bytes(&mut buf, &self.script_sig).unwrap();
        buf.write_all(&self.sequence.to_le_bytes()).unwrap();
        buf
    }

    /// Deserialize the input
    pub fn deserialize(reader: &mut dyn Read) -> Result<Self, String> {
        let mut hash_bytes = [0u8; 32];
        reader.read_exact(&mut hash_bytes).map_err(|e| e.to_string())?;
        let prev_tx_hash = Hash256::new(hash_bytes);

        let mut index_bytes = [0u8; 4];
        reader.read_exact(&mut index_bytes).map_err(|e| e.to_string())?;
        let prev_index = u32::from_le_bytes(index_bytes);

        let script_sig = read_var_bytes(reader).map_err(|e| e.to_string())?;

        let mut sequence_bytes = [0u8; 4];
        reader.read_exact(&mut sequence_bytes).map_err(|e| e.to_string())?;
        let sequence = u32::from_le_bytes(sequence_bytes);

        Ok(Self {
            prev_tx_hash,
            prev_index,
            script_sig,
            sequence,
        })
    }
}

/// Transaction output - specifies amount and recipient
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxOutput {
    /// Amount in satoshis (1 BTC = 100,000,000 satoshis)
    pub value: u64,
    /// Public key script (scriptPubKey) - specifies conditions for spending
    pub script_pubkey: Vec<u8>,
}

impl TxOutput {
    /// Create a new transaction output
    pub fn new(value: u64, script_pubkey: Vec<u8>) -> Self {
        Self {
            value,
            script_pubkey,
        }
    }

    /// Serialize the output
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_all(&self.value.to_le_bytes()).unwrap();
        write_var_bytes(&mut buf, &self.script_pubkey).unwrap();
        buf
    }

    /// Deserialize the output
    pub fn deserialize(reader: &mut dyn Read) -> Result<Self, String> {
        let mut value_bytes = [0u8; 8];
        reader.read_exact(&mut value_bytes).map_err(|e| e.to_string())?;
        let value = u64::from_le_bytes(value_bytes);

        let script_pubkey = read_var_bytes(reader).map_err(|e| e.to_string())?;

        Ok(Self {
            value,
            script_pubkey,
        })
    }
}

/// Transaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    /// Transaction version
    pub version: u32,
    /// Transaction inputs
    pub inputs: Vec<TxInput>,
    /// Transaction outputs
    pub outputs: Vec<TxOutput>,
    /// Lock time (block height or timestamp when tx becomes valid)
    pub lock_time: u32,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(inputs: Vec<TxInput>, outputs: Vec<TxOutput>) -> Self {
        Self {
            version: 1,
            inputs,
            outputs,
            lock_time: 0,
        }
    }

    /// Create a coinbase transaction (mining reward)
    pub fn coinbase(script_sig: Vec<u8>, output: TxOutput, height: u32) -> Self {
        Self {
            version: 1,
            inputs: vec![TxInput::coinbase(script_sig, height)],
            outputs: vec![output],
            lock_time: 0,
        }
    }

    /// Check if this is a coinbase transaction
    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1 && self.inputs[0].is_coinbase()
    }

    /// Calculate transaction ID (double SHA256 of serialized tx)
    pub fn txid(&self) -> Hash256 {
        let serialized = self.serialize();
        hash256(&serialized)
    }

    /// Calculate total input value (requires UTXO set lookup in real impl)
    pub fn total_input_value(&self) -> u64 {
        // Note: In a real implementation, we'd need to look up the UTXO set
        // For coinbase transactions, input value is 0
        if self.is_coinbase() {
            0
        } else {
            // This is a placeholder - actual value needs UTXO lookup
            0
        }
    }

    /// Calculate total output value
    pub fn total_output_value(&self) -> u64 {
        self.outputs.iter().map(|out| out.value).sum()
    }
}

impl Transaction {
    /// Deserialize from a reader (optimized for streaming)
    pub fn from_reader(reader: &mut dyn Read) -> Result<Self, String> {
        // Version
        let mut version_bytes = [0u8; 4];
        reader.read_exact(&mut version_bytes).map_err(|e| e.to_string())?;
        let version = u32::from_le_bytes(version_bytes);

        // Input count
        let input_count = read_varint(reader).map_err(|e| e.to_string())? as usize;

        // Inputs
        let mut inputs = Vec::with_capacity(input_count);
        for _ in 0..input_count {
            inputs.push(TxInput::deserialize(reader)?);
        }

        // Output count
        let output_count = read_varint(reader).map_err(|e| e.to_string())? as usize;

        // Outputs
        let mut outputs = Vec::with_capacity(output_count);
        for _ in 0..output_count {
            outputs.push(TxOutput::deserialize(reader)?);
        }

        // Lock time
        let mut lock_time_bytes = [0u8; 4];
        reader.read_exact(&mut lock_time_bytes).map_err(|e| e.to_string())?;
        let lock_time = u32::from_le_bytes(lock_time_bytes);

        Ok(Self {
            version,
            inputs,
            outputs,
            lock_time,
        })
    }
}

impl Serializable for Transaction {
    fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Version
        buf.write_all(&self.version.to_le_bytes()).unwrap();

        // Input count
        write_varint(&mut buf, self.inputs.len() as u64).unwrap();

        // Inputs
        for input in &self.inputs {
            buf.write_all(&input.serialize()).unwrap();
        }

        // Output count
        write_varint(&mut buf, self.outputs.len() as u64).unwrap();

        // Outputs
        for output in &self.outputs {
            buf.write_all(&output.serialize()).unwrap();
        }

        // Lock time
        buf.write_all(&self.lock_time.to_le_bytes()).unwrap();

        buf
    }

    fn deserialize(data: &[u8]) -> Result<Self, String> {
        let mut cursor = Cursor::new(data);
        Self::from_reader(&mut cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coinbase_input() {
        let input = TxInput::coinbase(vec![1, 2, 3], 0);
        assert!(input.is_coinbase());
        assert_eq!(input.prev_tx_hash, Hash256::zero());
        assert_eq!(input.prev_index, 0xffffffff);
    }

    #[test]
    fn test_transaction_serialization() {
        let input = TxInput::coinbase(vec![1, 2, 3], 0);
        let output = TxOutput::new(5000000000, vec![4, 5, 6]);
        let tx = Transaction::new(vec![input], vec![output]);

        let serialized = tx.serialize();
        let deserialized = Transaction::deserialize(&serialized).unwrap();

        assert_eq!(tx, deserialized);
    }

    #[test]
    fn test_txid() {
        let input = TxInput::coinbase(vec![1, 2, 3], 0);
        let output = TxOutput::new(5000000000, vec![4, 5, 6]);
        let tx = Transaction::new(vec![input], vec![output]);

        let txid = tx.txid();
        assert_eq!(txid.as_bytes().len(), 32);

        // Same transaction should have same txid
        let txid2 = tx.txid();
        assert_eq!(txid, txid2);
    }

    #[test]
    fn test_coinbase_transaction() {
        let output = TxOutput::new(5000000000, vec![1, 2, 3]);
        let tx = Transaction::coinbase(vec![4, 5, 6], output, 0);

        assert!(tx.is_coinbase());
        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(tx.outputs.len(), 1);
    }
}
