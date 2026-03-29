# Kaspa Atomic Swap CLI - Agent API Reference

Complete JSON API for AI agents to perform atomic swaps on Kaspa testnet.

## Endpoints (CLI Commands)

All commands output JSON with `--json` flag. Responses include:
- `success`: boolean
- `message`: human-readable description
- `data`: operation payload
- `error`: error code (if failed)

### 1. Create HTLC Swap

```bash
kaspa-atomic-swap-cli initiate \
  --amount <sompi> \
  --to <claim_address> \
  --secret-hash <32-byte-hex> \
  --timelock-blocks <blocks> \
  [--from <refund_address>] \
  [--network testnet-uxto|mainnet] \
  --json
```

**Response:**
```json
{
  "success": true,
  "message": "HTLC covenant created: 1000000000 sompi swap...",
  "data": {
    "amount": "1000000000",
    "claim_address": "kaspa:q...",
    "refund_address": "kaspa:q...",
    "secret_hash": "1234...",
    "timelock_blocks": "288",
    "network": "testnet-uxto",
    "script_hex": "HTLC[...]"
  }
}
```

**Parameters:**
- `amount`: Sompi (1 KAS = 100,000,000 sompi)
- `to`: Counterparty's claim address
- `secret_hash`: SHA256(preimage), 64 hex chars
- `timelock_blocks`: Block count (~10 min per block)
- `from`: Your refund address (optional, defaults to `to`)
- `network`: testnet-uxto or mainnet

### 2. Claim Swap (Reveal Preimage)

```bash
kaspa-atomic-swap-cli claim \
  --utxo <txid:index> \
  --secret <32-byte-hex> \
  [--network testnet-uxto|mainnet] \
  --json
```

**Response:**
```json
{
  "success": true,
  "message": "Claiming HTLC abc123... by revealing preimage on testnet-uxto",
  "data": {
    "utxo": "abc123:0",
    "network": "testnet-uxto",
    "action": "spending_with_preimage"
  }
}
```

**Parameters:**
- `utxo`: Swap UTXO (txid:output_index)
- `secret`: 32-byte preimage hex (64 chars)
- `network`: testnet-uxto or mainnet

### 3. Refund Swap (After Timelock)

```bash
kaspa-atomic-swap-cli refund \
  --utxo <txid:index> \
  [--network testnet-uxto|mainnet] \
  --json
```

**Response:**
```json
{
  "success": true,
  "message": "Refunding HTLC abc123... after timelock on testnet-uxto",
  "data": {
    "utxo": "abc123:0",
    "network": "testnet-uxto",
    "action": "timelock_refund"
  }
}
```

### 4. Check Swap Status

```bash
kaspa-atomic-swap-cli status \
  --txid <transaction-id> \
  [--network testnet-uxto|mainnet] \
  --json
```

**Response:**
```json
{
  "success": true,
  "message": "Fetching HTLC status abc123... on testnet-uxto",
  "data": {
    "txid": "abc123...",
    "network": "testnet-uxto",
    "status": "pending"
  }
}
```

### 5. Monitor for Swaps

```bash
kaspa-atomic-swap-cli monitor \
  --wallet <address> \
  [--interval <seconds>] \
  [--network testnet-uxto|mainnet] \
  --json
```

**Response:**
```json
{
  "success": true,
  "message": "Monitoring kaspa:q... for HTLC swaps every 10 seconds on testnet-uxto",
  "data": {
    "wallet": "kaspa:q...",
    "interval_secs": "10",
    "network": "testnet-uxto"
  }
}
```

### 6. Inspect Covenant Script

```bash
kaspa-atomic-swap-cli show-script \
  --secret-hash <32-byte-hex> \
  --timelock-blocks <blocks> \
  --claim-addr <address> \
  --refund-addr <address> \
  --json
```

**Response:**
```json
{
  "success": true,
  "message": "HTLC Covenant Script (timelock: 288, secret: 1234...)",
  "data": {
    "script_hex": "HTLC[...]",
    "script_asm": "# HTLC Covenant - Kaspa Atomic Swap\n...",
    "secret_hash": "1234...",
    "timelock_blocks": "288"
  }
}
```

## Error Responses

```json
{
  "success": false,
  "message": "Invalid secret_hash: Expected 32 bytes (64 hex chars), got 16",
  "error": "execution_failed"
}
```

**Error Codes:**
- `execution_failed`: Command failed (check message for reason)

## Helper Functions

### Generate Secret & Hash

```bash
# Generate random 32-byte secret
SECRET=$(head -c 32 /dev/urandom | xxd -p)

# Compute SHA256 hash
HASH=$(echo -n "$SECRET" | sha256sum | cut -d' ' -f1)
```

### Parse JSON Response

```bash
# Check success
jq '.success' response.json

# Extract data
jq '.data' response.json

# Get script
jq '.data.script_hex' response.json
```

## Agent Implementation Example

```python
import subprocess
import json

def create_htlc(amount, to_addr, secret_hash, timelock=288):
    cmd = [
        './kaspa-atomic-swap-cli', 'initiate',
        '--amount', str(amount),
        '--to', to_addr,
        '--secret-hash', secret_hash,
        '--timelock-blocks', str(timelock),
        '--network', 'testnet-uxto',
        '--json'
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    response = json.loads(result.stdout)
    
    if response['success']:
        print(f"HTLC created: {response['data']['script_hex']}")
        return response['data']
    else:
        print(f"Error: {response['message']}")
        return None

def claim_htlc(utxo, preimage):
    cmd = [
        './kaspa-atomic-swap-cli', 'claim',
        '--utxo', utxo,
        '--secret', preimage,
        '--network', 'testnet-uxto',
        '--json'
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    response = json.loads(result.stdout)
    
    return response
```

## Async Usage

Commands are synchronous (blocking). For non-blocking usage:

```bash
# Background polling
while true; do
  kaspa-atomic-swap-cli monitor \
    --wallet kaspa:qme \
    --interval 30 \
    --json | jq '.' >> swap_monitor.log
done &
```

## Configuration

Edit `~/.kaspa-swap/config.json`:

```json
{
  "kaspa_rpc_url": "http://localhost:16110",
  "network": "testnet-uxto",
  "private_key": ""
}
```

## Testing on Testnet

1. Get faucet KAS: https://faucet.kaspanet.io/
2. Run local node:
   ```bash
   docker run -d -p 16110:16110 kaspanet/kaspad:testnet-uxto --testnet-uxto
   ```
3. Create HTLC and test swap lifecycle
4. Verify claim/refund on chain

## Performance

- **Initiate**: <100ms (CLI + JSON)
- **Claim**: <100ms
- **Refund**: <100ms
- **Status**: <100ms (without RPC)
- **Monitor**: 10s+ (configurable poll interval)

## Rate Limits

None (local CLI). RPC node may have limits if remote.

## Timeout

Default 30 seconds per command (modifiable).

## Support

Issues: Ensure `--json` flag is used for agent integration. Always validate `success: true`.
