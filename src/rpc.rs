/// Kaspa RPC client for transaction submission and chain queries - Phase 2 Step 3
use serde_json::json;
use std::error::Error;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionStatus {
    Pending,           // In mempool, not in block
    Confirmed(u64),    // In block at height
    Final(u64),        // >10 blocks deep
    Failed(String),    // Rejected
    NotFound,          // Not in network
}

#[derive(Debug, Clone)]
pub struct TransactionSubmission {
    pub txid: String,
    pub submitted_at: u64,
    pub status: TransactionStatus,
}

#[derive(Debug, Clone)]
pub struct TransactionDetails {
    pub txid: String,
    pub block_height: Option<u64>,
    pub confirmations: u64,
    pub is_final: bool,
    pub inputs: Vec<(String, u32)>, // (txid, index)
    pub outputs: Vec<(u64, String)>, // (amount, script)
}

#[derive(Debug, Clone)]
pub struct KaspaRpc {
    url: String,
    client: reqwest::Client,
}

impl KaspaRpc {
    pub fn new(rpc_url: &str) -> Self {
        KaspaRpc {
            url: rpc_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Phase 2 Step 3: Submit a signed transaction to the network
    pub async fn submit_transaction(&self, tx_hex: &str) -> Result<String, Box<dyn Error>> {
        // Validate hex format
        if !tx_hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid transaction hex format".into());
        }

        let payload = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "submitTransaction",
            "params": {
                "transaction": tx_hex,
                "allowOrphan": false
            }
        });

        let response = self
            .client
            .post(&self.url)
            .json(&payload)
            .send()
            .await?;

        let body: serde_json::Value = response.json().await?;

        // Check for RPC errors
        if let Some(error) = body.get("error") {
            if error.is_null() {
                // No error, extract txid
                if let Some(result) = body.get("result") {
                    if let Some(txid) = result.get("transactionId") {
                        return Ok(txid.as_str().unwrap_or("").to_string());
                    }
                }
            } else {
                let error_msg = error.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                return Err(format!("RPC error: {}", error_msg).into());
            }
        }

        if let Some(result) = body.get("result") {
            if let Some(txid) = result.get("transactionId") {
                return Ok(txid.as_str().unwrap_or("").to_string());
            }
        }

