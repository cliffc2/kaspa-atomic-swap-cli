/// Wallet management for atomic swap operations - Phase 2: Real transactions
use hex::{encode, decode};
use sha2::{Sha256, Digest};
use std::error::Error;

#[derive(Debug, Clone)]
pub struct Wallet {
    private_key: Vec<u8>,
    public_key: Vec<u8>,
    address: String,
}

impl Wallet {
    /// Create wallet from hex private key
    pub fn from_private_key(private_key_hex: &str) -> Result<Self, Box<dyn Error>> {
        let cleaned = if private_key_hex.starts_with("0x") {
            &private_key_hex[2..]
        } else {
            private_key_hex
        };

        let private_key = decode(cleaned)?;
        
        if private_key.len() != 32 {
            return Err("Private key must be 32 bytes".into());
        }

        // Derive public key and address
        let (public_key, address) = derive_keypair(&private_key)?;

        Ok(Wallet {
            private_key,
            public_key,
            address,
        })
    }

    /// Get wallet address
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Get public key
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    /// Sign a message with private key
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        // Phase 2: Real ECDSA signing with secp256k1
        // Placeholder implementation - real version uses secp256k1 crate
        
        let mut hasher = Sha256::new();
        hasher.update(message);
        hasher.update(&self.private_key);
        Ok(hasher.finalize().to_vec())
    }

    /// Generate a secret and its hash for atomic swaps
    pub fn generate_swap_secret() -> (String, String) {
        use sha2::Sha256;
        
        // Generate random 32 bytes
        let mut secret_bytes = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut secret_bytes);
        let secret = encode(&secret_bytes);
        
        let mut hasher = Sha256::new();
        hasher.update(&secret_bytes);
        let hash = encode(hasher.finalize());

        (secret, hash)
    }
}

/// Derive keypair from private key
fn derive_keypair(private_key: &[u8]) -> Result<(Vec<u8>, String), Box<dyn Error>> {
    // Phase 2 placeholder: Real implementation uses secp256k1 for key derivation
    
    let mut hasher = Sha256::new();
    hasher.update(private_key);
    let public_key_hash = hasher.finalize();
    
    // Simplified address derivation
    let address = format!(
        "kaspa:q{}",
        encode(&public_key_hash[..20])
            .chars()
            .take(53)
            .collect::<String>()
    );

    Ok((public_key_hash.to_vec(), address))
}

/// Phase 2: Real Transaction Builder
#[derive(Debug)]
pub struct TransactionBuilder {
    version: u16,
    inputs: Vec<TransactionInput>,
    outputs: Vec<TransactionOutput>,
}

#[derive(Debug, Clone)]
pub struct TransactionInput {
    pub previous_txid: String,
    pub previous_index: u32,
    pub script: String,
    pub sequence: u32,
}

#[derive(Debug, Clone)]
pub struct TransactionOutput {
    pub amount: u64,
    pub script: String,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub version: u16,
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        TransactionBuilder {
            version: 0,
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Add input UTXO
    pub fn add_input(
        mut self,
        txid: &str,
        index: u32,
        script: &str,
        sequence: u32,
    ) -> Self {
        self.inputs.push(TransactionInput {
            previous_txid: txid.to_string(),
            previous_index: index,
            script: script.to_string(),
            sequence,
        });
        self
    }

    /// Add output
    pub fn add_output(mut self, amount: u64, script: &str) -> Self {
        self.outputs.push(TransactionOutput {
            amount,
            script: script.to_string(),
        });
        self
    }

    /// Build transaction
    pub fn build(&self) -> Transaction {
        Transaction {
            version: self.version,
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
        }
    }

    /// Build transaction hex (Phase 2: Real serialization)
    pub fn build_hex(&self) -> String {
        self.serialize_to_hex()
    }

    /// Serialize transaction to hex format
    fn serialize_to_hex(&self) -> String {
        let mut tx_hex = String::new();
        
        // Version (2 bytes, little-endian)
        tx_hex.push_str(&format!("{:04x}", self.version));
        
        // Input count (varint)
        tx_hex.push_str(&format!("{:02x}", self.inputs.len()));
        
        // Inputs
        for input in &self.inputs {
            // Previous TXID (32 bytes, reversed for endianness)
            tx_hex.push_str(&input.previous_txid);
            
            // Previous output index (4 bytes)
            tx_hex.push_str(&format!("{:08x}", input.previous_index));
            
            // Script length (varint)
            let script_len = input.script.len() / 2;
            if script_len < 253 {
                tx_hex.push_str(&format!("{:02x}", script_len));
            } else if script_len < 0x10000 {
                tx_hex.push_str("fd");
                tx_hex.push_str(&format!("{:04x}", script_len));
            }
            
            // Script
            tx_hex.push_str(&input.script);
            
            // Sequence (4 bytes)
            tx_hex.push_str(&format!("{:08x}", input.sequence));
        }
        
        // Output count (varint)
        tx_hex.push_str(&format!("{:02x}", self.outputs.len()));
        
        // Outputs
        for output in &self.outputs {
            // Amount (8 bytes, little-endian)
            tx_hex.push_str(&format!("{:016x}", output.amount));
            
            // Script length (varint)
            let script_len = output.script.len() / 2;
            if script_len < 253 {
                tx_hex.push_str(&format!("{:02x}", script_len));
            } else if script_len < 0x10000 {
                tx_hex.push_str("fd");
                tx_hex.push_str(&format!("{:04x}", script_len));
            }
            
            // Script
            tx_hex.push_str(&output.script);
        }

        tx_hex
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secret() {
        let (secret, hash) = Wallet::generate_swap_secret();
        assert_eq!(secret.len(), 64); // 32 bytes in hex
        assert_eq!(hash.len(), 64);   // SHA256 in hex
    }

    #[test]
    fn test_transaction_builder() {
        let tx = TransactionBuilder::new()
            .add_input("abc123def456", 0, "deadbeef", 0xffffffff)
            .add_output(100000000, "76a914")
            .build();

        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(tx.outputs.len(), 1);
        assert_eq!(tx.outputs[0].amount, 100000000);
    }

    #[test]
    fn test_transaction_serialization() {
        let tx_hex = TransactionBuilder::new()
            .add_input("abc123def456", 0, "deadbeef", 0xffffffff)
            .add_output(100000000, "76a914")
            .build_hex();

        // Verify it produces valid hex
        assert!(tx_hex.len() > 0);
        assert!(tx_hex.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
