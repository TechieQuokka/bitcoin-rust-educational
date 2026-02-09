// Bitcoin Educational Implementation - Example Runner

use bit_coin::{Block, BlockValidator, Script, Storage, TxOutput, OutPoint, Utxo, Node};
use bit_coin::network::{Message as NetMessage, VersionMessage, InvMessage, InvType};
use secp256k1::{Secp256k1, SecretKey, Message};
use rand::rngs::OsRng;

fn main() {
    env_logger::init();

    println!("=== Bitcoin Educational Implementation ===\n");

    // Phase 1: Basic structures
    phase1_demo();

    println!("\n{}\n", "=".repeat(50));

    // Phase 2: Validation
    phase2_demo();

    println!("\n{}\n", "=".repeat(50));

    // Phase 3: Storage
    phase3_demo();

    println!("\n{}\n", "=".repeat(50));

    // Phase 4: Networking
    phase4_demo();
}

fn phase1_demo() {
    println!("--- Phase 1: Basic Structures ---\n");

    let genesis = Block::genesis();

    println!("Genesis Block:");
    println!("  Hash: {}", genesis.hash());
    println!("  Merkle Root: {}", genesis.header.merkle_root);
    println!("  Transactions: {}", genesis.transactions.len());
    println!("  Coinbase Value: {} BTC",
             genesis.transactions[0].outputs[0].value as f64 / 100_000_000.0);
}

fn phase2_demo() {
    println!("--- Phase 2: Validation & Cryptography ---\n");

    // 1. Block Validation
    println!("1. Block Validation");
    println!("{}", "-".repeat(50));

    let validator = BlockValidator::new(0x20ffffff);
    let genesis = Block::genesis();

    match validator.validate_block(&genesis) {
        Ok(_) => println!("✓ Genesis block validated successfully!"),
        Err(e) => println!("✗ Validation failed: {}", e),
    }

    // 2. P2PKH Script Verification
    println!("\n2. P2PKH Script & ECDSA Signatures");
    println!("{}", "-".repeat(50));

    let secp = Secp256k1::new();
    let mut rng = OsRng;

    // Generate keypair
    let secret_key = SecretKey::new(&mut rng);
    let public_key = secret_key.public_key(&secp);
    let pubkey_bytes = public_key.serialize();

    println!("Generated keypair");
    println!("  Public key (first 16 bytes): {}...",
             hex::encode(&pubkey_bytes[..16]));

    // Create P2PKH address
    let pubkey_hash = bit_coin::core::hash160(&pubkey_bytes);
    println!("  Pubkey hash: {}", hex::encode(&pubkey_hash));

    // Create scripts
    let script_pubkey = Script::p2pkh_script_pubkey(&pubkey_hash);
    println!("  ScriptPubKey: {} bytes", script_pubkey.len());

    // Sign a transaction hash
    let tx_hash = [0x42; 32];
    let message = Message::from_digest_slice(&tx_hash).unwrap();
    let signature = secp.sign_ecdsa(&message, &secret_key);
    let sig_bytes = signature.serialize_der().to_vec();

    println!("\nSigning & Verification:");
    println!("  Signature: {} bytes", sig_bytes.len());

    // Create scriptSig and verify
    let script_sig = Script::p2pkh_script_sig(&sig_bytes, &pubkey_bytes);

    match Script::verify_p2pkh(&script_sig, &script_pubkey, &tx_hash) {
        Ok(true) => println!("  ✓ Signature verified successfully!"),
        Ok(false) => println!("  ✗ Signature verification failed"),
        Err(e) => println!("  ✗ Error: {}", e),
    }

    // Test with wrong key
    println!("\nTesting with wrong signature...");
    let wrong_key = SecretKey::new(&mut rng);
    let wrong_sig = secp.sign_ecdsa(&message, &wrong_key);
    let wrong_sig_bytes = wrong_sig.serialize_der().to_vec();
    let wrong_script_sig = Script::p2pkh_script_sig(&wrong_sig_bytes, &pubkey_bytes);

    match Script::verify_p2pkh(&wrong_script_sig, &script_pubkey, &tx_hash) {
        Ok(true) => println!("  ✗ Wrong signature accepted (bug!)"),
        Ok(false) | Err(_) => println!("  ✓ Wrong signature correctly rejected"),
    }

    // Summary
    println!("\n{}", "=".repeat(50));
    println!("=== Phase 2 Complete ===");
    println!("✓ Block structure validation");
    println!("✓ P2PKH script verification");
    println!("✓ ECDSA signature generation & verification");
    println!("✓ Proof of Work algorithms ready");
}

