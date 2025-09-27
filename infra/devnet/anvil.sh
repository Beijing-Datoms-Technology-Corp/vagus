#!/bin/bash

# Start Anvil devnet for Vagus development
# Requires: foundry (forge, anvil)

set -e

# Default configuration
PORT=${ANVIL_PORT:-8545}
HOST=${ANVIL_HOST:-127.0.0.1}
BLOCK_TIME=${ANVIL_BLOCK_TIME:-2}

echo "Starting Anvil devnet on $HOST:$PORT (block time: $BLOCK_TIME seconds)"

# Start anvil in background
anvil \
  --host $HOST \
  --port $PORT \
  --block-time $BLOCK_TIME \
  --accounts 10 \
  --balance 10000 \
  --gas-limit 30000000 \
  --gas-price 20000000000

echo "Anvil devnet started. RPC URL: http://$HOST:$PORT"
