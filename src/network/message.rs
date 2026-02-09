// Network protocol messages

use crate::core::{Block, Transaction, Hash256, Serializable};
use std::io::{Read, Write};

/// Network message types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    Version,
    Verack,
    Ping,
    Pong,
    Inv,
    GetData,
    Block,
    Tx,
    GetBlocks,
}

impl MessageType {
    pub fn to_string(&self) -> &str {
        match self {
            MessageType::Version => "version",
            MessageType::Verack => "verack",
            MessageType::Ping => "ping",
            MessageType::Pong => "pong",
            MessageType::Inv => "inv",
            MessageType::GetData => "getdata",
            MessageType::Block => "block",
            MessageType::Tx => "tx",
            MessageType::GetBlocks => "getblocks",
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "version" => Some(MessageType::Version),
            "verack" => Some(MessageType::Verack),
            "ping" => Some(MessageType::Ping),
            "pong" => Some(MessageType::Pong),
            "inv" => Some(MessageType::Inv),
            "getdata" => Some(MessageType::GetData),
            "block" => Some(MessageType::Block),
            "tx" => Some(MessageType::Tx),
            "getblocks" => Some(MessageType::GetBlocks),
            _ => None,
        }
    }
}

/// Inventory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvType {
    Block,
    Tx,
}

/// Version message
#[derive(Debug, Clone)]
pub struct VersionMessage {
    pub version: u32,
    pub services: u64,
    pub timestamp: u64,
    pub addr_recv: String,
    pub addr_from: String,
    pub nonce: u64,
    pub user_agent: String,
    pub start_height: u32,
}

impl VersionMessage {
    pub fn new(addr_recv: String, addr_from: String, start_height: u32) -> Self {
        Self {
            version: 1,
            services: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            addr_recv,
            addr_from,
            nonce: rand::random(),
            user_agent: "bitcoin-edu/0.1.0".to_string(),
            start_height,
        }
    }
}

/// Inventory message
#[derive(Debug, Clone)]
pub struct InvMessage {
    pub inv_type: InvType,
    pub hashes: Vec<Hash256>,
}

impl InvMessage {
    pub fn new(inv_type: InvType, hashes: Vec<Hash256>) -> Self {
        Self { inv_type, hashes }
    }
}

/// Network message
#[derive(Debug, Clone)]
pub enum Message {
    Version(VersionMessage),
    Verack,
    Ping(u64),
    Pong(u64),
    Inv(InvMessage),
    GetData(InvMessage),
    Block(Block),
    Tx(Transaction),
    GetBlocks { start: Vec<Hash256>, stop: Hash256 },
}

impl Message {
    /// Get message type
    pub fn message_type(&self) -> MessageType {
        match self {
            Message::Version(_) => MessageType::Version,
            Message::Verack => MessageType::Verack,
            Message::Ping(_) => MessageType::Ping,
            Message::Pong(_) => MessageType::Pong,
            Message::Inv(_) => MessageType::Inv,
            Message::GetData(_) => MessageType::GetData,
            Message::Block(_) => MessageType::Block,
            Message::Tx(_) => MessageType::Tx,
            Message::GetBlocks { .. } => MessageType::GetBlocks,
        }
    }

    /// Serialize message to bytes (simplified)
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Message type (12 bytes, padded with zeros)
        let msg_type_enum = self.message_type();
        let msg_type = msg_type_enum.to_string();
        let mut type_bytes = [0u8; 12];
        let type_str_bytes = msg_type.as_bytes();
        let len = type_str_bytes.len().min(12);
        type_bytes[..len].copy_from_slice(&type_str_bytes[..len]);
        bytes.extend_from_slice(&type_bytes);

        // Payload
        let payload = self.serialize_payload();