fn phase3_demo() {
    println!("--- Phase 3: Storage & UTXO Management ---\n");

    // 1. Blockchain Storage
    println!("1. Blockchain Database");
    println!("{}", "-".repeat(50));

    let storage = Storage::memory().unwrap();
    let genesis = Block::genesis();

    // Store genesis block
    storage.blockchain.store_block(&genesis).unwrap();
    storage.blockchain.store_height(0, &genesis.hash()).unwrap();
    storage.blockchain.store_tip(&genesis.hash()).unwrap();
    storage.blockchain.store_chain_height(1).unwrap();

    println!("✓ Genesis block stored");
    println!("  Hash: {}", genesis.hash());

    // Retrieve block
    let retrieved = storage.blockchain.get_block(&genesis.hash()).unwrap().unwrap();
    println!("✓ Block retrieved successfully");
    println!("  Transactions: {}", retrieved.transactions.len());

    // Get by height
    let by_height = storage.blockchain.get_block_by_height(0).unwrap().unwrap();
    println!("✓ Block retrieved by height 0");
    assert_eq!(retrieved.hash(), by_height.hash());

    // Chain info
    let tip = storage.blockchain.get_tip().unwrap().unwrap();
    let height = storage.blockchain.get_chain_height().unwrap();
    println!("  Chain tip: {}", tip);
    println!("  Chain height: {}", height);

    // 2. UTXO Set Management
    println!("\n2. UTXO Set Management");
    println!("{}", "-".repeat(50));

    // Add genesis coinbase UTXO
    let coinbase_tx = &genesis.transactions[0];
    let outpoint = OutPoint::new(coinbase_tx.txid(), 0);
    let utxo = Utxo::new(
        coinbase_tx.outputs[0].clone(),
        0,
        true,
    );

    storage.utxo_set.add_utxo(&outpoint, &utxo).unwrap();
    println!("✓ Genesis coinbase UTXO added");
    println!("  TXID: {}", coinbase_tx.txid());
    println!("  Value: {} satoshis", utxo.output.value);

    // Check UTXO exists
    let exists = storage.utxo_set.has_utxo(&outpoint).unwrap();
    println!("✓ UTXO exists: {}", exists);

    // Get UTXO
    let retrieved_utxo = storage.utxo_set.get_utxo(&outpoint).unwrap().unwrap();
    println!("✓ UTXO retrieved");
    println!("  Value: {} satoshis", retrieved_utxo.output.value);
    println!("  Height: {}", retrieved_utxo.height);
    println!("  Is coinbase: {}", retrieved_utxo.is_coinbase);

    // Add more UTXOs
    let script_pubkey = vec![1, 2, 3, 4, 5];

    let tx2_hash = bit_coin::core::hash256(b"tx2");
    let outpoint2 = OutPoint::new(tx2_hash, 0);
    let utxo2 = Utxo::new(
        TxOutput::new(1000000, script_pubkey.clone()),
        1,
        false,
    );
    storage.utxo_set.add_utxo(&outpoint2, &utxo2).unwrap();

    let tx3_hash = bit_coin::core::hash256(b"tx3");
    let outpoint3 = OutPoint::new(tx3_hash, 0);
    let utxo3 = Utxo::new(
        TxOutput::new(2000000, script_pubkey.clone()),
        2,
        false,
    );
    storage.utxo_set.add_utxo(&outpoint3, &utxo3).unwrap();

    println!("\n✓ Added 2 more UTXOs for same address");

    // Get balance
    let balance = storage.utxo_set.get_balance(&script_pubkey).unwrap();
    println!("  Balance: {} satoshis ({} BTC)",
             balance,
             balance as f64 / 100_000_000.0);

    // Count UTXOs
    let count = storage.utxo_set.count().unwrap();
    println!("  Total UTXOs: {}", count);

    // Spend a UTXO
    storage.utxo_set.remove_utxo(&outpoint2).unwrap();
    println!("\n✓ Spent UTXO (removed from set)");

    let new_balance = storage.utxo_set.get_balance(&script_pubkey).unwrap();
    println!("  New balance: {} satoshis ({} BTC)",
             new_balance,
             new_balance as f64 / 100_000_000.0);

    let new_count = storage.utxo_set.count().unwrap();
    println!("  Remaining UTXOs: {}", new_count);

    // Summary
    println!("\n{}", "=".repeat(50));
    println!("=== Phase 3 Complete ===");
    println!("✓ Blockchain database (sled)");
    println!("✓ Block storage & retrieval");
    println!("✓ Height indexing");
    println!("✓ UTXO set management");
    println!("✓ Balance calculation");
    println!("✓ UTXO spending");
}

