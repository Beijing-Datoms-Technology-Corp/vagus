// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

// Auto-generated from spec/events.yml
// DO NOT EDIT MANUALLY

event CapabilityIssued(uint256 indexed tokenId, uint256 indexed executorId, address indexed planner, bytes32 actionId, uint256 expiresAt);

event CapabilityRevoked(uint256 indexed tokenId, uint256 indexed executorId, uint8 reason, uint256 revokedAt);

event AEPPosted(uint256 indexed executorId, bytes32 stateRootSha256, bytes32 stateRootKeccak, bytes32 metricsHashSha256, bytes32 metricsHashKeccak, uint256 timestamp);

event VagalToneUpdated(uint256 indexed tone, uint8 indexed state, uint256 updatedAt);

event ReflexTriggered(uint256 indexed executorId, string reason, uint256 revokedCount, uint256 triggeredAt);
