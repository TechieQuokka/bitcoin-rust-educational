// Transaction builder

use crate::core::{Transaction, TxInput, TxOutput, Script};
use crate::storage::{UtxoSet, OutPoint, Utxo};
use crate::wallet::{Keystore, Address};
use secp256k1::{Secp256k1, Message};

/// Transaction builder
pub struct TransactionBuilder<'a> {
    keystore: &'a Keystore,
    utxo_set: &'a UtxoSet,
}

impl<'a> TransactionBuilder<'a> {
    /// Create a new transaction builder
    pub fn new(keystore: &'a Keystore, utxo_set: &'a UtxoSet) -> Self {
        Self { keystore, utxo_set }
    }

    /// Build a transaction to send amount to recipient
    pub fn build(
        &self,
        from: &Address,
        to: &Address,
        amount: u64,
        fee: u64,
    ) -> Result<Transaction, String> {
        // Get keypair for sender
        let keypair = self.keystore
            .get_keypair(from)
            .ok_or("Sender address not found in keystore")?;

        // Get script pubkey for sender
        let sender_script = keypair.script_pubkey();

        // Get UTXOs for sender
        let utxos = self.utxo_set.get_utxos_for_script(&sender_script)?;

        if utxos.is_empty() {
            return Err("No UTXOs available for sender".to_string());
        }

        // Select UTXOs (simple strategy: use all available)
        let (selected_utxos, total_input) = self.select_utxos(&utxos, amount + fee)?;

        // Create inputs (unsigned)
        let inputs: Vec<TxInput> = selected_utxos
            .iter()
            .map(|(outpoint, _)| TxInput::new(outpoint.txid, outpoint.vout, vec![]))
            .collect();

        // Create outputs
        let mut outputs = Vec::new();

        // Payment output
        let recipient_hash = to.to_pubkey_hash()?;
        let recipient_script = Script::p2pkh_script_pubkey(&recipient_hash);
        outputs.push(TxOutput::new(amount, recipient_script));

        // Change output (if any)
        let change = total_input.saturating_sub(amount + fee);
        if change > 0 {
            outputs.push(TxOutput::new(change, sender_script.clone()));
        }

        // Create unsigned transaction
        let mut tx = Transaction::new(inputs, outputs);

        // Sign inputs
        self.sign_transaction(&mut tx, &selected_utxos, keypair)?;

        Ok(tx)
    }

    /// Select UTXOs to cover amount
    fn select_utxos(
        &self,
        utxos: &[(OutPoint, Utxo)],
        target: u64,
    ) -> Result<(Vec<(OutPoint, Utxo)>, u64), String> {
        let mut selected = Vec::new();
        let mut total = 0u64;

        for (outpoint, utxo) in utxos {
            selected.push((outpoint.clone(), utxo.clone()));
            total += utxo.output.value;

            if total >= target {
                return Ok((selected, total));
            }
        }

        Err(format!("Insufficient funds: have {}, need {}", total, target))
    }

    /// Sign transaction inputs
    fn sign_transaction(
        &self,
        tx: &mut Transaction,
        utxos: &[(OutPoint, Utxo)],
        keypair: &crate::wallet::KeyPair,
    ) -> Result<(), String> {
        let secp = Secp256k1::new();
        let tx_hash = tx.txid();

        for (i, (_, _utxo)) in utxos.iter().enumerate() {
            // Create message from tx hash
            let message = Message::from_digest_slice(tx_hash.as_bytes())
                .map_err(|e| format!("Invalid message: {}", e))?;

            // Sign
            let signature = secp.sign_ecdsa(&message, &keypair.secret_key);
            let sig_bytes = signature.serialize_der().to_vec();

            // Create script sig
            let script_sig = Script::p2pkh_script_sig(&sig_bytes, &keypair.pubkey_bytes());

            // Update input
            tx.inputs[i].script_sig = script_sig;
        }

        Ok(())
    }

    /// Get balance for address
    pub fn get_balance(&self, address: &Address) -> Result<u64, String> {
        let keypair = self.keystore
            .get_keypair(address)
            .ok_or("Address not found in keystore")?;

        let script_pubkey = keypair.script_pubkey();
        self.utxo_set.get_balance(&script_pubkey)
    }

    /// List UTXOs for address
    pub fn list_utxos(&self, address: &Address) -> Result<Vec<(OutPoint, Utxo)>, String> {
        let keypair = self.keystore
            .get_keypair(address)
            .ok_or("Address not found in keystore")?;

        let script_pubkey = keypair.script_pubkey();
        self.utxo_set.get_utxos_for_script(&script_pubkey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Hash256;

    #[test]
    fn test_transaction_builder() {
        let mut keystore = Keystore::new();
        let utxo_set = UtxoSet::memory().unwrap();

        // Create addresses
        let addr1 = keystore.new_address();
        let addr2 = keystore.new_address();

        // Add UTXO for addr1
        let kp1 = keystore.get_keypair(&addr1).unwrap();
        let script1 = kp1.script_pubkey();

        let outpoint = OutPoint::new(Hash256::new([1; 32]), 0);
        let utxo = Utxo::new(TxOutput::new(100000, script1), 1, false);
        utxo_set.add_utxo(&outpoint, &utxo).unwrap();

        // Build transaction
        let builder = TransactionBuilder::new(&keystore, &utxo_set);

        let tx = builder.build(&addr1, &addr2, 50000, 1000).unwrap();

        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(tx.outputs.len(), 2); // Payment + change

        // Verify amounts
        assert_eq!(tx.outputs[0].value, 50000); // Payment
        assert_eq!(tx.outputs[1].value, 49000); // Change (100000 - 50000 - 1000)
    }

    #[test]
    fn test_get_balance() {
        let mut keystore = Keystore::new();
        let utxo_set = UtxoSet::memory().unwrap();

        let addr = keystore.new_address();
        let kp = keystore.get_keypair(&addr).unwrap();
        let script = kp.script_pubkey();

        // Add UTXOs
        let outpoint1 = OutPoint::new(Hash256::new([1; 32]), 0);
        let utxo1 = Utxo::new(TxOutput::new(50000, script.clone()), 1, false);
        utxo_set.add_utxo(&outpoint1, &utxo1).unwrap();

        let outpoint2 = OutPoint::new(Hash256::new([2; 32]), 0);
        let utxo2 = Utxo::new(TxOutput::new(30000, script.clone()), 2, false);
        utxo_set.add_utxo(&outpoint2, &utxo2).unwrap();

        let builder = TransactionBuilder::new(&keystore, &utxo_set);
        let balance = builder.get_balance(&addr).unwrap();

        assert_eq!(balance, 80000);
    }

    #[test]
    fn test_insufficient_funds() {
        let mut keystore = Keystore::new();
        let utxo_set = UtxoSet::memory().unwrap();

        let addr1 = keystore.new_address();
        let addr2 = keystore.new_address();

        // Add small UTXO
        let kp1 = keystore.get_keypair(&addr1).unwrap();
        let script1 = kp1.script_pubkey();

        let outpoint = OutPoint::new(Hash256::new([1; 32]), 0);
        let utxo = Utxo::new(TxOutput::new(1000, script1), 1, false);
        utxo_set.add_utxo(&outpoint, &utxo).unwrap();

        // Try to send more than available
        let builder = TransactionBuilder::new(&keystore, &utxo_set);
        let result = builder.build(&addr1, &addr2, 50000, 1000);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Insufficient funds"));
    }
}
