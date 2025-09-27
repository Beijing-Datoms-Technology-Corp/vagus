// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// @title Core Types for Vagus Protocol
/// @notice Defines fundamental data structures used across the Vagus ecosystem
library Types {
    // Core Intent structure
    struct Intent {
        uint256 executorId;
        bytes32 actionId;
        bytes params;
        bytes32 envelopeHash;
        bytes32 preStateRoot;
        uint64 notBefore;
        uint64 notAfter;
        uint32 maxDurationMs;
        uint32 maxEnergyJ;
        address planner;
        uint256 nonce;
    }

    // Capability Token metadata
    struct TokenMeta {
        uint256 executorId;
        bytes32 actionId;
        bytes32 scaledLimitsHash;
        uint64 issuedAt;
        uint64 expiresAt;
        bool revoked;
        address issuer;
    }

    // Constants
    uint256 constant MAX_DURATION_MS = 30000; // 30 seconds
    uint256 constant MAX_ENERGY_J = 1000; // 1000 Joules
}
