// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "../src/core/Types.sol";

/// @title Vagal Brake Interface
/// @notice Interface for applying dynamic scaling to intents based on ANS state
interface IVagalBrake {
    /// @notice Issue capability with vagal brake applied
    /// @param intent The intent to process
    /// @return tokenId The issued capability token ID
    function issueWithBrake(Types.Intent calldata intent) external returns (uint256 tokenId);

    /// @notice Preview scaled limits without issuing
    /// @param intent The intent to preview
    /// @return scaledLimitsHash Hash of the scaled limits
    /// @return allowed Whether the intent would be allowed
    function previewBrake(Types.Intent calldata intent) external view returns (bytes32 scaledLimitsHash, bool allowed);
}
