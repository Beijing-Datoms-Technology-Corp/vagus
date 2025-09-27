// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Events.sol";
import "./Interfaces.sol";

/// @title Reflex Arc
/// @notice Triggers automated revocation based on afferent evidence analysis
contract ReflexArc is Events {
    /// @notice AfferentInbox contract
    address public afferentInbox;

    /// @notice CapabilityIssuer contract
    address public capabilityIssuer;

    /// @notice Contract owner
    address public owner;

    /// @notice Last reflex trigger timestamp per executor (for rate limiting)
    mapping(uint256 => uint256) public lastReflexTrigger;

    /// @notice Minimum time between reflex triggers per executor
    uint256 public constant REFLEX_COOLDOWN = 30; // 30 seconds

    /// @notice Danger thresholds for reflex triggering
    uint256 public constant HUMAN_DISTANCE_DANGER = 500; // mm
    uint256 public constant TEMPERATURE_DANGER = 80;     // Celsius
    uint256 public constant ENERGY_LOW_DANGER = 10;      // percent
    uint256 public constant JERK_DANGER = 1000;          // mm/s²

    /// @notice Constructor
    /// @param _afferentInbox Address of the AfferentInbox contract
    /// @param _capabilityIssuer Address of the CapabilityIssuer contract
    constructor(address _afferentInbox, address _capabilityIssuer) {
        owner = msg.sender;
        afferentInbox = _afferentInbox;
        capabilityIssuer = _capabilityIssuer;
    }

    /// @notice Analyze afferent evidence and trigger reflex if needed
    /// @param executorId The executor ID to analyze
    function analyzeAndTrigger(uint256 executorId) external {
        // Rate limiting check
        if (block.timestamp - lastReflexTrigger[executorId] < REFLEX_COOLDOWN) {
            return; // Skip if cooldown not expired
        }

        // Get latest evidence from AfferentInbox
        bytes32 stateRoot = IAfferentInbox(afferentInbox).latestStateRoot(executorId);
        if (stateRoot == bytes32(0)) {
            return; // No evidence available
        }

        // TODO: In a full implementation, we would decode the state root
        // and metrics hash to analyze sensor data. For MVP, we use a simplified approach.

        // For MVP: Simulate reflex triggering based on executor ID
        // In production, this would analyze actual sensor data from the state root
        bool shouldTrigger = _shouldTriggerReflex(executorId);

        if (shouldTrigger) {
            _triggerReflex(executorId, "danger_detected");
            lastReflexTrigger[executorId] = block.timestamp;
        }
    }

    /// @notice Manually trigger reflex for testing/emergency
    /// @param executorId The executor ID to trigger reflex for
    /// @param reason The reason for triggering
    function manualTrigger(uint256 executorId, string calldata reason) external {
        require(msg.sender == owner, "Only owner can manually trigger reflex");

        _triggerReflex(executorId, reason);
    }

    /// @notice Internal function to determine if reflex should be triggered
    /// @param executorId The executor ID
    /// @return True if reflex should be triggered
    function _shouldTriggerReflex(uint256 executorId) internal view returns (bool) {
        // For MVP: Simple logic based on executor ID for testing
        // In production: Analyze decoded sensor data from state root

        // Simulate: executor 999 always triggers reflex (for testing)
        if (executorId == 999) {
            return true;
        }

        // TODO: Implement actual sensor data analysis:
        // - Decode state root to get sensor measurements
        // - Check human distance, temperature, energy, jerk against thresholds
        // - Return true if any danger condition is met

        return false;
    }

    /// @notice Internal function to execute reflex action
    /// @param executorId The executor ID
    /// @param reason The reason for triggering
    function _triggerReflex(uint256 executorId, string memory reason) internal {
        // Get all active tokens for this executor
        uint256[] memory activeTokens = ICapabilityIssuer(capabilityIssuer).activeTokensOf(executorId);

        // Revoke all active tokens
        uint256 revokedCount = 0;
        for (uint256 i = 0; i < activeTokens.length; i++) {
            uint256 tokenId = activeTokens[i];
            if (ICapabilityIssuer(capabilityIssuer).isValid(tokenId)) {
                ICapabilityIssuer(capabilityIssuer).revoke(tokenId, 1);
                revokedCount++;
            }
        }

        if (revokedCount > 0) {
            emit ReflexTriggered(
                executorId,
                keccak256(abi.encodePacked(reason)),
                activeTokens
            );
        }
    }

    /// @notice Update danger thresholds (admin only)
    /// @param humanDistance New human distance threshold
    /// @param temperature New temperature threshold
    /// @param energyLow New energy low threshold
    /// @param jerk New jerk threshold
    function updateThresholds(
        uint256 humanDistance,
        uint256 temperature,
        uint256 energyLow,
        uint256 jerk
    ) external {
        require(msg.sender == owner, "Only owner can update thresholds");
        // TODO: Store and use these thresholds in _shouldTriggerReflex
    }
}