        // Payload length
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());

        // Payload
        bytes.extend_from_slice(&payload);

        bytes
    }

    /// Serialize message payload
    fn serialize_payload(&self) -> Vec<u8> {
        match self {
            Message::Version(v) => {
                let mut bytes = Vec::new();
                bytes.extend_from_slice(&v.version.to_le_bytes());
                bytes.extend_from_slice(&v.services.to_le_bytes());
                bytes.extend_from_slice(&v.timestamp.to_le_bytes());
                bytes.extend_from_slice(&v.nonce.to_le_bytes());
                bytes.extend_from_slice(&v.start_height.to_le_bytes());
                bytes
            }
            Message::Verack => Vec::new(),
            Message::Ping(nonce) | Message::Pong(nonce) => nonce.to_le_bytes().to_vec(),
            Message::Inv(inv) | Message::GetData(inv) => {
                let mut bytes = Vec::new();
                bytes.push(match inv.inv_type {
                    InvType::Block => 1,
                    InvType::Tx => 2,
                });
                bytes.extend_from_slice(&(inv.hashes.len() as u32).to_le_bytes());
                for hash in &inv.hashes {
                    bytes.extend_from_slice(hash.as_bytes());
                }
                bytes
            }
            Message::Block(block) => Serializable::serialize(block),
            Message::Tx(tx) => Serializable::serialize(tx),
            Message::GetBlocks { start, stop } => {
                let mut bytes = Vec::new();
                bytes.extend_from_slice(&(start.len() as u32).to_le_bytes());
                for hash in start {
                    bytes.extend_from_slice(hash.as_bytes());
                }
                bytes.extend_from_slice(stop.as_bytes());
                bytes
            }
        }
    }

    /// Deserialize message from bytes (simplified)
    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        if data.len() < 16 {
            return Err("Message too short".to_string());
        }

        // Parse message type
        let type_bytes = &data[0..12];
        let msg_type_str = std::str::from_utf8(type_bytes)
            .map_err(|e| format!("Invalid message type: {}", e))?
            .trim_end_matches('\0');

        let msg_type = MessageType::from_string(msg_type_str)
            .ok_or_else(|| format!("Unknown message type: {}", msg_type_str))?;

        // Parse payload length
        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&data[12..16]);
        let payload_len = u32::from_le_bytes(len_bytes) as usize;

        if data.len() < 16 + payload_len {
            return Err("Incomplete payload".to_string());
        }

        let payload = &data[16..16 + payload_len];

        // Deserialize based on type
        match msg_type {
            MessageType::Verack => Ok(Message::Verack),
            MessageType::Ping => {
                if payload.len() < 8 {
                    return Err("Invalid ping payload".to_string());
                }
                let mut nonce_bytes = [0u8; 8];
                nonce_bytes.copy_from_slice(&payload[0..8]);
                Ok(Message::Ping(u64::from_le_bytes(nonce_bytes)))
            }
            MessageType::Pong => {
                if payload.len() < 8 {
                    return Err("Invalid pong payload".to_string());
                }
                let mut nonce_bytes = [0u8; 8];
                nonce_bytes.copy_from_slice(&payload[0..8]);
                Ok(Message::Pong(u64::from_le_bytes(nonce_bytes)))
            }
            _ => Err(format!("Deserialization not implemented for {:?}", msg_type)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_conversion() {
        assert_eq!(MessageType::Version.to_string(), "version");
        assert_eq!(MessageType::from_string("version"), Some(MessageType::Version));
    }

    #[test]
    fn test_version_message() {
        let msg = VersionMessage::new(
            "127.0.0.1:8333".to_string(),
            "127.0.0.1:8334".to_string(),
            100,
        );

        assert_eq!(msg.version, 1);
        assert_eq!(msg.start_height, 100);
    }

    #[test]
    fn test_ping_pong_serialization() {
        let nonce = 12345u64;
        let ping = Message::Ping(nonce);

        let serialized = ping.serialize();
        let deserialized = Message::deserialize(&serialized).unwrap();

        match deserialized {
            Message::Ping(n) => assert_eq!(n, nonce),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_verack_serialization() {
        let verack = Message::Verack;
        let serialized = verack.serialize();
        let deserialized = Message::deserialize(&serialized).unwrap();

        assert!(matches!(deserialized, Message::Verack));
    }
}
