// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// @title Canonical Events for Vagus Protocol
/// @notice Defines all events that must be emitted by Vagus contracts
contract Events {
    /// @notice Emitted when a new capability token is issued
    event CapabilityIssued(
        uint256 indexed tokenId,
        uint256 indexed executorId,
        address indexed planner,
        bytes32 actionId,
        uint256 expiresAt
    );

    /// @notice Emitted when a capability token is revoked
    event CapabilityRevoked(
        uint256 indexed tokenId,
        uint8 reason
    );

    /// @notice Emitted when afferent evidence is posted
    event AEPPosted(
        uint256 indexed executorId,
        bytes32 stateRoot,
        bytes32 metricsHash
    );

    /// @notice Emitted when vagal tone is updated
    event VagalToneUpdated(
        uint256 tone,
        uint8 state
    );

    /// @notice Emitted when reflex arc triggers capability revocation
    event ReflexTriggered(
        uint256 indexed executorId,
        string reason,
        uint256 revokedCount,
        uint256 triggeredAt
    );
}