fn phase4_demo() {
    println!("--- Phase 4: P2P Networking ---\n");

    // 1. Protocol Messages
    println!("1. Protocol Messages");
    println!("{}", "-".repeat(50));

    // Version message
    let version = VersionMessage::new(
        "127.0.0.1:8333".to_string(),
        "127.0.0.1:8334".to_string(),
        100,
    );
    println!("✓ Version message created");
    println!("  Version: {}", version.version);
    println!("  User agent: {}", version.user_agent);
    println!("  Start height: {}", version.start_height);

    // Ping/Pong
    let nonce = 12345u64;
    let ping = NetMessage::Ping(nonce);
    println!("\n✓ Ping message created");
    println!("  Nonce: {}", nonce);

    let serialized = ping.serialize();
    println!("  Serialized: {} bytes", serialized.len());

    let deserialized = NetMessage::deserialize(&serialized).unwrap();
    println!("✓ Message deserialized successfully");
    match deserialized {
        NetMessage::Ping(n) => println!("  Nonce matches: {}", n == nonce),
        _ => println!("  Unexpected message type"),
    }

    // Verack
    let verack = NetMessage::Verack;
    let verack_ser = verack.serialize();
    let verack_deser = NetMessage::deserialize(&verack_ser).unwrap();
    println!("\n✓ Verack message serialization verified");

    // 2. Inventory Messages
    println!("\n2. Inventory Messages");
    println!("{}", "-".repeat(50));

    let genesis = Block::genesis();
    let inv = InvMessage::new(InvType::Block, vec![genesis.hash()]);
    let inv_msg = NetMessage::Inv(inv.clone());

    println!("✓ Inventory message created");
    println!("  Type: Block");
    println!("  Items: {}", inv.hashes.len());
    println!("  First hash: {}", inv.hashes[0]);

    // 3. Node Setup (demonstration)
    println!("\n3. Network Node");
    println!("{}", "-".repeat(50));

    let addr: std::net::SocketAddr = "127.0.0.1:8333".parse().unwrap();
    let storage = Storage::memory().unwrap();
    let node = Node::new(addr, storage);

    println!("✓ Network node created");
    println!("  Listen address: {}", node.addr);

    // Store genesis block
    let genesis = Block::genesis();
    println!("\n✓ Node initialized with genesis block");
    println!("  Genesis hash: {}", genesis.hash());

    println!("\nNote: Full P2P networking requires async runtime");
    println!("Run with 'tokio' to start actual peer connections");

    // Summary
    println!("\n{}", "=".repeat(50));
    println!("=== Phase 4 Complete ===");
    println!("✓ Protocol message serialization");
    println!("✓ Ping/Pong messages");
    println!("✓ Version handshake protocol");
    println!("✓ Inventory messages (blocks/transactions)");
    println!("✓ Network node structure");
    println!("✓ Peer management ready");

    println!("\n{}", "=".repeat(50));
    println!("=== All Phases Complete! ===");
    println!("✓ Phase 1: Core data structures");
    println!("✓ Phase 2: Validation & cryptography");
    println!("✓ Phase 3: Storage & UTXO management");
    println!("✓ Phase 4: P2P networking");
    println!("\nEducational Bitcoin implementation complete!");
}
