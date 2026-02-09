// CLI commands

use clap::{Parser, Subcommand};
use crate::{Storage, Block};
use crate::wallet::{Keystore, TransactionBuilder};

#[derive(Parser)]
#[command(name = "bitcoin-edu")]
#[command(about = "Educational Bitcoin implementation", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new blockchain
    Init,

    /// Get blockchain info
    Info,

    /// Wallet commands
    #[command(subcommand)]
    Wallet(WalletCommands),

    /// Block commands
    #[command(subcommand)]
    Block(BlockCommands),
}

#[derive(Subcommand)]
pub enum WalletCommands {
    /// Create a new address
    NewAddress,

    /// List all addresses
    List,

    /// Get balance for an address
    Balance {
        /// Address to check (uses default if not specified)
        address: Option<String>,
    },

    /// Send coins to an address
    Send {
        /// Recipient address
        to: String,
        /// Amount in satoshis
        amount: u64,
        /// Transaction fee in satoshis
        #[arg(short, long, default_value = "1000")]
        fee: u64,
    },
}

#[derive(Subcommand)]
pub enum BlockCommands {
    /// Get block by hash or height
    Get {
        /// Block hash or height
        id: String,
    },

    /// Get blockchain height
    Height,

    /// Get best block hash
    BestBlock,
}

/// CLI handler
pub struct CliHandler {
    storage: Storage,
    keystore: Keystore,
    keystore_path: String,
}

impl CliHandler {
    /// Create a new CLI handler
    pub fn new(data_dir: &str) -> Result<Self, String> {
        let storage = Storage::new(data_dir)?;

        // Load or create keystore
        let keystore_path = format!("{}/keystore.json", data_dir);
        let keystore = if std::path::Path::new(&keystore_path).exists() {
            log::info!("Loading keystore from {}", keystore_path);
            Keystore::load(&keystore_path)?
        } else {
            log::info!("Creating new keystore");
            Keystore::new()
        };

        Ok(Self {
            storage,
            keystore,
            keystore_path,
        })
    }

    /// Save keystore to disk
    fn save_keystore(&self) -> Result<(), String> {
        self.keystore.save(&self.keystore_path)
    }

    /// Handle CLI command
    pub fn handle(&mut self, cli: Cli) -> Result<(), String> {
        match cli.command {
            Commands::Init => self.init(),
            Commands::Info => self.info(),
            Commands::Wallet(cmd) => self.handle_wallet(cmd),
            Commands::Block(cmd) => self.handle_block(cmd),
        }
    }

    /// Initialize blockchain
    fn init(&mut self) -> Result<(), String> {
        println!("Initializing blockchain...");

        // Store genesis block
        let genesis = Block::genesis();
        self.storage.blockchain.store_block(&genesis)?;
        self.storage.blockchain.store_height(0, &genesis.hash())?;
        self.storage.blockchain.store_tip(&genesis.hash())?;
        self.storage.blockchain.store_chain_height(1)?;

        // Add genesis coinbase UTXO
        let coinbase_tx = &genesis.transactions[0];
        let outpoint = crate::storage::OutPoint::new(coinbase_tx.txid(), 0);
        let utxo = crate::storage::Utxo::new(
            coinbase_tx.outputs[0].clone(),
            0,
            true,
        );
        self.storage.utxo_set.add_utxo(&outpoint, &utxo)?;

        println!("✓ Genesis block created");
        println!("  Hash: {}", genesis.hash());
        println!("  Height: 0");

        Ok(())
    }

    /// Get blockchain info
    fn info(&self) -> Result<(), String> {
        let height = self.storage.blockchain.get_chain_height()?;
        let tip = self.storage.blockchain.get_tip()?;
        let utxo_count = self.storage.utxo_set.count()?;

        println!("Blockchain Info:");
        println!("  Height: {}", height);
        if let Some(hash) = tip {
            println!("  Best block: {}", hash);
        }
        println!("  UTXO count: {}", utxo_count);

        Ok(())
    }

    /// Handle wallet commands
    fn handle_wallet(&mut self, cmd: WalletCommands) -> Result<(), String> {
        match cmd {
            WalletCommands::NewAddress => {
                let addr = self.keystore.new_address();
                println!("New address: {}", addr);
                self.save_keystore()?;
                Ok(())
            }
            WalletCommands::List => {
                let addresses = self.keystore.list_addresses();
                println!("Addresses ({}):", addresses.len());
                for addr in addresses {
                    println!("  {}", addr);
                }
                Ok(())
            }
            WalletCommands::Balance { address } => {
                let addr = if let Some(a) = address {
                    crate::wallet::Address(a)
                } else {
                    self.keystore.default_address()
                        .ok_or("No default address. Create one with 'wallet new-address'")?
                        .clone()
                };

                let builder = TransactionBuilder::new(&self.keystore, &self.storage.utxo_set);
                let balance = builder.get_balance(&addr)?;

                println!("Balance for {}:", addr);
                println!("  {} satoshis ({} BTC)", balance, balance as f64 / 100_000_000.0);

                Ok(())
            }
            WalletCommands::Send { to, amount, fee } => {
                let from = self.keystore.default_address()
                    .ok_or("No default address. Create one with 'wallet new-address'")?
                    .clone();

                let to_addr = crate::wallet::Address(to);

                let builder = TransactionBuilder::new(&self.keystore, &self.storage.utxo_set);
                let tx = builder.build(&from, &to_addr, amount, fee)?;

                println!("Transaction created:");
                println!("  TXID: {}", tx.txid());
                println!("  Inputs: {}", tx.inputs.len());
                println!("  Outputs: {}", tx.outputs.len());
                println!("  Total output: {} satoshis", tx.total_output_value());

                Ok(())
            }
        }
    }

    /// Handle block commands
    fn handle_block(&self, cmd: BlockCommands) -> Result<(), String> {
        match cmd {
            BlockCommands::Get { id } => {
                // Try parsing as height first
                if let Ok(height) = id.parse::<u32>() {
                    if let Some(block) = self.storage.blockchain.get_block_by_height(height)? {
                        self.print_block(&block);
                        return Ok(());
                    }
                }

                // Try as hash
                if let Ok(hash) = crate::core::Hash256::from_hex(&id) {
                    if let Some(block) = self.storage.blockchain.get_block(&hash)? {
                        self.print_block(&block);
                        return Ok(());
                    }
                }

                Err(format!("Block not found: {}", id))
            }
            BlockCommands::Height => {
                let height = self.storage.blockchain.get_chain_height()?;
                println!("Blockchain height: {}", height);
                Ok(())
            }
            BlockCommands::BestBlock => {
                if let Some(hash) = self.storage.blockchain.get_tip()? {
                    println!("Best block: {}", hash);
                } else {
                    println!("No blocks in chain");
                }
                Ok(())
            }
        }
    }

    /// Print block information
    fn print_block(&self, block: &Block) {
        println!("Block:");
        println!("  Hash: {}", block.hash());
        println!("  Previous: {}", block.header.prev_block_hash);
        println!("  Merkle root: {}", block.header.merkle_root);
        println!("  Timestamp: {}", block.header.timestamp);
        println!("  Nonce: {}", block.header.nonce);
        println!("  Transactions: {}", block.transactions.len());

        for (i, tx) in block.transactions.iter().enumerate() {
            println!("    [{}] {}", i, tx.txid());
        }
    }
}
