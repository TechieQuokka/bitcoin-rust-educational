// Bitcoin Script implementation (simplified for P2PKH)

use crate::core::hash160;
use secp256k1::{Secp256k1, Message, PublicKey, ecdsa::Signature};

/// Opcodes for P2PKH script
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    /// Duplicate the top stack item
    OpDup = 0x76,
    /// Hash the top stack item with HASH160
    OpHash160 = 0xa9,
    /// Push 20 bytes (pubkey hash size)
    OpPushBytes20 = 0x14,
    /// Verify that the top two items are equal
    OpEqualVerify = 0x88,
    /// Check signature
    OpCheckSig = 0xac,
}

impl OpCode {
    /// Convert byte to opcode
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x76 => Some(OpCode::OpDup),
            0xa9 => Some(OpCode::OpHash160),
            0x14 => Some(OpCode::OpPushBytes20),
            0x88 => Some(OpCode::OpEqualVerify),
            0xac => Some(OpCode::OpCheckSig),
            _ => None,
        }
    }
}

/// Script builder for P2PKH
pub struct Script;

impl Script {
    /// Create a P2PKH scriptPubKey
    /// OP_DUP OP_HASH160 <pubKeyHash> OP_EQUALVERIFY OP_CHECKSIG
    pub fn p2pkh_script_pubkey(pubkey_hash: &[u8; 20]) -> Vec<u8> {
        let mut script = Vec::new();
        script.push(OpCode::OpDup as u8);
        script.push(OpCode::OpHash160 as u8);
        script.push(OpCode::OpPushBytes20 as u8);
        script.extend_from_slice(pubkey_hash);
        script.push(OpCode::OpEqualVerify as u8);
        script.push(OpCode::OpCheckSig as u8);
        script
    }

    /// Create a P2PKH scriptSig
    /// <signature> <pubkey>
    pub fn p2pkh_script_sig(signature: &[u8], pubkey: &[u8]) -> Vec<u8> {
        let mut script = Vec::new();

        // Push signature
        script.push(signature.len() as u8);
        script.extend_from_slice(signature);

        // Push pubkey
        script.push(pubkey.len() as u8);
        script.extend_from_slice(pubkey);

        script
    }

    /// Verify a P2PKH script
    /// scriptSig: <sig> <pubkey>
    /// scriptPubKey: OP_DUP OP_HASH160 <pubKeyHash> OP_EQUALVERIFY OP_CHECKSIG
    pub fn verify_p2pkh(
        script_sig: &[u8],
        script_pubkey: &[u8],
        tx_hash: &[u8; 32],
    ) -> Result<bool, String> {
        // Parse scriptSig
        let (signature, pubkey) = Self::parse_script_sig(script_sig)?;

        // Parse scriptPubKey
        let pubkey_hash = Self::parse_script_pubkey(script_pubkey)?;

        // Step 1: Hash the public key and verify it matches
        let computed_hash = hash160(&pubkey);
        if computed_hash != pubkey_hash {
            return Ok(false);
        }

        // Step 2: Verify the signature
        Self::verify_signature(&signature, &pubkey, tx_hash)
    }

    /// Parse scriptSig: <sig> <pubkey>
    fn parse_script_sig(script_sig: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
        if script_sig.is_empty() {
            return Err("Empty scriptSig".to_string());
        }

        let mut pos = 0;

        // Read signature
        let sig_len = script_sig[pos] as usize;
        pos += 1;

        if pos + sig_len > script_sig.len() {
            return Err("Invalid signature length".to_string());
        }

        let signature = script_sig[pos..pos + sig_len].to_vec();
        pos += sig_len;

        // Read pubkey
        if pos >= script_sig.len() {
            return Err("Missing pubkey".to_string());
        }

        let pubkey_len = script_sig[pos] as usize;
        pos += 1;

        if pos + pubkey_len > script_sig.len() {
            return Err("Invalid pubkey length".to_string());
        }

        let pubkey = script_sig[pos..pos + pubkey_len].to_vec();

        Ok((signature, pubkey))
    }

    /// Parse scriptPubKey: OP_DUP OP_HASH160 <pubKeyHash> OP_EQUALVERIFY OP_CHECKSIG
    fn parse_script_pubkey(script_pubkey: &[u8]) -> Result<[u8; 20], String> {
        if script_pubkey.len() != 25 {
            return Err(format!("Invalid scriptPubKey length: {}", script_pubkey.len()));
        }

        // Verify opcodes
        if script_pubkey[0] != OpCode::OpDup as u8 {
            return Err("Expected OP_DUP".to_string());
        }
        if script_pubkey[1] != OpCode::OpHash160 as u8 {
            return Err("Expected OP_HASH160".to_string());
        }
        if script_pubkey[2] != OpCode::OpPushBytes20 as u8 {
            return Err("Expected OP_PUSHBYTES20".to_string());
        }
        if script_pubkey[23] != OpCode::OpEqualVerify as u8 {
            return Err("Expected OP_EQUALVERIFY".to_string());
        }
        if script_pubkey[24] != OpCode::OpCheckSig as u8 {
            return Err("Expected OP_CHECKSIG".to_string());
        }

        // Extract pubkey hash
        let mut pubkey_hash = [0u8; 20];
        pubkey_hash.copy_from_slice(&script_pubkey[3..23]);

        Ok(pubkey_hash)
    }

