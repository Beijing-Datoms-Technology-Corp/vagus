#!/bin/bash
# Vagus Protocol Release Script
# Implements T-7: Versioned product releases with build fingerprints, SBOM, and LICENSE

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VERSION_FILE="VERSION"
BUILD_DIR="build"
RELEASE_DIR="releases"
SBOM_FILE="sbom.json"

# Functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Get version from VERSION file
get_version() {
    if [ ! -f "$VERSION_FILE" ]; then
        log_error "VERSION file not found"
        exit 1
    fi
    cat "$VERSION_FILE" | tr -d '\n'
}

# Generate build fingerprint
generate_fingerprint() {
    local version=$1
    local timestamp=$(date -u +"%Y%m%d%H%M%S")
    local git_commit=$(git rev-parse HEAD 2>/dev/null || echo "nogit")
    local build_fingerprint="${version}-${timestamp}-${git_commit:0:8}"

    echo "$build_fingerprint"
}

# Build EVM contracts
build_evm() {
    log_info "Building EVM contracts..."
    cd contracts

    # Clean and build
    rm -rf "$BUILD_DIR"
    mkdir -p "$BUILD_DIR"

    # Run forge build with optimization
    forge build --optimize --optimizer-runs 200 --out "$BUILD_DIR"

    # Generate contract sizes report
    forge build --sizes > "$BUILD_DIR/contract-sizes.txt"

    cd ..
    log_success "EVM contracts built"
}

