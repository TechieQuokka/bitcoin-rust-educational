// Mine the genesis block to find the correct nonce

use bit_coin::{Block, Miner};

fn main() {
    println!("Mining genesis block...\n");

    let mut genesis = Block::genesis();
    let bits = 0x20ffffff;

    let miner = Miner::new(bits);
    let result = miner.mine(&mut genesis.header);

    if result.success {
        println!("✓ Genesis block mined successfully!\n");
        println!("Nonce: {}", result.nonce);
        println!("Hash: {}", result.hash);
        println!("Attempts: {}", result.attempts);
        println!("Duration: {:?}", result.duration);
        println!("Hash rate: {:.2} H/s", result.hash_rate());

        println!("\n--- Update block.rs with this nonce ---");
        println!("nonce: {},  // genesis nonce", result.nonce);
    } else {
        println!("✗ Mining failed!");
    }
}
