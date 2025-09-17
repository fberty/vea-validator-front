#!/bin/bash

set -e

# Kill any existing anvil processes
pkill -f "anvil.*854[56]" || true

# Start anvil instances
anvil -p 8545 --chain-id 31337 > /tmp/arbitrum-devnet.log 2>&1 &
ARBITRUM_PID=$!

anvil -p 8546 --chain-id 31338 > /tmp/mainnet-devnet.log 2>&1 &
MAINNET_PID=$!

# Cleanup on exit
trap 'kill $ARBITRUM_PID $MAINNET_PID 2>/dev/null || true' INT TERM EXIT

echo "Starting devnet..."
sleep 3

# Check if devnets are running
if ! curl -s -H "Content-Type: application/json" \
      --data '{"jsonrpc":"2.0","method":"eth_blockNumber","id":1}' localhost:8545 >/dev/null; then
    echo "Failed to start Arbitrum devnet"
    exit 1
fi

if ! curl -s -H "Content-Type: application/json" \
      --data '{"jsonrpc":"2.0","method":"eth_blockNumber","id":1}' localhost:8546 >/dev/null; then
    echo "Failed to start Mainnet devnet"
    exit 1
fi

echo "Devnet running: Arbitrum on 8545 (PID $ARBITRUM_PID), Mainnet on 8546 (PID $MAINNET_PID)"

echo "Deploying contracts..."

ARB_RPC="http://localhost:8545"
ETH_RPC="http://localhost:8546"
PK="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

forge build -q

FROM="$(cast wallet address "$PK")"
NONCE="$(cast nonce "$FROM" --rpc-url "$ARB_RPC")"
INBOX_PREDICTED="$(cast compute-address "$FROM" --nonce "$NONCE" \
  | awk '/Computed Address:/ {print $3}')"

create() {
  forge create "$1" \
    --rpc-url "$2" \
    --private-key "$PK" \
    --broadcast \
    "${@:3}"
}

addr_of() {
  awk '/Deployed to:/ {print $3}'
}

SEQ_INBOX="$(
  create contracts/src/test/bridge-mocks/arbitrum/SequencerInboxMock.sol:SequencerInboxMock \
    "$ETH_RPC" \
    --constructor-args 86400 | addr_of
)"

OUTBOX_MOCK="$(
  create contracts/src/test/bridge-mocks/arbitrum/OutboxMock.sol:OutboxMock \
    "$ETH_RPC" \
    --constructor-args "$INBOX_PREDICTED" | addr_of
)"

BRIDGE="$(
  create contracts/src/test/bridge-mocks/arbitrum/BridgeMock.sol:BridgeMock \
    "$ETH_RPC" \
    --constructor-args "$OUTBOX_MOCK" "$SEQ_INBOX" | addr_of
)"

OUTBOX="$(
  create contracts/src/arbitrumToEth/VeaOutboxArbToEth.sol:VeaOutboxArbToEth \
    "$ETH_RPC" \
    --constructor-args \
      1000000000000000000 \
      3600 \
      600 \
      24 \
      "$INBOX_PREDICTED" \
      "$BRIDGE" \
      10 | addr_of
)"

INBOX="$(
  create contracts/src/arbitrumToEth/VeaInboxArbToEth.sol:VeaInboxArbToEth \
    "$ARB_RPC" \
    --constructor-args 3600 "$OUTBOX" | addr_of
)"

cat > .env <<EOF
ARBITRUM_RPC_URL=$ARB_RPC
MAINNET_RPC_URL=$ETH_RPC
VEA_INBOX_ARB_TO_ETH=$INBOX
VEA_OUTBOX_ARB_TO_ETH=$OUTBOX
SEQUENCER_INBOX_MOCK=$SEQ_INBOX
OUTBOX_MOCK=$OUTBOX_MOCK
BRIDGE_MOCK=$BRIDGE
EOF

echo "SequencerInboxMock: $SEQ_INBOX"
echo "OutboxMock: $OUTBOX_MOCK"
echo "BridgeMock: $BRIDGE"
echo "VeaOutboxArbToEth: $OUTBOX"
echo "VeaInboxArbToEth: $INBOX"

echo "✅ Full devnet ready!"
echo "🛑 Press Ctrl+C to stop"

# Keep running until interrupted
wait