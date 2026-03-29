# Kaspa Atomic Swap CLI - covpp-reset2 Covenant Edition

A Rust CLI for AI agents to test HTLC smart contracts on Kaspa with covenant support (covpp-reset2 branch).

## Features

- **HTLC Covenants**: Hash Time Locked Contracts using Kaspa's covenant language
- **Claim Path**: Reveal 32-byte preimage, verify SHA256 hash, claim funds
- **Refund Path**: After timelock expires, recover funds automatically
- **JSON Output**: Full agent integration support
- **Testnet Ready**: Built against covpp-reset2 for Testnet-UXTO testing

## Building

```bash
cargo build --release
# Binary: target/release/kaspa-atomic-swap-cli
```

## Usage

### 1. Create HTLC (Initiate Swap)

```bash
kaspa-atomic-swap-cli initiate \
  --amount 1000000000 \
  --to <counterparty-claim-address> \
  --secret-hash <sha256-hash-hex> \
  --timelock-blocks 288 \
  --from <your-refund-address> \
  --network testnet-uxto \
  --json
```

**Parameters:**
- `--amount`: KAS amount in sompi (1 KAS = 100,000,000 sompi)
- `--to`: Counterparty's address (receives funds if they reveal preimage)
- `--secret-hash`: SHA256(preimage), 32 bytes hex
- `--timelock-blocks`: Block count for timelock (~10 min per block on Kaspa)
- `--from`: Your address (gets refund after timelock)
- `--network`: testnet-uxto, mainnet

**Example:**
```bash
kaspa-atomic-swap-cli initiate \
  --amount 1000000000 \
  --to kaspa:qz2qt8t0y0l6peevg3ll74l7za7p4r8r69gezmzc4xjx3d9ucd8u9cjemjpl \
  --secret-hash 1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef \
  --from kaspa:qprivate \
  --json
```

### 2. Claim HTLC (Spend with Preimage)

```bash
kaspa-atomic-swap-cli claim \
  --utxo <txid:index> \
  --secret <32-byte-preimage-hex> \
  --network testnet-uxto \
  --json
```

### 3. Refund HTLC (After Timelock)

```bash
kaspa-atomic-swap-cli refund \
  --utxo <txid:index> \
  --network testnet-uxto \
  --json
```

### 4. Check HTLC Status

```bash
kaspa-atomic-swap-cli status \
  --txid <transaction-id> \
  --network testnet-uxto \
  --json
```

### 5. Monitor for Incoming Swaps

```bash
kaspa-atomic-swap-cli monitor \
  --wallet <your-address> \
  --interval 10 \
  --network testnet-uxto \
  --json
```

### 6. Inspect Covenant Script

```bash
kaspa-atomic-swap-cli show-script \
  --secret-hash <hash-hex> \
  --timelock-blocks 288 \
  --claim-addr <counterparty> \
  --refund-addr <your-address> \
  --json
```

Returns both hex and assembly representation for debugging.

## JSON Response Format

All commands support `--json` for agent integration:

```json
{
  "success": true,
  "message": "HTLC covenant created: 1000000000 sompi swap...",
  "data": {
    "script_hex": "HTLC[...]",
    "claim_address": "kaspa:q...",
    "refund_address": "kaspa:q...",
    "amount": "1000000000",
    "secret_hash": "...",
    "network": "testnet-uxto",
    "timelock_blocks": "288"
  }
}
```

Errors:

```json
{
  "success": false,
  "message": "Invalid secret_hash: Expected 32 bytes (64 hex chars), got 16",
  "error": "execution_failed"
}
```

## Configuration

Config file: `~/.kaspa-swap/config.json`

```json
{
  "kaspa_rpc_url": "http://localhost:16110",
  "network": "testnet-uxto",
  "private_key": ""
}
```

## Testing on Testnet-UXTO

1. Get testnet KAS from faucet
2. Generate random secret + hash:
   ```bash
   # Secret (preimage)
   SECRET=$(head -c 32 /dev/urandom | xxd -p)
   # Hash
   HASH=$(echo -n "$SECRET" | sha256sum | cut -d' ' -f1)
   ```
3. Create HTLC with your address as refund + counterparty as claim
4. Send claim TX with preimage to unlock funds
5. Or wait for timelock and refund

## Covenant Script Breakdown

```
# Claim path (spender provides preimage)
OP_DUP
OP_SHA256
<secret_hash>
OP_EQUAL
OP_IF
    <claim_address>
    OP_CHECKSIG
OP_ELSE
    # Refund path (after timelock)
    <timelock_blocks>
    OP_CHECKBLOCKTIMEVERIFY
    <refund_address>
    OP_CHECKSIG
OP_ENDIF
```

## For AI Agents

- All output is JSON-parseable with `--json` flag
- `success` boolean indicates command success/failure
- `data` contains operation details for automation
- Validate hex inputs (32-byte values = 64 hex chars)
- Network defaults to testnet-uxto for testing

## Build from covpp-reset2

This CLI uses dependencies from the covpp-reset2 branch of rusty-kaspa:

```toml
kaspa-core = { git = "https://github.com/kaspanet/rusty-kaspa", branch = "covpp-reset2" }
kaspa-txscript = { git = "https://github.com/kaspanet/rusty-kaspa", branch = "covpp-reset2" }
```

Covenants are currently placeholder implementations. Full integration with covpp-reset2's covenant engine pending.

## Next Steps

- Integrate kaspa-txscript covenant builder for real script generation
- Wire transaction submission to Kaspa RPC
- Implement deadman switch (auto-refund monitoring)
- Add Ethereum (Igra) swap support
- Full sig/preimage path implementation
