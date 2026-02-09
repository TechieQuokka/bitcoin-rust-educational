// Key management

use crate::core::{hash160, Script};
use secp256k1::{Secp256k1, SecretKey, PublicKey};
use rand::rngs::OsRng;
use std::collections::HashMap;

/// Bitcoin address
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Address(pub String);

impl Address {
    /// Create address from public key hash
    pub fn from_pubkey_hash(hash: &[u8; 20]) -> Self {
        // Simple hex encoding (not Base58Check for simplicity)
        Self(hex::encode(hash))
    }

    /// Get address string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get pubkey hash from address
    pub fn to_pubkey_hash(&self) -> Result<[u8; 20], String> {
        let bytes = hex::decode(&self.0)
            .map_err(|e| format!("Invalid address: {}", e))?;

        if bytes.len() != 20 {
            return Err(format!("Invalid address length: {}", bytes.len()));
        }

        let mut hash = [0u8; 20];
        hash.copy_from_slice(&bytes);
        Ok(hash)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Key pair
#[derive(Clone)]
pub struct KeyPair {
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
    pub address: Address,
}

impl KeyPair {
    /// Generate a new key pair
    pub fn generate() -> Self {
        let secp = Secp256k1::new();
        let mut rng = OsRng;

        let secret_key = SecretKey::new(&mut rng);
        let public_key = secret_key.public_key(&secp);

        let pubkey_bytes = public_key.serialize();
        let pubkey_hash = hash160(&pubkey_bytes);
        let address = Address::from_pubkey_hash(&pubkey_hash);

        Self {
            secret_key,
            public_key,
            address,
        }
    }

    /// Get public key bytes
    pub fn pubkey_bytes(&self) -> Vec<u8> {
        self.public_key.serialize().to_vec()
    }

    /// Get pubkey hash
    pub fn pubkey_hash(&self) -> [u8; 20] {
        hash160(&self.pubkey_bytes())
    }

    /// Get script pubkey (P2PKH)
    pub fn script_pubkey(&self) -> Vec<u8> {
        Script::p2pkh_script_pubkey(&self.pubkey_hash())
    }
}

/// Keystore - manages multiple key pairs
pub struct Keystore {
    keys: HashMap<Address, KeyPair>,
    default_address: Option<Address>,
}

impl Keystore {
    /// Create a new keystore
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            default_address: None,
        }
    }

    /// Generate a new address
    pub fn new_address(&mut self) -> Address {
        let keypair = KeyPair::generate();
        let address = keypair.address.clone();

        // Set as default if first address
        if self.default_address.is_none() {
            self.default_address = Some(address.clone());
        }

        self.keys.insert(address.clone(), keypair);
        address
    }

    /// Get key pair for address
    pub fn get_keypair(&self, address: &Address) -> Option<&KeyPair> {
        self.keys.get(address)
    }

    /// Get all addresses
    pub fn list_addresses(&self) -> Vec<Address> {
        self.keys.keys().cloned().collect()
    }

    /// Get default address
    pub fn default_address(&self) -> Option<&Address> {
        self.default_address.as_ref()
    }

    /// Set default address
    pub fn set_default(&mut self, address: Address) -> Result<(), String> {
        if !self.keys.contains_key(&address) {
            return Err("Address not found in keystore".to_string());
        }
        self.default_address = Some(address);
        Ok(())
    }

    /// Get script pubkey for address
    pub fn get_script_pubkey(&self, address: &Address) -> Option<Vec<u8>> {
        self.keys.get(address).map(|kp| kp.script_pubkey())
    }

    /// Count addresses
    pub fn count(&self) -> usize {
        self.keys.len()
    }
}

impl Default for Keystore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp = KeyPair::generate();

        assert_eq!(kp.pubkey_bytes().len(), 33); // Compressed pubkey
        assert_eq!(kp.pubkey_hash().len(), 20);
    }

    #[test]
    fn test_address_conversion() {
        let hash = [0x12; 20];
        let addr = Address::from_pubkey_hash(&hash);

        let decoded = addr.to_pubkey_hash().unwrap();
        assert_eq!(hash, decoded);
    }

    #[test]
    fn test_keystore() {
        let mut ks = Keystore::new();

        assert_eq!(ks.count(), 0);
        assert!(ks.default_address().is_none());

        let addr1 = ks.new_address();
        assert_eq!(ks.count(), 1);
        assert_eq!(ks.default_address(), Some(&addr1));

        let addr2 = ks.new_address();
        assert_eq!(ks.count(), 2);

        assert!(ks.get_keypair(&addr1).is_some());
        assert!(ks.get_keypair(&addr2).is_some());

        let addresses = ks.list_addresses();
        assert_eq!(addresses.len(), 2);
    }

    #[test]
    fn test_script_pubkey() {
        let kp = KeyPair::generate();
        let script = kp.script_pubkey();

        assert_eq!(script.len(), 25); // P2PKH script length
        assert_eq!(script[0], 0x76); // OP_DUP
    }
}
