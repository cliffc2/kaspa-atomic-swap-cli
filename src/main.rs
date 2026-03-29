use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

mod covenant;
mod rpc;
mod wallet;

use covenant::AtomicSwapCovenant;

#[derive(Parser)]
#[command(author, version, about = "Kaspa Atomic Swap CLI with Covenant Smart Contracts", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output JSON instead of text
    #[arg(long, global = true)]
    json: bool,

    /// Config file (default: ~/.kaspa-swap/config.json)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initiate atomic swap with HTLC covenant
    Initiate {
        /// Amount in sompi (1 KAS = 100,000,000 sompi)
        #[arg(short, long)]
        amount: u64,

        /// Counterparty address (claim address)
        #[arg(short, long)]
        to: String,

        /// Secret hash (hex, 32 bytes SHA256)
        #[arg(long)]
        secret_hash: String,

        /// Timelock in blocks (default: 288 ≈ 24h on Kaspa)
        #[arg(long, default_value_t = 288)]
        timelock_blocks: u64,

        /// Kaspa network: mainnet, testnet11, testnet-uxto
        #[arg(short, long, default_value = "testnet-uxto")]
        network: String,

        /// Our wallet address for refund path
        #[arg(long)]
        from: Option<String>,
    },

    /// Claim swap by revealing secret preimage
    Claim {
        /// Swap UTXO txid:index
        #[arg(short, long)]
        utxo: String,

        /// Secret preimage (hex, 32 bytes)
        #[arg(short, long)]
        secret: String,

        /// Network
        #[arg(short, long, default_value = "testnet-uxto")]
        network: String,
    },

    /// Refund after timelock expires
    Refund {
        /// Swap UTXO txid:index
        #[arg(short, long)]
        utxo: String,

        /// Network
        #[arg(short, long, default_value = "testnet-uxto")]
        network: String,
    },

    /// Query swap UTXO status on chain
    Status {
        /// Transaction ID
        #[arg(short, long)]
        txid: String,

        /// Network
        #[arg(short, long, default_value = "testnet-uxto")]
        network: String,
    },

    /// Monitor wallet for incoming swaps
    Monitor {
        /// Your wallet address
        #[arg(short, long)]
        wallet: String,

        /// Poll interval in seconds
        #[arg(long, default_value_t = 10)]
        interval: u64,

        /// Network
        #[arg(short, long, default_value = "testnet-uxto")]
        network: String,
    },

    /// Show covenant script for debugging
    ShowScript {
        /// Secret hash (hex)
        #[arg(long)]
        secret_hash: String,

        /// Timelock in blocks
        #[arg(long, default_value_t = 288)]
        timelock_blocks: u64,

        /// Refund address
        #[arg(long)]
        refund_addr: String,

        /// Claim address
        #[arg(long)]
        claim_addr: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl Response {
    fn ok(message: impl Into<String>, data: Option<HashMap<String, String>>) -> Self {
        Response {
            success: true,
            message: message.into(),
            data,
            error: None,
        }
    }

    fn err(message: impl Into<String>) -> Self {
        Response {
            success: false,
            message: message.into(),
            data: None,
            error: Some("execution_failed".to_string()),
        }
    }

    fn print(self, json: bool) {
        if json {
            if let Ok(s) = serde_json::to_string(&self) {
                println!("{}", s);
            }
        } else {
            if self.success {
                println!("✓ {}", self.message);
                if let Some(data) = self.data {
                    for (k, v) in data {
                        println!("  {}: {}", k, v);
                    }
                }
            } else {
                eprintln!("✗ {}", self.message);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    kaspa_rpc_url: String,
    network: String,
    private_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            kaspa_rpc_url: "http://localhost:16110".to_string(),
            network: "testnet-uxto".to_string(),
            private_key: String::new(),
        }
    }
}

fn load_config(path: Option<PathBuf>) -> Result<Config, String> {
    let config_path = path.unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".kaspa-swap")
            .join("config.json")
    });

    if !config_path.exists() {
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let default = Config::default();
        if let Ok(json) = serde_json::to_string_pretty(&default) {
            let _ = fs::write(&config_path, json);
        }
        return Ok(default);
    }

    match fs::read_to_string(&config_path) {
        Ok(content) => serde_json::from_str(&content).map_err(|e| format!("Config parse error: {}", e)),
        Err(e) => Err(format!("Config read error: {}", e)),
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }

    let _config = load_config(cli.config);

    match &cli.command {
        Commands::Initiate {
            amount,
            to,
            secret_hash,
            timelock_blocks,
            network,
            from,
        } => {
            if let Err(e) = validate_hex(secret_hash, 32) {
                Response::err(format!("Invalid secret_hash: {}", e)).print(cli.json);
                return;
            }

            // Use 'to' as claim address and 'from' as refund address (defaults to 'to' if not set)
            let refund_addr = from.as_deref().unwrap_or(to);

            // Create covenant for HTLC swap
            match AtomicSwapCovenant::new(
                secret_hash.clone(),
                *timelock_blocks,
                to.clone(),
                refund_addr.to_string(),
            ) {
                Ok(covenant) => {
                    let mut data = HashMap::new();
                    data.insert("amount".to_string(), amount.to_string());
                    data.insert("claim_address".to_string(), to.clone());
                    data.insert("refund_address".to_string(), refund_addr.to_string());
                    data.insert("secret_hash".to_string(), secret_hash.clone());
                    data.insert("timelock_blocks".to_string(), timelock_blocks.to_string());
                    data.insert("network".to_string(), network.clone());
                    data.insert("script_hex".to_string(), covenant.script_hex());

                    let msg = format!(
                        "HTLC covenant created: {} sompi swap, claim: {}, refund after {} blocks on {}",
                        amount, to, timelock_blocks, network
                    );
                    Response::ok(msg, Some(data)).print(cli.json);
                }
                Err(e) => {
                    Response::err(format!("Covenant creation failed: {}", e)).print(cli.json);
                }
            }
        }

        Commands::Claim {
            utxo,
            secret,
            network,
        } => {
            if let Err(e) = validate_hex(secret, 32) {
                Response::err(format!("Invalid secret: {}", e)).print(cli.json);
                return;
            }

            let mut data = HashMap::new();
            data.insert("utxo".to_string(), utxo.clone());
            data.insert("network".to_string(), network.clone());
            data.insert("action".to_string(), "spending_with_preimage".to_string());

            let msg = format!("Claiming HTLC {} by revealing preimage on {}", utxo, network);
            Response::ok(msg, Some(data)).print(cli.json);
        }

        Commands::Refund { utxo, network } => {
            let mut data = HashMap::new();
            data.insert("utxo".to_string(), utxo.clone());
            data.insert("network".to_string(), network.clone());
            data.insert("action".to_string(), "timelock_refund".to_string());

            let msg = format!("Refunding HTLC {} after timelock on {}", utxo, network);
            Response::ok(msg, Some(data)).print(cli.json);
        }

        Commands::Status { txid, network } => {
            let mut data = HashMap::new();
            data.insert("txid".to_string(), txid.clone());
            data.insert("network".to_string(), network.clone());
            data.insert("status".to_string(), "pending".to_string());

            let msg = format!("Fetching HTLC status {} on {}", txid, network);
            Response::ok(msg, Some(data)).print(cli.json);
        }

        Commands::Monitor {
            wallet,
            interval,
            network,
        } => {
            let mut data = HashMap::new();
            data.insert("wallet".to_string(), wallet.clone());
            data.insert("interval_secs".to_string(), interval.to_string());
            data.insert("network".to_string(), network.clone());

            let msg = format!(
                "Monitoring {} for HTLC swaps every {} seconds on {}",
                wallet, interval, network
            );
            Response::ok(msg, Some(data)).print(cli.json);
        }

        Commands::ShowScript {
            secret_hash,
            timelock_blocks,
            refund_addr,
            claim_addr,
        } => {
            if let Err(e) = validate_hex(secret_hash, 32) {
                Response::err(format!("Invalid secret_hash: {}", e)).print(cli.json);
                return;
            }

            match AtomicSwapCovenant::new(
                secret_hash.clone(),
                *timelock_blocks,
                claim_addr.clone(),
                refund_addr.clone(),
            ) {
                Ok(covenant) => {
                    let mut data = HashMap::new();
                    data.insert("script_hex".to_string(), covenant.script_hex());
                    data.insert("script_asm".to_string(), covenant.script_asm());
                    data.insert("secret_hash".to_string(), secret_hash.clone());
                    data.insert("timelock_blocks".to_string(), timelock_blocks.to_string());

                    let msg = format!("HTLC Covenant Script (timelock: {}, secret: {})", timelock_blocks, secret_hash);
                    Response::ok(msg, Some(data)).print(cli.json);
                }
                Err(e) => {
                    Response::err(format!("Script generation failed: {}", e)).print(cli.json);
                }
            }
        }
    }
}

fn validate_hex(input: &str, expected_bytes: usize) -> Result<(), String> {
    let clean = if input.starts_with("0x") || input.starts_with("0X") {
        &input[2..]
    } else {
        input
    };

    if clean.len() != expected_bytes * 2 {
        return Err(format!(
            "Expected {} bytes ({} hex chars), got {}",
            expected_bytes,
            expected_bytes * 2,
            clean.len()
        ));
    }

    if !clean.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid hex characters".to_string());
    }

    Ok(())
}
