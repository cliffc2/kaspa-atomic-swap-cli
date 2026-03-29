/// Kaspa RPC client for transaction submission and chain queries - Phase 2
use serde_json::json;
use std::error::Error;

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

    /// Submit a transaction to the network
    pub async fn submit_transaction(&self, tx_hex: &str) -> Result<String, Box<dyn Error>> {
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

        if let Some(result) = body.get("result") {
            if let Some(txid) = result.get("transactionId") {
                return Ok(txid.as_str().unwrap_or("").to_string());
            }
        }

        Err("Failed to submit transaction".into())
    }

    /// Get transaction by ID
    pub async fn get_transaction(&self, txid: &str) -> Result<TransactionInfo, Box<dyn Error>> {
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
            let tx_info = serde_json::from_value(result.clone())?;
            return Ok(tx_info);
        }

        Err("Transaction not found".into())
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

    /// Phase 2: Get UTXO by txid:index
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

    /// Phase 2: Estimate fee rate (sompi per byte)
    pub async fn estimate_fee_rate(&self) -> Result<u64, Box<dyn Error>> {
        // Default: 1 sompi/byte for testnet, can be dynamic in production
        // Typical range: 0.5 - 2 sompi/byte
        Ok(1)
    }

    /// Phase 2: Check if wallet has sufficient balance
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

    #[tokio::test]
    async fn test_fee_rate_estimation() {
        let rpc = KaspaRpc::new("http://localhost:16110");
        let rate = rpc.estimate_fee_rate().await;
        assert!(rate.is_ok());
        assert_eq!(rate.unwrap(), 1);
    }
}
