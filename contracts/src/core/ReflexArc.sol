// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Events.sol";
import "../../interfaces/IANSStateManager.sol";
import "../../interfaces/ICapabilityIssuer.sol";
import "../../interfaces/IAfferentInbox.sol";
import "../../interfaces/IReflexArc.sol";

/// @title Reflex Arc
/// @notice Triggers automated revocation based on afferent evidence analysis
contract ReflexArc is Events {
    /// @notice AfferentInbox contract
    address public afferentInbox;

    /// @notice CapabilityIssuer contract
    address public capabilityIssuer;

    /// @notice ANS State Manager contract
    address public ansStateManager;

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

    /// @notice Constructor with dependency injection
    /// @param _afferentInbox Address of the AfferentInbox contract
    /// @param _capabilityIssuer Address of the CapabilityIssuer contract
    /// @param _ansStateManager Address of the ANS State Manager contract
    constructor(address _afferentInbox, address _capabilityIssuer, address _ansStateManager) {
        owner = msg.sender;
        afferentInbox = _afferentInbox;
        capabilityIssuer = _capabilityIssuer;
        ansStateManager = _ansStateManager;
    }

    /// @notice Trigger reflex when ANS state changes (ER2)
    /// @param executorId The executor ID
    /// @param newState The new ANS state
    function on_state_change(uint256 executorId, uint8 newState) external {
        require(msg.sender == owner || msg.sender == address(afferentInbox), "Unauthorized");

        // Rate limiting check - allow if first trigger or cooldown expired
        if (lastReflexTrigger[executorId] != 0 && block.timestamp - lastReflexTrigger[executorId] < REFLEX_COOLDOWN) {
            return; // Skip if cooldown not expired and not first trigger
        }

        // Trigger reflex for DANGER or SHUTDOWN states
        if (newState == 1 || newState == 2) { // DANGER = 1, SHUTDOWN = 2
            _triggerReflexPaginated(executorId, "state_change", 0, 10); // Start with first 10 tokens
            lastReflexTrigger[executorId] = block.timestamp;
        }
    }

    /// @notice Trigger reflex when afferent evidence is posted (ER2)
    /// @param executorId The executor ID
    function on_aep(uint256 executorId) external {
        // Allow owner or afferentInbox to trigger
        require(msg.sender == owner || msg.sender == address(afferentInbox), "Unauthorized");

        // Rate limiting check - allow if first trigger or cooldown expired
        if (lastReflexTrigger[executorId] != 0 && block.timestamp - lastReflexTrigger[executorId] < REFLEX_COOLDOWN) {
            return; // Skip if cooldown not expired and not first trigger
        }

        // Get latest evidence and check if it indicates danger
        bytes32 stateRoot = IAfferentInbox(afferentInbox).latestStateRoot(executorId);
        if (stateRoot == bytes32(0)) {
            return; // No evidence available
        }

        // For MVP: Check if executor indicates danger (executor 999)
        // In production: Analyze actual sensor data from state root
        if (_shouldTriggerReflex(executorId)) {
            _triggerReflexPaginated(executorId, "danger_detected", 0, 10); // Start with first 10 tokens
            lastReflexTrigger[executorId] = block.timestamp;
        }
    }

    /// @notice Pulse mechanism to continue paginated revocation (ER2)
    /// @param executorId The executor ID
    /// @param startIndex Starting index for pagination
    /// @param maxCount Maximum tokens to process
    function pulse(uint256 executorId, uint256 startIndex, uint256 maxCount) external {
        require(msg.sender == owner || msg.sender == address(afferentInbox), "Unauthorized");

        _triggerReflexPaginated(executorId, "pulse_continue", startIndex, maxCount);
    }

    /// @notice Manually trigger reflex for testing/emergency
    /// @param executorId The executor ID to trigger reflex for
    /// @param reason The reason for triggering
    function manualTrigger(uint256 executorId, string calldata reason) external {
        require(msg.sender == owner, "Only owner can manually trigger reflex");

        _triggerReflexPaginated(executorId, reason, 0, 50); // Process up to 50 tokens
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

    /// @notice Internal function to execute paginated reflex action (ER2)
    /// @param executorId The executor ID
    /// @param reason The reason for triggering
    /// @param startIndex Starting index in active tokens list
    /// @param maxCount Maximum number of tokens to process
    function _triggerReflexPaginated(uint256 executorId, string memory reason, uint256 startIndex, uint256 maxCount) internal {
        // Get all active tokens for this executor
        uint256[] memory activeTokens = ICapabilityIssuer(capabilityIssuer).activeTokensOf(executorId);

        // Process tokens with pagination
        uint256 processedCount = 0;
        uint256 revokedCount = 0;
        uint256[] memory revokedTokens = new uint256[](maxCount);

        for (uint256 i = startIndex; i < activeTokens.length && processedCount < maxCount; i++) {
            uint256 tokenId = activeTokens[i];
            if (ICapabilityIssuer(capabilityIssuer).isValid(tokenId)) {
                ICapabilityIssuer(capabilityIssuer).revoke(tokenId, 1);
                revokedTokens[revokedCount] = tokenId;
                revokedCount++;
            }
            processedCount++;
        }

        // Emit event if any tokens were revoked
        if (revokedCount > 0) {
            // Create a compact array with only revoked tokens
            uint256[] memory actualRevokedTokens = new uint256[](revokedCount);
            for (uint256 i = 0; i < revokedCount; i++) {
                actualRevokedTokens[i] = revokedTokens[i];
            }

            emit ReflexTriggered(
                executorId,
                reason,
                revokedCount,
                block.timestamp
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