# Build WASM contracts
build_wasm() {
    log_info "Building WASM contracts..."
    cd wasm-contracts

    # Clean and build
    cargo build --release

    # Create build directory
    mkdir -p "../$BUILD_DIR/wasm"

    # Copy artifacts
    cp -r target/wasm32-unknown-unknown/release/*.wasm "../$BUILD_DIR/wasm/"

    cd ..
    log_success "WASM contracts built"
}

# Generate SBOM (Software Bill of Materials)
generate_sbom() {
    local version=$1
    local fingerprint=$2

    log_info "Generating SBOM..."

    # Create SBOM JSON
    cat > "$SBOM_FILE" << EOF
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.4",
  "serialNumber": "urn:uuid:$(uuidgen)",
  "version": 1,
  "metadata": {
    "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
    "tools": [
      {
        "vendor": "Vagus Protocol",
        "name": "release.sh",
        "version": "$fingerprint"
      }
    ],
    "component": {
      "type": "library",
      "name": "vagus-protocol",
      "version": "$version",
      "description": "Vagus Protocol - Decentralized Autonomous Safety System"
    }
  },
  "components": [
    {
      "type": "library",
      "name": "solidity-contracts",
      "version": "$version",
      "description": "EVM Smart Contracts",
      "licenses": [
        {
          "license": {
            "id": "Apache-2.0"
          }
        }
      ],
      "hashes": [
        {
          "alg": "SHA-256",
          "content": "$(find contracts/src -name "*.sol" -exec sha256sum {} \; | sort | sha256sum | cut -d' ' -f1)"
        }
      ]
    },
    {
      "type": "library",
      "name": "cosmwasm-contracts",
      "version": "$version",
      "description": "CosmWasm Smart Contracts",
      "licenses": [
        {
          "license": {
            "id": "Apache-2.0"
          }
        }
      ],
      "hashes": [
        {
          "alg": "SHA-256",
          "content": "$(find wasm-contracts -name "*.rs" -exec sha256sum {} \; | sort | sha256sum | cut -d' ' -f1)"
        }
      ]
    },
    {
      "type": "library",
      "name": "python-planner",
      "version": "$version",
      "description": "Python Intent Planner",
      "licenses": [
        {
          "license": {
            "id": "Apache-2.0"
          }
        }
      ]
    },
    {
      "type": "library",
      "name": "rust-gateway",
      "version": "$version",
      "description": "Rust Gateway and Oracle",
      "licenses": [
        {
          "license": {
            "id": "Apache-2.0"
          }
        }
      ]
    }
  ]
}
EOF

    log_success "SBOM generated: $SBOM_FILE"
}

# Create release archive
create_release_archive() {
    local version=$1
    local fingerprint=$2

    log_info "Creating release archive..."

    # Create release directory
    mkdir -p "$RELEASE_DIR"

    # Create archive name
    local archive_name="vagus-protocol-${fingerprint}.tar.gz"

    # Create archive
    tar -czf "$RELEASE_DIR/$archive_name" \
        --exclude='target' \
        --exclude='node_modules' \
        --exclude='.git' \
        --exclude="$BUILD_DIR" \
        --exclude="$RELEASE_DIR" \
        --exclude="*.log" \
        --exclude="*.tmp" \
        .

    # Generate checksum
    cd "$RELEASE_DIR"
    sha256sum "$archive_name" > "${archive_name}.sha256"
    cd ..

    log_success "Release archive created: $RELEASE_DIR/$archive_name"
    log_info "SHA256 checksum: $(cat "$RELEASE_DIR/${archive_name}.sha256")"
}

# Generate release notes
generate_release_notes() {
    local version=$1
    local fingerprint=$2

    log_info "Generating release notes..."

    local release_notes="$RELEASE_DIR/RELEASE_NOTES-${fingerprint}.md"

    cat > "$release_notes" << EOF
# Vagus Protocol Release $version

**Build Fingerprint:** $fingerprint
**Release Date:** $(date -u +"%Y-%m-%d %H:%M:%S UTC")
**Git Commit:** $(git rev-parse HEAD 2>/dev/null || echo "N/A")

## Overview

Vagus Protocol is a decentralized autonomous safety system that implements cross-stack equivalence between EVM and WASM L1 blockchains.

## Components

### EVM Contracts (Solidity)
- CapabilityIssuer: Issues revocable capability tokens with rate limiting and circuit breaker
- VagalBrake: Applies dynamic scaling based on ANS state
- ANSStateManager: Manages autonomic nervous system state with hysteresis
- ReflexArc: Triggers automated revocation based on evidence analysis
- AfferentInbox: Receives and archives afferent evidence

### WASM Contracts (CosmWasm)
- capability_issuer: WASM version with governance integration
- vagus_governor: Multi-signature governance using cw3-fixed-multisig

### Off-chain Components
- Python Planner: Generates and validates intents with CBOR encoding
- Rust Gateway/Oracle: Processes and verifies data with deterministic CBOR

## Key Features

- **Cross-stack Equivalence:** Identical semantics across EVM and WASM
- **Rate Limiting:** Sliding window rate limiting per executor-action pair
- **Circuit Breaker:** Three-state circuit breaker for fault tolerance
- **Deterministic CBOR:** Canonical encoding ensuring hash consistency
- **Governance:** Multi-signature governance on both stacks
- **ANS Hysteresis:** Anti-jitter state management with dwell times

## Security Features

- Reentrancy guards and access controls
- Emergency pause functionality
- Upgrade safety with version checking
- Comprehensive error handling

## Installation

\`\`\`bash
# Extract archive
tar -xzf vagus-protocol-${fingerprint}.tar.gz

# For EVM deployment
cd contracts
npm install
npx hardhat run scripts/deploy.js

# For WASM deployment
cd wasm-contracts
cargo build --release
\`\`\`

## Verification

Verify the release integrity:

\`\`\`bash
# Check SHA256 checksum
sha256sum -c ${archive_name}.sha256

# Verify SBOM
cat sbom.json
\`\`\`

## License

Licensed under Apache License 2.0. See LICENSE file for details.

## Changelog

### Features
- Implemented complete T-1 through T-7 of Master Plan M11-M20
- Cross-stack equivalence for EVM and WASM L1
- Rate limiting and circuit breaker implementation
- Governance integration with multi-signature controls
- Deterministic CBOR encoding with dual hashing
- ANS hysteresis with anti-jitter behavior

### Bug Fixes
- Fixed ANS state manager hysteresis logic
- Resolved CBOR encoding consistency issues
- Improved error handling and validation

## Support

For issues and questions, please refer to the project documentation or create an issue in the repository.
EOF

    log_success "Release notes generated: $release_notes"
}

# Main release function
main() {
    log_info "Starting Vagus Protocol release process..."

    # Get version
    local version=$(get_version)
    log_info "Version: $version"

    # Generate build fingerprint
    local fingerprint=$(generate_fingerprint "$version")
    log_info "Build fingerprint: $fingerprint"

    # Create build directories
    mkdir -p "$BUILD_DIR"
    mkdir -p "$RELEASE_DIR"

    # Build components
    build_evm
    build_wasm

    # Generate SBOM
    generate_sbom "$version" "$fingerprint"

    # Create release archive
    create_release_archive "$version" "$fingerprint"

    # Generate release notes
    generate_release_notes "$version" "$fingerprint"

    log_success "Release $version ($fingerprint) completed successfully!"
    log_info "Release artifacts available in: $RELEASE_DIR/"
    log_info "Build artifacts available in: $BUILD_DIR/"
}

# Check if script is being run directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
