// Transaction and block validation

use crate::core::{Block, BlockHeader, Transaction, Script};
use crate::consensus::pow::Miner;

/// Validation error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Block hash doesn't meet PoW target
    InvalidProofOfWork,
    /// Merkle root doesn't match calculated value
    InvalidMerkleRoot,
    /// Block has no transactions
    NoTransactions,
    /// First transaction is not coinbase
    MissingCoinbase,
    /// More than one coinbase transaction
    MultipleCoinbase,
    /// Transaction has no inputs or outputs
    EmptyTransaction,
    /// Transaction input script verification failed
    InvalidSignature,
    /// Coinbase transaction in non-first position
    CoinbaseNotFirst,
    /// Block timestamp is too far in the future
    InvalidTimestamp,
    /// Block version not supported
    InvalidVersion,
    /// Coinbase transaction must have exactly one input
    InvalidCoinbaseInputCount,
    /// Total output value exceeds the maximum allowed supply
    OutputValueExceedsMax,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ValidationError::InvalidProofOfWork => write!(f, "Invalid proof of work"),
            ValidationError::InvalidMerkleRoot => write!(f, "Invalid merkle root"),
            ValidationError::NoTransactions => write!(f, "Block has no transactions"),
            ValidationError::MissingCoinbase => write!(f, "Missing coinbase transaction"),
            ValidationError::MultipleCoinbase => write!(f, "Multiple coinbase transactions"),
            ValidationError::EmptyTransaction => write!(f, "Empty transaction"),
            ValidationError::InvalidSignature => write!(f, "Invalid signature"),
            ValidationError::CoinbaseNotFirst => write!(f, "Coinbase not in first position"),
            ValidationError::InvalidTimestamp => write!(f, "Invalid timestamp"),
            ValidationError::InvalidVersion => write!(f, "Invalid version"),
            ValidationError::InvalidCoinbaseInputCount => write!(f, "Coinbase must have exactly one input"),
            ValidationError::OutputValueExceedsMax => write!(f, "Total output value exceeds maximum supply"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Block validator
pub struct BlockValidator {
    /// PoW miner for verification
    miner: Miner,
}

impl BlockValidator {
    /// Create a new block validator with fixed difficulty
    pub fn new(bits: u32) -> Self {
        Self {
            miner: Miner::new(bits),
        }
    }

    /// Validate a block header
    pub fn validate_header(&self, header: &BlockHeader) -> Result<(), ValidationError> {
        // Skip PoW validation for genesis block (prev_hash is zero)
        use crate::core::Hash256;
        if header.prev_block_hash != Hash256::zero() {
            // Check proof of work for non-genesis blocks
            if !self.miner.verify(header) {
                return Err(ValidationError::InvalidProofOfWork);
            }
        }

        // Check version (must be >= 1)
        if header.version < 1 {
            return Err(ValidationError::InvalidVersion);
        }

        // Check timestamp (not too far in the future - within 2 hours)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;

        if header.timestamp > now + 2 * 60 * 60 {
            return Err(ValidationError::InvalidTimestamp);
        }

        Ok(())
    }

    /// Validate a complete block
    pub fn validate_block(&self, block: &Block) -> Result<(), ValidationError> {
        // Validate header
        self.validate_header(&block.header)?;

        // Must have at least one transaction
        if block.transactions.is_empty() {
            return Err(ValidationError::NoTransactions);
        }

        // First transaction must be coinbase
        if !block.transactions[0].is_coinbase() {
            return Err(ValidationError::MissingCoinbase);
        }

        // Only first transaction can be coinbase
        for tx in &block.transactions[1..] {
            if tx.is_coinbase() {
                return Err(ValidationError::CoinbaseNotFirst);
            }
        }

        // Check for multiple coinbase transactions
        let coinbase_count = block.transactions.iter()
            .filter(|tx| tx.is_coinbase())
            .count();

        if coinbase_count > 1 {
            return Err(ValidationError::MultipleCoinbase);
        }

        // Validate merkle root
        let calculated_merkle = Block::calculate_merkle_root(&block.transactions);
        if calculated_merkle != block.header.merkle_root {
            return Err(ValidationError::InvalidMerkleRoot);
        }

        // Validate all transactions
        for tx in &block.transactions {
            self.validate_transaction(tx)?;
        }

        Ok(())
    }

    /// Validate a transaction (basic checks)
    pub fn validate_transaction(&self, tx: &Transaction) -> Result<(), ValidationError> {
        // Must have inputs and outputs
        if tx.inputs.is_empty() || tx.outputs.is_empty() {
            return Err(ValidationError::EmptyTransaction);
        }

        // Coinbase transactions have special rules
        if tx.is_coinbase() {
            // Coinbase must have exactly 1 input
            if tx.inputs.len() != 1 {
                return Err(ValidationError::InvalidCoinbaseInputCount);
            }
            // Coinbase validation is simple - just structure check
            return Ok(());
        }

        // For non-coinbase transactions, we would need UTXO set to fully validate
        // For now, just do basic structure validation
        // Full validation will be implemented in Phase 3 with UTXO set

        Ok(())
    }

    /// Validate a transaction's signature (simplified for P2PKH)
    /// In a real implementation, this would be called during full validation with UTXO lookup
    pub fn validate_transaction_signature(
        &self,
        tx: &Transaction,
        input_index: usize,
        script_pubkey: &[u8],
    ) -> Result<(), ValidationError> {
        if input_index >= tx.inputs.len() {
            return Err(ValidationError::EmptyTransaction);
        }

        let input = &tx.inputs[input_index];

        // Skip coinbase inputs
        if input.is_coinbase() {
            return Ok(());
        }

        // Get transaction hash for signature verification
        let tx_hash = tx.txid();

        // Verify P2PKH script
        Script::verify_p2pkh(&input.script_sig, script_pubkey, tx_hash.as_bytes())
            .map_err(|_| ValidationError::InvalidSignature)?
            .then_some(())
            .ok_or(ValidationError::InvalidSignature)
    }
}

/// Transaction validator (for mempool validation)
pub struct TransactionValidator;

impl TransactionValidator {
    /// Validate a transaction for mempool acceptance
    pub fn validate_for_mempool(tx: &Transaction) -> Result<(), ValidationError> {
        // Must have inputs and outputs
        if tx.inputs.is_empty() || tx.outputs.is_empty() {
            return Err(ValidationError::EmptyTransaction);
        }

        // Cannot be coinbase
        if tx.is_coinbase() {
            return Err(ValidationError::CoinbaseNotFirst);
        }

        // Check that total output doesn't exceed reasonable limits
        let total_output = tx.total_output_value();
        const MAX_MONEY: u64 = 21_000_000 * 100_000_000; // 21M BTC in satoshis

        if total_output > MAX_MONEY {
            return Err(ValidationError::OutputValueExceedsMax);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{TxOutput, Hash256};

    #[test]
    fn test_validate_genesis_block() {
        let validator = BlockValidator::new(0x1d00ffff);
        let genesis = Block::genesis();

        // Genesis block should be valid
        assert!(validator.validate_block(&genesis).is_ok());
    }

    #[test]
    fn test_validate_header_pow() {
        let validator = BlockValidator::new(0x20ffffff);

        // Genesis header (PoW skipped)
        let genesis_header = BlockHeader::new(
            1,
            Hash256::zero(),
            Hash256::zero(),
            1231006505,
            0x20ffffff,
            0,
        );

        // Should pass - genesis blocks skip PoW validation
        assert!(validator.validate_header(&genesis_header).is_ok());

        // Non-genesis header without valid PoW
        let invalid_header = BlockHeader::new(
            1,
            Hash256::new([1; 32]), // Non-zero prev hash
            Hash256::zero(),
            1234567890,
            0x20ffffff,
            0, // Wrong nonce
        );

        // Should fail - non-genesis blocks require PoW
        assert_eq!(
            validator.validate_header(&invalid_header),
            Err(ValidationError::InvalidProofOfWork)
        );
    }

    #[test]
    fn test_validate_block_no_transactions() {
        let validator = BlockValidator::new(0x20ffffff);

        let header = BlockHeader::new(
            1,
            Hash256::zero(), // Genesis
            Hash256::zero(),
            1234567890,
            0x20ffffff,
            0,
        );

        let block = Block::new(header, vec![]);

        assert_eq!(
            validator.validate_block(&block),
            Err(ValidationError::NoTransactions)
        );
    }

    #[test]
    #[ignore] // Too slow - requires mining
    fn test_validate_block_missing_coinbase() {
        let validator = BlockValidator::new(0x207fffff);

        // Create a non-coinbase transaction
        let tx = Transaction::new(
            vec![crate::core::TxInput::new(
                Hash256::new([1; 32]),
                0,
                vec![],
            )],
            vec![TxOutput::new(1000, vec![])],
        );

        let merkle = Block::calculate_merkle_root(&[tx.clone()]);
        let mut header = BlockHeader::new(
            1,
            Hash256::zero(),
            merkle,
            1234567890,
            0x207fffff,
            0,
        );

        // Mine the block
        let miner = Miner::new(0x207fffff);
        miner.mine(&mut header);

        let block = Block::new(header, vec![tx]);

        assert_eq!(
            validator.validate_block(&block),
            Err(ValidationError::MissingCoinbase)
        );
    }

    #[test]
    fn test_validate_mempool_transaction() {
        // Valid transaction
        let tx = Transaction::new(
            vec![crate::core::TxInput::new(
                Hash256::new([1; 32]),
                0,
                vec![1, 2, 3],
            )],
            vec![TxOutput::new(1000, vec![4, 5, 6])],
        );

        assert!(TransactionValidator::validate_for_mempool(&tx).is_ok());

        // Coinbase should fail
        let coinbase = Transaction::coinbase(
            vec![1, 2, 3],
            TxOutput::new(5000000000, vec![]),
            0,
        );

        assert_eq!(
            TransactionValidator::validate_for_mempool(&coinbase),
            Err(ValidationError::CoinbaseNotFirst)
        );
    }
}
