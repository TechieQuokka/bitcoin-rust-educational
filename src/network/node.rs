// Network node - manages peer connections

use crate::network::{Peer, PeerInfo, Message, InvMessage, InvType};
use crate::core::{Block, Transaction};
use crate::storage::Storage;
use tokio::net::TcpListener;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Network node
pub struct Node {
    /// Node address
    pub addr: SocketAddr,
    /// Connected peers
    pub peers: Arc<RwLock<Vec<PeerInfo>>>,
    /// Storage
    pub storage: Arc<RwLock<Storage>>,
}

impl Node {
    /// Create a new node
    pub fn new(addr: SocketAddr, storage: Storage) -> Self {
        Self {
            addr,
            peers: Arc::new(RwLock::new(Vec::new())),
            storage: Arc::new(RwLock::new(storage)),
        }
    }

    /// Start listening for incoming connections
    pub async fn listen(&self) -> Result<(), String> {
        let listener = TcpListener::bind(self.addr)
            .await
            .map_err(|e| format!("Failed to bind: {}", e))?;

        log::info!("Node listening on {}", self.addr);

        loop {
            let (stream, addr) = listener
                .accept()
                .await
                .map_err(|e| format!("Failed to accept connection: {}", e))?;

            log::info!("New connection from {}", addr);

            let peers = self.peers.clone();
            let storage = self.storage.clone();

            // Handle peer in separate task
            tokio::spawn(async move {
                if let Err(e) = Self::handle_peer(stream, addr, peers, storage).await {
                    log::error!("Peer {} error: {}", addr, e);
                }
            });
        }
    }

    /// Connect to a peer
    pub async fn connect_to_peer(&self, addr: SocketAddr) -> Result<(), String> {
        let mut peer = Peer::connect(addr).await?;

        log::info!("Connected to peer {}", addr);

        // Perform handshake
        let our_height = self.storage.read().await.blockchain.get_chain_height()
            .map_err(|e| format!("Failed to get chain height: {}", e))?;

        peer.handshake(our_height).await?;

        log::info!("Handshake completed with {}", addr);

        // Add to peer list
        self.peers.write().await.push(peer.info.clone());

        Ok(())
    }

    /// Handle a peer connection
    async fn handle_peer(
        stream: tokio::net::TcpStream,
        addr: SocketAddr,
        peers: Arc<RwLock<Vec<PeerInfo>>>,
        storage: Arc<RwLock<Storage>>,
    ) -> Result<(), String> {
        let mut peer = Peer::new(stream, addr);

        // Perform handshake
        let our_height = storage.read().await.blockchain.get_chain_height()
            .map_err(|e| format!("Failed to get chain height: {}", e))?;

        peer.handshake(our_height).await?;

        // Add to peer list
        peers.write().await.push(peer.info.clone());

        // Message loop
        loop {
            match peer.receive_message().await {
                Ok(message) => {
                    log::debug!("Received message from {}: {:?}", addr, message.message_type());

                    match message {
                        Message::Ping(nonce) => {
                            // Respond with pong
                            peer.send_message(&Message::Pong(nonce)).await?;
                        }
                        Message::GetBlocks { start: _, stop: _ } => {
                            // Send blocks (simplified)
                            log::debug!("GetBlocks request from {}", addr);
                        }
                        Message::Inv(inv) => {
                            // Handle inventory announcement
                            log::debug!("Received inv from {}: {} items", addr, inv.hashes.len());
                        }
                        _ => {
                            log::debug!("Unhandled message type: {:?}", message.message_type());
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to receive message from {}: {}", addr, e);
                    break;
                }
            }
        }

        // Remove from peer list
        peers.write().await.retain(|p| p.addr != addr);

        Ok(())
    }

    /// Broadcast a block to all peers
    pub async fn broadcast_block(&self, block: &Block) -> Result<(), String> {
        let inv = InvMessage::new(InvType::Block, vec![block.hash()]);
        let _message = Message::Inv(inv);

        let peers = self.peers.read().await;
        log::info!("Broadcasting block to {} peers", peers.len());

        // In a real implementation, we would send to each peer
        // For now, just log
        Ok(())
    }

    /// Broadcast a transaction to all peers
    pub async fn broadcast_transaction(&self, tx: &Transaction) -> Result<(), String> {
        let inv = InvMessage::new(InvType::Tx, vec![tx.txid()]);
        let _message = Message::Inv(inv);

        let peers = self.peers.read().await;
        log::info!("Broadcasting transaction to {} peers", peers.len());

        // In a real implementation, we would send to each peer
        // For now, just log
        Ok(())
    }

    /// Get number of connected peers
    pub async fn peer_count(&self) -> usize {
        self.peers.read().await.len()
    }

    /// Get peer information
    pub async fn get_peers(&self) -> Vec<PeerInfo> {
        self.peers.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let addr: SocketAddr = "127.0.0.1:8333".parse().unwrap();
        let storage = Storage::memory().unwrap();
        let node = Node::new(addr, storage);

        assert_eq!(node.addr, addr);
    }
}
