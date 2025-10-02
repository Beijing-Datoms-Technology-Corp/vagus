// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// @title Reflex Arc Interface
/// @notice Interface for triggering automated revocation based on afferent evidence analysis
interface IReflexArc {
    /// @notice Trigger reflex when ANS state changes
    /// @param executorId The executor ID
    /// @param newState The new ANS state
    function on_state_change(uint256 executorId, uint8 newState) external;

    /// @notice Trigger reflex when afferent evidence is posted
    /// @param executorId The executor ID
    function on_aep(uint256 executorId) external;

    /// @notice Pulse mechanism to continue paginated revocation
    /// @param executorId The executor ID
    /// @param startIndex Starting index for pagination
    /// @param maxCount Maximum tokens to process
    function pulse(uint256 executorId, uint256 startIndex, uint256 maxCount) external;

    /// @notice Manually trigger reflex for testing/emergency
    /// @param executorId The executor ID to trigger reflex for
    /// @param reason The reason for triggering
    function manualTrigger(uint256 executorId, string calldata reason) external;

    /// @notice Update danger thresholds
    /// @param humanDistance New human distance threshold
    /// @param temperature New temperature threshold
    /// @param energyLow New energy low threshold
    /// @param jerk New jerk threshold
    function updateThresholds(
        uint256 humanDistance,
        uint256 temperature,
        uint256 energyLow,
        uint256 jerk
    ) external;
}
