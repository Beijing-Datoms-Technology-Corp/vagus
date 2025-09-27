#!/bin/bash
# up.sh - One-click startup script for Vagus dual-chain development environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🚀 Starting Vagus Dual-Chain Development Environment"
echo "=================================================="

# Check if docker and docker-compose are available
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found. Please install Docker."
    exit 1
fi

if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    echo "❌ Docker Compose not found. Please install Docker Compose."
    exit 1
fi

cd "$SCRIPT_DIR"

# Build and start all services
echo "🏗️  Building and starting services..."
if command -v docker-compose &> /dev/null; then
    docker-compose up --build -d
else
    docker compose up --build -d
fi

echo "⏳ Waiting for services to be ready..."

# Wait for anvil
echo "⏳ Waiting for EVM chain (Anvil)..."
timeout=60
counter=0
while ! curl -s http://localhost:8545 -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' > /dev/null; do
    if [ $counter -ge $timeout ]; then
        echo "❌ EVM chain failed to start within $timeout seconds"
        exit 1
    fi
    counter=$((counter + 1))
    sleep 1
done
echo "✅ EVM chain ready at http://localhost:8545"

# Wait for wasmd
echo "⏳ Waiting for Cosmos chain (wasmd)..."
counter=0
while ! curl -s http://localhost:26657/status > /dev/null; do
    if [ $counter -ge $timeout ]; then
        echo "❌ Cosmos chain failed to start within $timeout seconds"
        exit 1
    fi
    counter=$((counter + 1))
    sleep 1
done
echo "✅ Cosmos chain ready at http://localhost:26657"

# Wait for tone oracle
echo "⏳ Waiting for Tone Oracle..."
counter=0
while ! curl -s http://localhost:3000/health > /dev/null; do
    if [ $counter -ge 30 ]; then
        echo "❌ Tone Oracle failed to start within 30 seconds"
        exit 1
    fi
    counter=$((counter + 1))
    sleep 1
done
echo "✅ Tone Oracle ready at http://localhost:3000"

echo ""
echo "🎉 Vagus Dual-Chain Environment Started Successfully!"
echo "=================================================="
echo ""
echo "📋 Service Endpoints:"
echo "   🌐 EVM Chain (Anvil):     http://localhost:8545"
echo "   🌌 Cosmos Chain (wasmd):  http://localhost:26657 (RPC)"
echo "                             http://localhost:1317 (REST)"
echo "   🎛️  Tone Oracle:          http://localhost:3000"
echo ""
echo "🤖 Gateway Instances:"
echo "   🔗 EVM Gateway:           Running (executor ID: 1)"
echo "   🌀 Cosmos Gateway:        Running (executor ID: 2)"
echo ""
echo "🔄 Relayer Services:"
echo "   ↔️  EVM → Cosmos:         Running"
echo "   ↔️  Cosmos → EVM:         Running"
echo ""
echo "💡 Useful Commands:"
echo "   # View logs: docker-compose logs -f [service-name]"
echo "   # Stop all: docker-compose down"
echo "   # Test VTI: curl -X POST http://localhost:3000/vti -H 'Content-Type: application/json' -d '{\"executor_id\": 1, \"human_distance_mm\": 1500.0, \"temperature_celsius\": 25.0, \"energy_consumption_j\": 50.0, \"jerk_m_s3\": 2.0}'"
echo ""
echo "📝 Next Steps:"
echo "   1. Deploy Vagus contracts to both chains"
echo "   2. Update gateway contract addresses"
echo "   3. Test cross-chain event synchronization"
echo ""
echo "Happy hacking! 🚀"
