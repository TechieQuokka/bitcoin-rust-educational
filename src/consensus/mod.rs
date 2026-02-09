// Consensus and validation logic

pub mod pow;
pub mod validation;

pub use pow::{Miner, Target, MiningResult};
pub use validation::{BlockValidator, TransactionValidator, ValidationError};