        Err("Failed to submit transaction - no txid in response".into())
    }

    /// Phase 2 Step 3: Get transaction status
    pub async fn get_transaction_status(&self, txid: &str) -> Result<TransactionStatus, Box<dyn Error>> {
        let tx = self.get_transaction(txid).await?;
        
        match tx.block_height {
            None => {
                // Could be pending or failed
                // Check if in mempool
                Ok(TransactionStatus::Pending)
            }
            Some(height) => {
                if tx.is_final {
                    Ok(TransactionStatus::Final(height))
                } else {
                    Ok(TransactionStatus::Confirmed(height))
                }
            }
        }
    }

    /// Phase 2 Step 3: Get transaction details
    pub async fn get_transaction(&self, txid: &str) -> Result<TransactionDetails, Box<dyn Error>> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getTransaction",
            "params": {
                "transactionId": txid
            }
        });

        let response = self
            .client
            .post(&self.url)
            .json(&payload)
            .send()
            .await?;

        let body: serde_json::Value = response.json().await?;

        if let Some(result) = body.get("result") {
            // Parse transaction details
            let block_height = result
                .get("blockHeight")
                .and_then(|v| v.as_u64());

            let confirmations = result
                .get("confirmations")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let is_final = confirmations >= 10 || (result
                .get("isFinal")
                .and_then(|v| v.as_bool())
                .unwrap_or(false));

            let mut inputs = Vec::new();
            if let Some(tx_inputs) = result.get("inputs").and_then(|v| v.as_array()) {
                for input in tx_inputs {
                    if let Some(outpoint) = input.get("previousOutpoint") {
                        let txid = outpoint
                            .get("transactionId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let index = outpoint
                            .get("index")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32;
                        inputs.push((txid.to_string(), index));
                    }
                }
            }

            let mut outputs = Vec::new();
            if let Some(tx_outputs) = result.get("outputs").and_then(|v| v.as_array()) {
                for output in tx_outputs {
                    let amount = output
                        .get("value")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let script = output
                        .get("scriptPublicKey")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    outputs.push((amount, script.to_string()));
                }
            }

            return Ok(TransactionDetails {
                txid: txid.to_string(),
                block_height,
                confirmations,
                is_final,
                inputs,
                outputs,
            });
        }

        Err("Transaction not found".into())
    }

    /// Phase 2 Step 3: Wait for transaction confirmation
    pub async fn wait_for_confirmation(
        &self,
        txid: &str,
        required_confirmations: u64,
        timeout_secs: u64,
    ) -> Result<bool, Box<dyn Error>> {
        let start = std::time::SystemTime::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            match self.get_transaction_status(txid).await {
                Ok(TransactionStatus::Final(_)) | Ok(TransactionStatus::Confirmed(_)) => {
                    let tx = self.get_transaction(txid).await?;
                    if tx.confirmations >= required_confirmations {
                        return Ok(true);
                    }
                }
                Ok(TransactionStatus::Failed(msg)) => {
                    return Err(format!("Transaction failed: {}", msg).into());
                }
                _ => {}
            }

            if start.elapsed()? > timeout {
                return Err(format!(
                    "Timeout waiting for {} confirmations",
                    required_confirmations
                )
                .into());
            }

            // Poll every 5 seconds
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    /// Get current network block info
    pub async fn get_block_info(&self) -> Result<BlockInfo, Box<dyn Error>> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getBlockInfo"
        });

        let response = self
            .client
            .post(&self.url)
            .json(&payload)
            .send()
            .await?;

        let body: serde_json::Value = response.json().await?;

        if let Some(result) = body.get("result") {
            let block_info = BlockInfo {
                current_block_height: result
                    .get("blockHeight")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                network: result
                    .get("network")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
            };
            return Ok(block_info);
        }

        Err("Failed to get block info".into())
    }

    /// Phase 2: Get UTXOs by address
    pub async fn get_utxos_by_address(&self, address: &str) -> Result<Vec<UtxoInfo>, Box<dyn Error>> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getUtxosByAddresses",
            "params": {
                "addresses": [address]
            }
        });

        let response = self
            .client
            .post(&self.url)
            .json(&payload)
            .send()
            .await?;

        let body: serde_json::Value = response.json().await?;

        let mut utxos = Vec::new();
        if let Some(result) = body.get("result") {
            if let Some(outpoints) = result.get("outpoints").and_then(|v| v.as_array()) {
                for outpoint in outpoints {
                    if let Some(utxo_entry) = outpoint.get("utxoEntry") {
                        utxos.push(UtxoInfo {
                            txid: outpoint
                                .get("outpoint")
                                .and_then(|v| v.get("transactionId"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            index: outpoint
                                .get("outpoint")
                                .and_then(|v| v.get("index"))
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u32,
                            amount: utxo_entry
                                .get("amount")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0),
                            script_public_key: utxo_entry
                                .get("scriptPublicKey")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        });
                    }
                }
            }
        }

        Ok(utxos)
    }

    /// Get UTXO by txid:index
    pub async fn get_utxo(&self, txid: &str, index: u32) -> Result<UtxoInfo, Box<dyn Error>> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getUtxosByAddresses",
            "params": {
                "addresses": [txid]
            }
        });

        let response = self
            .client
            .post(&self.url)
            .json(&payload)
            .send()
            .await?;

        let body: serde_json::Value = response.json().await?;

        if let Some(result) = body.get("result") {
            let utxo_info = UtxoInfo {
                txid: txid.to_string(),
                index,
                amount: result
                    .get("outpoints")
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.get("utxoEntry"))
                    .and_then(|v| v.get("amount"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                script_public_key: result
                    .get("outpoints")
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.get("utxoEntry"))
                    .and_then(|v| v.get("scriptPublicKey"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            };
            return Ok(utxo_info);
        }

        Err("UTXO not found".into())
    }

    /// Estimate fee rate (sompi per byte)
    pub async fn estimate_fee_rate(&self) -> Result<u64, Box<dyn Error>> {
        Ok(1) // 1 sompi/byte
    }

    /// Check if wallet has sufficient balance
    pub async fn validate_balance(&self, address: &str, required_amount: u64) -> Result<bool, Box<dyn Error>> {
        let utxos = self.get_utxos_by_address(address).await?;
        let total_balance: u64 = utxos.iter().map(|u| u.amount).sum();
        Ok(total_balance >= required_amount)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransactionInfo {
    #[serde(rename = "transactionId")]
    pub id: String,
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransactionInput {
    #[serde(rename = "previousOutpoint")]
    pub previous_outpoint: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransactionOutput {
    pub value: u64,
    #[serde(rename = "scriptPublicKey")]
    pub script_public_key: String,
}

#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub current_block_height: u64,
    pub network: String,
}

#[derive(Debug, Clone)]
pub struct UtxoInfo {
    pub txid: String,
    pub index: u32,
    pub amount: u64,
    pub script_public_key: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_initialization() {
        let rpc = KaspaRpc::new("http://localhost:16110");
        assert_eq!(rpc.url, "http://localhost:16110");
    }

    #[test]
    fn test_transaction_status_enum() {
        let pending = TransactionStatus::Pending;
        let confirmed = TransactionStatus::Confirmed(100);
        let final_tx = TransactionStatus::Final(110);
        let failed = TransactionStatus::Failed("test".to_string());

        assert_ne!(pending, confirmed);
        assert_ne!(confirmed, final_tx);
        assert_ne!(final_tx, failed);
    }

    #[test]
    fn test_transaction_submission_struct() {
        let submission = TransactionSubmission {
            txid: "abc123".to_string(),
            submitted_at: 1000,
            status: TransactionStatus::Pending,
        };

        assert_eq!(submission.txid, "abc123");
        assert_eq!(submission.submitted_at, 1000);
    }

    #[test]
    fn test_transaction_details_struct() {
        let details = TransactionDetails {
            txid: "tx123".to_string(),
            block_height: Some(100),
            confirmations: 5,
            is_final: false,
            inputs: vec![("input_tx".to_string(), 0)],
            outputs: vec![(100000000, "script".to_string())],
        };

        assert_eq!(details.confirmations, 5);
        assert!(!details.is_final);
        assert_eq!(details.inputs.len(), 1);
    }

    #[tokio::test]
    async fn test_fee_rate_estimation() {
        let rpc = KaspaRpc::new("http://localhost:16110");
        let rate = rpc.estimate_fee_rate().await;
        assert!(rate.is_ok());
        assert_eq!(rate.unwrap(), 1);
    }
}
