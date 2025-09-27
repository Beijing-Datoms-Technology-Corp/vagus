#!/bin/bash
# Spec Drift Check Script
# Ensures that generated code matches specifications and hand-written code is consistent

set -e

echo "🔍 Running Vagus Spec Drift Check..."

# Change to project root
cd "$(dirname "$0")/.."

# Generate code from specifications
echo "📝 Generating code from specs..."
cd planner
python -m vagus_planner.codegen
cd ..

# Check if generated files differ from committed versions
echo "🔍 Checking for drift in generated files..."

# Check EVM generated files
if ! git diff --quiet contracts/src/core/GeneratedTypes.sol; then
    echo "❌ GeneratedTypes.sol has drifted from spec. Please regenerate and commit."
    git diff contracts/src/core/GeneratedTypes.sol
    exit 1
fi

if ! git diff --quiet contracts/src/core/GeneratedEvents.sol; then
    echo "❌ GeneratedEvents.sol has drifted from spec. Please regenerate and commit."
    git diff contracts/src/core/GeneratedEvents.sol
    exit 1
fi

# Check CosmWasm generated files
if ! git diff --quiet wasm-contracts/cosmwasm/packages/vagus-spec/src/lib.rs; then
    echo "❌ vagus-spec lib.rs has drifted from spec. Please regenerate and commit."
    git diff wasm-contracts/cosmwasm/packages/vagus-spec/src/lib.rs
    exit 1
fi

echo "✅ All generated code matches specifications!"
echo "🎉 Spec drift check passed!"
