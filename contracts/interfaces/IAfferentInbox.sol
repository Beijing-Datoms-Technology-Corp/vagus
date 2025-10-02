// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// @title Afferent Inbox Interface
/// @notice Interface for receiving and archiving afferent evidence from device gateways
interface IAfferentInbox {
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
    ) external;

    /// @notice Get latest state root for an executor
    /// @param executorId The executor identifier
    /// @return The latest state root
    function latestStateRoot(uint256 executorId) external view returns (bytes32);

    /// @notice Authorize an attestor
    /// @param attestor The attestor address to authorize
    function authorizeAttestor(address attestor) external;

    /// @notice Revoke attestor authorization
    /// @param attestor The attestor address to revoke
    function revokeAttestor(address attestor) external;

    /// @notice Check if attestor is authorized
    /// @param attestor The attestor address to check
    /// @return True if the attestor is authorized
    function authorizedAttestors(address attestor) external view returns (bool);
}
