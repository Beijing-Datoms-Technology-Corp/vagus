// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "../src/core/Types.sol";

/// @title Capability Issuer Interface
/// @notice Interface for issuing and managing revocable capability tokens
interface ICapabilityIssuer {
    /// @notice Issue a capability token for an intent
    /// @param intent The intent to issue a capability for
    /// @param scaledLimitsHash Hash of the scaled limits from VagalBrake
    /// @return tokenId The issued token ID
    function issueCapability(
        Types.Intent calldata intent,
        bytes32 scaledLimitsHash
    ) external returns (uint256 tokenId);

    /// @notice Revoke a capability token
    /// @param tokenId The token ID to revoke
    /// @param reason The revocation reason code
    function revoke(uint256 tokenId, uint8 reason) external;

    /// @notice Check if a token is valid (not expired, not revoked)
    /// @param tokenId The token ID to check
    /// @return True if the token is valid
    function isValid(uint256 tokenId) external view returns (bool);

    /// @notice Get active tokens for an executor
    /// @param executorId The executor ID
    /// @return Array of active token IDs
    function activeTokensOf(uint256 executorId) external view returns (uint256[] memory);

    /// @notice Get token metadata
    /// @param tokenId The token ID
    /// @return The token metadata
    function getTokenMeta(uint256 tokenId) external view returns (Types.TokenMeta memory);
}
