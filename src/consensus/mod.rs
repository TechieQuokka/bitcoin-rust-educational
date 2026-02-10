// Consensus and validation logic

pub mod pow;
pub mod validation;
pub mod gpu_pow;

pub use pow::{Miner, Target, MiningResult};
pub use validation::{BlockValidator, TransactionValidator, ValidationError};
pub use gpu_pow::GpuMiner;
