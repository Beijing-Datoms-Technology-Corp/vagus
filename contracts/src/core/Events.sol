// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// @title Canonical Events for Vagus Protocol
/// @notice Defines all events that must be emitted by Vagus contracts
contract Events {
    /// @notice Emitted when a new capability token is issued
    event CapabilityIssued(
        uint256 indexed tokenId,
        uint256 indexed executorId,
        bytes32 indexed actionId,
        bytes32 paramsHash,
        uint64 notAfter
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

    /// @notice Emitted when reflex arc triggers bulk revocation
    event ReflexTriggered(
        uint256 indexed executorId,
        bytes32 reason,
        uint256[] revoked
    );
}
