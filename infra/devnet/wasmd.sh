#!/bin/bash
# wasmd.sh - Start local wasmd/Cosmos chain for Vagus testing
# Based on wasmd/cosmwasm documentation

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Configuration
CHAIN_ID="vagus-devnet"
MONIKER="vagus-devnet"
WASMD_HOME="$PROJECT_ROOT/infra/devnet/wasmd-home"
LOG_FILE="$WASMD_HOME/wasmd.log"

# Check if wasmd is installed
if ! command -v wasmd &> /dev/null; then
    echo "❌ wasmd not found. Please install wasmd:"
    echo "   go install github.com/CosmWasm/wasmd@latest"
    echo "   # or download from https://github.com/CosmWasm/wasmd/releases"
    exit 1
fi

# Create home directory if it doesn't exist
if [ ! -d "$WASMD_HOME" ]; then
    echo "📁 Creating wasmd home directory: $WASMD_HOME"
    mkdir -p "$WASMD_HOME"
fi

# Initialize chain if not already done
if [ ! -d "$WASMD_HOME/config" ]; then
    echo "🔗 Initializing wasmd chain..."
    wasmd init "$MONIKER" --chain-id "$CHAIN_ID" --home "$WASMD_HOME"

    # Configure chain parameters for development
    sed -i 's/"stake"/"ucosm"/g' "$WASMD_HOME/config/genesis.json"
    sed -i 's/"voting_period": "172800000000000"/"voting_period": "30000000000"/g' "$WASMD_HOME/config/genesis.json"

    # Add genesis account
    echo "👤 Adding genesis account..."
    wasmd keys add validator --keyring-backend test --home "$WASMD_HOME"
    wasmd add-genesis-account validator 1000000000ucosm --keyring-backend test --home "$WASMD_HOME"
fi

# Start wasmd in background
echo "🚀 Starting wasmd..."
wasmd start --home "$WASMD_HOME" > "$LOG_FILE" 2>&1 &
WASMD_PID=$!

echo "✅ wasmd started with PID: $WASMD_PID"
echo "📋 Log file: $LOG_FILE"
echo "🔗 RPC endpoint: http://localhost:26657"
echo "🌐 REST endpoint: http://localhost:1317"
echo "📊 gRPC endpoint: localhost:9090"

# Wait for chain to be ready
echo "⏳ Waiting for chain to be ready..."
sleep 5

# Check if chain is responding
if curl -s http://localhost:26657/status > /dev/null; then
    echo "✅ Chain is ready!"
    echo ""
    echo "💡 Useful commands:"
    echo "   # Check status: curl http://localhost:26657/status"
    echo "   # Get accounts: wasmd keys list --keyring-backend test --home $WASMD_HOME"
    echo "   # Stop chain: kill $WASMD_PID"
    echo ""
    echo "🔑 Default accounts:"
    echo "   validator: $(wasmd keys show validator -a --keyring-backend test --home "$WASMD_HOME")"
else
    echo "❌ Chain failed to start. Check logs: $LOG_FILE"
    exit 1
fi

# Keep script running to show logs
echo "📜 Showing logs (Ctrl+C to stop)..."
tail -f "$LOG_FILE"
