// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

// Auto-generated from spec/types.yml
// DO NOT EDIT MANUALLY

enum ANSState {
    SAFE,
    DANGER,
    SHUTDOWN
}

enum CapabilityRevocationReason {
    OWNER_REVOCATION,
    REFLEX_TRIGGER,
    EXPIRATION
}

struct Intent {
    uint256 executorId;
    bytes32 actionId;
    bytes params;
    bytes32 envelopeHash;
    bytes32 preStateRoot;
    uint256 notBefore;
    uint256 notAfter;
    uint256 maxDurationMs;
    uint256 maxEnergyJ;
    address planner;
    uint256 nonce;
}

struct TokenMeta {
    uint256 tokenId;
    uint256 executorId;
    bytes32 actionId;
    bytes32 scaledLimitsHash;
    uint256 issuedAt;
    uint256 expiresAt;
    bool revoked;
    uint256 revokedAt;
}

struct Guard {
    uint256 scalingFactor;
    bool allowed;
}

struct AfferentEvidencePacket {
    uint256 executorId;
    bytes32 stateRootSha256;
    bytes32 stateRootKeccak;
    bytes32 metricsHashSha256;
    bytes32 metricsHashKeccak;
    uint256 timestamp;
}

struct VagalToneIndicator {
    uint256 value;
    uint256 timestamp;
}

uint256 constant MAX_DURATION_MS = 30000;
uint256 constant MAX_ENERGY_J = 1000;
uint256 constant MIN_STATE_RESIDENCY = 60;
uint256 constant REFLEX_COOLDOWN = 30;

error StateChangeTooFrequent();
error InvalidToneValue(uint256 tone);
error IntentExpired();
error InvalidPreState();
error NonceAlreadyUsed();
error TokenNotFound(uint256 tokenId);
error TokenAlreadyRevoked(uint256 tokenId);
error UnauthorizedRevocation();
error ANSBlocked(string reason);
error ANSLimitExceeded(string field, uint256 requested, uint256 allowed);
error UnauthorizedAttestor();
error InvalidEvidenceFormat();
error Unauthorized();
error InvalidInput(string reason);