    /// Verify ECDSA signature
    fn verify_signature(
        signature: &[u8],
        pubkey: &[u8],
        message: &[u8; 32],
    ) -> Result<bool, String> {
        let secp = Secp256k1::verification_only();

        // Parse public key
        let pubkey = PublicKey::from_slice(pubkey)
            .map_err(|e| format!("Invalid public key: {}", e))?;

        // Parse signature (DER format)
        let signature = Signature::from_der(signature)
            .map_err(|e| format!("Invalid signature: {}", e))?;

        // Create message
        let message = Message::from_digest_slice(message)
            .map_err(|e| format!("Invalid message: {}", e))?;

        // Verify
        Ok(secp.verify_ecdsa(&message, &signature, &pubkey).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::{Secp256k1, SecretKey};
    use secp256k1::rand::rngs::OsRng;

    #[test]
    fn test_p2pkh_script_creation() {
        let pubkey_hash = [0x12; 20];
        let script = Script::p2pkh_script_pubkey(&pubkey_hash);

        assert_eq!(script.len(), 25);
        assert_eq!(script[0], OpCode::OpDup as u8);
        assert_eq!(script[1], OpCode::OpHash160 as u8);
        assert_eq!(script[2], OpCode::OpPushBytes20 as u8);
        assert_eq!(&script[3..23], &pubkey_hash);
        assert_eq!(script[23], OpCode::OpEqualVerify as u8);
        assert_eq!(script[24], OpCode::OpCheckSig as u8);
    }

    #[test]
    fn test_script_sig_creation() {
        let signature = vec![1, 2, 3, 4];
        let pubkey = vec![5, 6, 7, 8];

        let script_sig = Script::p2pkh_script_sig(&signature, &pubkey);

        assert_eq!(script_sig[0], 4); // sig length
        assert_eq!(&script_sig[1..5], &signature[..]);
        assert_eq!(script_sig[5], 4); // pubkey length
        assert_eq!(&script_sig[6..10], &pubkey[..]);
    }

    #[test]
    fn test_parse_script_pubkey() {
        let pubkey_hash = [0x12; 20];
        let script = Script::p2pkh_script_pubkey(&pubkey_hash);

        let parsed = Script::parse_script_pubkey(&script).unwrap();
        assert_eq!(parsed, pubkey_hash);
    }

    #[test]
    fn test_full_p2pkh_verification() {
        let secp = Secp256k1::new();
        let mut rng = OsRng;

        // Generate keypair
        let secret_key = SecretKey::new(&mut rng);
        let public_key = secret_key.public_key(&secp);
        let pubkey_bytes = public_key.serialize();

        // Create pubkey hash
        let pubkey_hash = hash160(&pubkey_bytes);

        // Create scriptPubKey
        let script_pubkey = Script::p2pkh_script_pubkey(&pubkey_hash);

        // Create message to sign (transaction hash)
        let tx_hash = [0x42; 32];
        let message = Message::from_digest_slice(&tx_hash).unwrap();

        // Sign
        let signature = secp.sign_ecdsa(&message, &secret_key);
        let sig_bytes = signature.serialize_der().to_vec();

        // Create scriptSig
        let script_sig = Script::p2pkh_script_sig(&sig_bytes, &pubkey_bytes);

        // Verify
        let valid = Script::verify_p2pkh(&script_sig, &script_pubkey, &tx_hash).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_p2pkh_verification_wrong_key() {
        let secp = Secp256k1::new();
        let mut rng = OsRng;

        // Generate two different keypairs
        let secret_key1 = SecretKey::new(&mut rng);
        let public_key1 = secret_key1.public_key(&secp);
        let pubkey_bytes1 = public_key1.serialize();

        let secret_key2 = SecretKey::new(&mut rng);
        let public_key2 = secret_key2.public_key(&secp);
        let pubkey_bytes2 = public_key2.serialize();

        // Create scriptPubKey for key1
        let pubkey_hash1 = hash160(&pubkey_bytes1);
        let script_pubkey = Script::p2pkh_script_pubkey(&pubkey_hash1);

        // Sign with key2 (wrong key)
        let tx_hash = [0x42; 32];
        let message = Message::from_digest_slice(&tx_hash).unwrap();
        let signature = secp.sign_ecdsa(&message, &secret_key2);
        let sig_bytes = signature.serialize_der().to_vec();

        // Create scriptSig with key2's signature and pubkey
        let script_sig = Script::p2pkh_script_sig(&sig_bytes, &pubkey_bytes2);

        // Verification should fail (pubkey hash mismatch)
        let valid = Script::verify_p2pkh(&script_sig, &script_pubkey, &tx_hash).unwrap();
        assert!(!valid);
    }
}
