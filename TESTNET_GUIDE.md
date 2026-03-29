# Kaspa Testnet UXTO Testing Guide

## Setup

### 1. Get Testnet KAS

Get funds from faucet: https://faucet.kaspanet.io/

### 2. Run Local Kaspa Node (Testnet-UXTO)

```bash
# Download kaspad or use Docker
docker run -d --name kaspad-testnet \
  -p 16110:16110 \
  kaspanet/kaspad:testnet-uxto \
  --testnet-uxto
```

### 3. Generate Swap Secret & Hash

```bash
# Generate random 32-byte secret
SECRET=$(head -c 32 /dev/urandom | xxd -p)
echo "Secret: $SECRET"

# Compute SHA256 hash
HASH=$(echo -n "$SECRET" | sha256sum | cut -d' ' -f1)
echo "Hash: $HASH"

# Save for later
echo "SECRET=$SECRET" > /tmp/swap.env
echo "HASH=$HASH" >> /tmp/swap.env
```

### 4. Create HTLC Covenant

```bash
source /tmp/swap.env

# Your address (for refund)
YOUR_ADDR="kaspa:q..."

# Counterparty address (receives funds if they reveal secret)
PEER_ADDR="kaspa:q..."

# Create HTLC
kaspa-atomic-swap-cli initiate \
  --amount 100000000 \
  --to "$PEER_ADDR" \
  --secret-hash "$HASH" \
  --timelock-blocks 100 \
  --from "$YOUR_ADDR" \
  --network testnet-uxto \
  --json > /tmp/htlc.json

# Extract covenant script
SCRIPT=$(jq -r '.data.script_hex' /tmp/htlc.json)
echo "Covenant Script: $SCRIPT"
```

### 5. Test Scenarios

#### Scenario A: Successful Claim (Preimage Reveal)

```bash
# Peer claims by revealing secret
kaspa-atomic-swap-cli claim \
  --utxo "<txid>:0" \
  --secret "$SECRET" \
  --network testnet-uxto \
  --json
```

#### Scenario B: Timeout Refund

```bash
# After timelock expires
kaspa-atomic-swap-cli refund \
  --utxo "<txid>:0" \
  --network testnet-uxto \
  --json
```

#### Scenario C: Monitor for Swaps

```bash
# Agent monitors for incoming HTLC opportunities
kaspa-atomic-swap-cli monitor \
  --wallet "$YOUR_ADDR" \
  --interval 10 \
  --network testnet-uxto \
  --json
```

## Testing Checklist

- [ ] Faucet sends testnet KAS to wallet
- [ ] `kaspa-atomic-swap-cli initiate` creates HTLC with proper script
- [ ] Counterparty can claim with correct preimage
- [ ] Timeout/refund works after blocks pass
- [ ] JSON output is parseable by agents
- [ ] Script assembly is valid Kaspa opcodes

## Example Full Swap

```bash
# 1. Alice generates secret
ALICE_SECRET=$(head -c 32 /dev/urandom | xxd -p)
ALICE_HASH=$(echo -n "$ALICE_SECRET" | sha256sum | cut -d' ' -f1)

# 2. Alice creates HTLC for 1 KAS -> Bob after 100 blocks
kaspa-atomic-swap-cli initiate \
  --amount 100000000 \
  --to kaspa:qbob \
  --secret-hash "$ALICE_HASH" \
  --timelock-blocks 100 \
  --from kaspa:qalice \
  --network testnet-uxto \
  --json

# 3. Bob waits, sees HTLC
# 4. Bob claims by sending secret
kaspa-atomic-swap-cli claim \
  --utxo "abc123:0" \
  --secret "$ALICE_SECRET" \
  --network testnet-uxto \
  --json

# 5. If Bob doesn't claim in 100 blocks, Alice refunds
kaspa-atomic-swap-cli refund \
  --utxo "abc123:0" \
  --network testnet-uxto \
  --json
```

## Debugging

### Inspect Covenant Script

```bash
kaspa-atomic-swap-cli show-script \
  --secret-hash "$HASH" \
  --timelock-blocks 100 \
  --claim-addr kaspa:qbob \
  --refund-addr kaspa:qalice \
  --json
```

### RPC Node Queries

```bash
# Check transaction
curl -s -X POST http://localhost:16110 \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "getTransaction",
    "params": {"transactionId": "..."}
  }' | jq

# Get UTXOs
curl -s -X POST http://localhost:16110 \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "getUtxosByAddresses",
    "params": {"addresses": ["kaspa:q..."]}
  }' | jq
```

## Notes for AI Agents

- Always use `--json` flag for automation
- Validate response `success: true` before proceeding
- Extract `script_hex` for transaction building
- Store `txid` from transaction submission for later status checks
- Implement timelock monitoring (deadman switch) for auto-refund
- Handle network timeouts gracefully (30s default)

## Real Implementation TODO

- [ ] Integrate kaspa-txscript covenant builder
- [ ] Wire actual transaction submission via RPC
- [ ] Implement deadman switch daemon
- [ ] Add private key signing
- [ ] Support multi-sig scenarios
- [ ] Ethereum (Igra) atomic swap support
