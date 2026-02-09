// Peer connection management

use crate::network::Message;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;

/// Peer information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub addr: SocketAddr,
    pub version: u32,
    pub services: u64,
    pub start_height: u32,
    pub user_agent: String,
}

impl PeerInfo {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            version: 0,
            services: 0,
            start_height: 0,
            user_agent: String::new(),
        }
    }
}

/// Peer connection
pub struct Peer {
    pub info: PeerInfo,
    stream: TcpStream,
}

impl Peer {
    /// Create a new peer from a TCP stream
    pub fn new(stream: TcpStream, addr: SocketAddr) -> Self {
        Self {
            info: PeerInfo::new(addr),
            stream,
        }
    }

    /// Connect to a peer
    pub async fn connect(addr: SocketAddr) -> Result<Self, String> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| format!("Failed to connect: {}", e))?;

        Ok(Self::new(stream, addr))
    }

    /// Send a message to the peer
    pub async fn send_message(&mut self, message: &Message) -> Result<(), String> {
        let data = message.serialize();

        self.stream
            .write_all(&data)
            .await
            .map_err(|e| format!("Failed to send message: {}", e))?;

        self.stream
            .flush()
            .await
            .map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(())
    }

    /// Receive a message from the peer
    pub async fn receive_message(&mut self) -> Result<Message, String> {
        // Read message header (16 bytes: 12 for type + 4 for length)
        let mut header = [0u8; 16];
        self.stream
            .read_exact(&mut header)
            .await
            .map_err(|e| format!("Failed to read header: {}", e))?;

        // Parse payload length
        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&header[12..16]);
        let payload_len = u32::from_le_bytes(len_bytes) as usize;

        // Read payload
        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            self.stream
                .read_exact(&mut payload)
                .await
                .map_err(|e| format!("Failed to read payload: {}", e))?;
        }

        // Reconstruct full message
        let mut full_message = Vec::new();
        full_message.extend_from_slice(&header);
        full_message.extend_from_slice(&payload);

        // Deserialize
        Message::deserialize(&full_message)
    }

    /// Perform handshake with peer
    pub async fn handshake(&mut self, our_height: u32) -> Result<(), String> {
        // Send version message
        let version_msg = Message::Version(crate::network::VersionMessage::new(
            self.info.addr.to_string(),
            "0.0.0.0:0".to_string(),
            our_height,
        ));

        self.send_message(&version_msg).await?;

        // Receive version message
        let their_version = self.receive_message().await?;
        if let Message::Version(v) = their_version {
            self.info.version = v.version;
            self.info.services = v.services;
            self.info.start_height = v.start_height;
            self.info.user_agent = v.user_agent;
        } else {
            return Err("Expected version message".to_string());
        }

        // Send verack
        self.send_message(&Message::Verack).await?;

        // Receive verack
        let verack = self.receive_message().await?;
        if !matches!(verack, Message::Verack) {
            return Err("Expected verack message".to_string());
        }

        Ok(())
    }

    /// Get peer address
    pub fn addr(&self) -> SocketAddr {
        self.info.addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_info() {
        let addr: SocketAddr = "127.0.0.1:8333".parse().unwrap();
        let info = PeerInfo::new(addr);

        assert_eq!(info.addr, addr);
        assert_eq!(info.version, 0);
    }
}
