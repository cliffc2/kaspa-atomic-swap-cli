/// HTLC Covenant for Kaspa Atomic Swaps
/// Implements: Hash Time Locked Contract using covpp-reset2 covenants
///
/// Script logic:
/// - Claim path: Reveal 32-byte preimage, verify SHA256(preimage) == secret_hash, spend to claim_addr
/// - Refund path: Check current block > timelock, spend to refund_addr

use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct AtomicSwapCovenant {
    secret_hash: String,
    timelock_blocks: u64,
    claim_address: String,
    refund_address: String,
}

impl AtomicSwapCovenant {
    pub fn new(
        secret_hash: String,
        timelock_blocks: u64,
        claim_address: String,
        refund_address: String,
    ) -> Result<Self, String> {
        // Validate inputs
        if secret_hash.is_empty() {
            return Err("secret_hash cannot be empty".to_string());
        }
        if timelock_blocks == 0 {
            return Err("timelock_blocks must be > 0".to_string());
        }
        if claim_address.is_empty() {
            return Err("claim_address cannot be empty".to_string());
        }
        if refund_address.is_empty() {
            return Err("refund_address cannot be empty".to_string());
        }

        Ok(AtomicSwapCovenant {
            secret_hash,
            timelock_blocks,
            claim_address,
            refund_address,
        })
    }

    /// Return the script as hex (for submitting to chain)
    pub fn script_hex(&self) -> String {
        // Placeholder: Real implementation would use kaspa-txscript to build
        // For now, return a descriptive format for testing
        let hash_preview = if self.secret_hash.len() > 16 { 
            &self.secret_hash[..16] 
        } else { 
            &self.secret_hash 
        };
        let claim_preview = if self.claim_address.len() > 16 { 
            &self.claim_address[..16] 
        } else { 
            &self.claim_address 
        };
        let refund_preview = if self.refund_address.len() > 16 { 
            &self.refund_address[..16] 
        } else { 
            &self.refund_address 
        };
        
        format!(
            "HTLC[hash={},timelock={},claim={},refund={}]",
            hash_preview,
            self.timelock_blocks,
            claim_preview,
            refund_preview
        )
    }

    /// Return assembly representation (for debugging)
    pub fn script_asm(&self) -> String {
        format!(
            r#"
# HTLC Covenant - Kaspa Atomic Swap
# Claim path: reveal preimage, SHA256 verify
# Refund path: timelock check

# Stack: [preimage] or []
OP_DUP
OP_SHA256
{}  # Push secret_hash
OP_EQUAL
OP_IF
    # Claim path - preimage revealed correctly
    {}  # Push claim_address
    OP_CHECKSIG
OP_ELSE
    # Refund path - check timelock
    {}  # Push current_block requirement
    OP_CHECKBLOCKTIMEVERIFY
    {}  # Push refund_address
    OP_CHECKSIG
OP_ENDIF
"#,
            self.secret_hash, self.claim_address, self.timelock_blocks, self.refund_address
        )
    }

    /// Information about this covenant for agents
    pub fn info(&self) -> CovenantInfo {
        CovenantInfo {
            script_type: "HTLC".to_string(),
            secret_hash: self.secret_hash.clone(),
            timelock_blocks: self.timelock_blocks,
            claim_address: self.claim_address.clone(),
            refund_address: self.refund_address.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CovenantInfo {
    pub script_type: String,
    pub secret_hash: String,
    pub timelock_blocks: u64,
    pub claim_address: String,
    pub refund_address: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_covenant_creation() {
        let covenant = AtomicSwapCovenant::new(
            "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            288,
            "kaspa:qz2qt8t0y0l6peevg3ll74l7za7p4r8r69gezmzc4xjx3d9ucd8u9cjemjpl".to_string(),
            "kaspa:qz2qt8t0y0l6peevg3ll74l7za7p4r8r69gezmzc4xjx3d9ucd8u9cjemjpl".to_string(),
        );

        assert!(covenant.is_ok());
    }

    #[test]
    fn test_invalid_covenant() {
        let covenant = AtomicSwapCovenant::new(
            String::new(),
            0,
            String::new(),
            String::new(),
        );

        assert!(covenant.is_err());
    }
}
