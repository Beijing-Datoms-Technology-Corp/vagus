// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Events.sol";
import "./Types.sol";

/// @title Afferent Evidence Inbox
/// @notice Receives and archives afferent evidence from device gateways
contract AfferentInbox is Events {
    /// @notice Evidence record structure
    struct Evidence {
        bytes32 stateRoot;
        bytes32 metricsHash;
        uint64 timestamp;
        address attestor;
    }

    /// @notice Mapping of executor ID to latest evidence
    mapping(uint256 => Evidence) public latestEvidence;

    /// @notice Authorized attestor addresses
    mapping(address => bool) public authorizedAttestors;

    /// @notice Contract owner
    address public owner;

    /// @notice Custom error for unauthorized attestor
    error UnauthorizedAttestor();

    /// @notice Custom error for invalid signature
    error InvalidSignature();

    /// @notice Constructor
    constructor() {
        owner = msg.sender;
    }

    /// @notice Post afferent evidence
    /// @param executorId The executor identifier
    /// @param stateRoot The state root hash
    /// @param metricsHash The metrics hash
    /// @param signature The attestation signature
    function postAEP(
        uint256 executorId,
        bytes32 stateRoot,
        bytes32 metricsHash,
        bytes calldata signature
    ) external {
        // Verify attestor is authorized
        if (!authorizedAttestors[msg.sender]) {
            revert UnauthorizedAttestor();
        }

        // TODO: Verify signature over the evidence data
        // For MVP, we skip signature verification and trust authorized attestor

        // Store the evidence
        latestEvidence[executorId] = Evidence({
            stateRoot: stateRoot,
            metricsHash: metricsHash,
            timestamp: uint64(block.timestamp),
            attestor: msg.sender
        });

        // Emit event
        emit AEPPosted(executorId, stateRoot, metricsHash);
    }

    /// @notice Get latest state root for an executor
    /// @param executorId The executor identifier
    /// @return The latest state root
    function latestStateRoot(uint256 executorId) external view returns (bytes32) {
        return latestEvidence[executorId].stateRoot;
    }

    /// @notice Authorize an attestor
    /// @param attestor The attestor address to authorize
    function authorizeAttestor(address attestor) external {
        require(msg.sender == owner, "Only owner can authorize attestors");
        authorizedAttestors[attestor] = true;
    }

    /// @notice Revoke attestor authorization
    /// @param attestor The attestor address to revoke
    function revokeAttestor(address attestor) external {
        require(msg.sender == owner, "Only owner can revoke attestors");
        authorizedAttestors[attestor] = false;
    }
}
