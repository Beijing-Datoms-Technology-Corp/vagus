// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "../src/core/Types.sol";

/// @title ANS State Manager Interface
/// @notice Interface for the Autonomic Nervous System State Manager
interface IANSStateManager {
    /// @notice Update vagal tone for an executor
    /// @param executorId The executor identifier
    /// @param tone The new tone value in ppm (0-1,000,000)
    function updateTone(uint256 executorId, uint32 tone) external;

    /// @notice Get guard information for an action per executor
    /// @param executorId The executor identifier
    /// @param actionId The action identifier
    /// @return scalingFactor The scaling factor in basis points
    /// @return allowed Whether the action is allowed
    function guardFor(uint256 executorId, bytes32 actionId) external view returns (uint256 scalingFactor, bool allowed);

    /// @notice Get executor state information
    /// @param executorId The executor identifier
    /// @return state Current state (0=SAFE, 1=DANGER, 2=SHUTDOWN)
    /// @return tone Current tone in ppm
    /// @return updatedAt Last update timestamp
    function getExecutorState(uint256 executorId) external view returns (uint8 state, uint32 tone, uint64 updatedAt);
}
