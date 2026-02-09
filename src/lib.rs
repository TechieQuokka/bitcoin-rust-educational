// Bitcoin Educational Implementation
// 2009 Bitcoin v0.1.0 재구현 (교육용)

pub mod core;
pub mod consensus;
pub mod storage;
pub mod network;
pub mod wallet;

// Re-exports for convenience
pub use core::{Block, BlockHeader, Transaction, TxInput, TxOutput, Script};
pub use consensus::{Miner, Target, BlockValidator, ValidationError};
pub use storage::{Storage, BlockchainDB, UtxoSet, Utxo, OutPoint};
pub use network::{Node, Message, Peer, PeerInfo};
