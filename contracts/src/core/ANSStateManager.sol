// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Events.sol";

/// @title Autonomic Nervous System State Manager
/// @notice Manages SAFE/DANGER/SHUTDOWN states with hysteresis
contract ANSStateManager is Events {
    /// @notice ANS states enumeration
    enum State { SAFE, DANGER, SHUTDOWN }

    /// @notice Guard information returned by guardFor
    struct Guard {
        uint256 scalingFactor; // 0-10000 (basis points, 10000 = 100%)
        bool allowed;
    }

    /// @notice Current ANS state
    State public currentState;

    /// @notice Current vagal tone value (0-10000)
    uint256 public currentTone;

    /// @notice Last state change timestamp
    uint256 public lastStateChange;

    /// @notice Minimum time between state changes (hysteresis)
    uint256 public constant MIN_STATE_RESIDENCY = 60; // 60 seconds

    /// @notice Tone thresholds for state transitions
    uint256 public constant SAFE_TO_DANGER_THRESHOLD = 3000;   // 30%
    uint256 public constant DANGER_TO_SHUTDOWN_THRESHOLD = 7000; // 70%
    uint256 public constant DANGER_TO_SAFE_THRESHOLD = 1500;    // 15% (hysteresis)
    uint256 public constant SHUTDOWN_TO_DANGER_THRESHOLD = 5000; // 50% (hysteresis)

    /// @notice Scaling factors per state (basis points)
    uint256 public constant SAFE_SCALING = 10000;    // 100%
    uint256 public constant DANGER_SCALING = 6000;   // 60%
    uint256 public constant SHUTDOWN_SCALING = 0;    // 0%

    /// @notice Contract owner
    address public owner;

    /// @notice Custom error for too frequent state changes
    error StateChangeTooFrequent();

    /// @notice Constructor initializes in SAFE state
    constructor() {
        currentState = State.SAFE;
        currentTone = 0;
        lastStateChange = 0; // No state change yet
        owner = msg.sender;
    }

    /// @notice Update vagal tone and potentially change state
    /// @param tone The new tone value (0-10000)
    /// @param suggested The suggested state transition
    function updateTone(uint256 tone, State suggested) external {
        require(msg.sender == owner, "Only owner can update tone");

        // Update tone
        currentTone = tone;

        // Check if state transition is needed
        State newState = _calculateNewState(tone, suggested);

        if (newState != currentState) {
            // Check minimum residency time (only if state has changed before)
            if (lastStateChange != 0 && block.timestamp - lastStateChange < MIN_STATE_RESIDENCY) {
                revert StateChangeTooFrequent();
            }

            currentState = newState;
            lastStateChange = block.timestamp;

            emit VagalToneUpdated(tone, uint8(newState));
        }
    }

    /// @notice Get guard information for an action
    /// @param actionId The action identifier (unused in MVP, all actions use same scaling)
    /// @return guard The guard information with scaling factor and allow flag
    function guardFor(bytes32 actionId) external view returns (Guard memory guard) {
        // For MVP, all actions use the same scaling based on current state
        uint256 scalingFactor;
        bool allowed;

        if (currentState == State.SAFE) {
            scalingFactor = SAFE_SCALING;
            allowed = true;
        } else if (currentState == State.DANGER) {
            scalingFactor = DANGER_SCALING;
            allowed = true;
        } else { // SHUTDOWN
            scalingFactor = SHUTDOWN_SCALING;
            allowed = false;
        }

        return Guard(scalingFactor, allowed);
    }

    /// @notice Get current state as uint8
    /// @return The current state as uint8
    function getCurrentState() external view returns (uint8) {
        return uint8(currentState);
    }

    /// @notice Internal function to calculate new state based on tone and suggestion
    /// @param tone The current tone value
    /// @param suggested The suggested state
    /// @return The new state
    function _calculateNewState(uint256 tone, State suggested) internal view returns (State) {
        State proposedState = suggested;

        // Apply hysteresis logic based on current state
        if (currentState == State.SAFE) {
            if (tone >= SAFE_TO_DANGER_THRESHOLD) {
                proposedState = State.DANGER;
            }
        } else if (currentState == State.DANGER) {
            if (tone >= DANGER_TO_SHUTDOWN_THRESHOLD) {
                proposedState = State.SHUTDOWN;
            } else if (tone <= DANGER_TO_SAFE_THRESHOLD) {
                proposedState = State.SAFE;
            }
        } else { // SHUTDOWN
            if (tone <= SHUTDOWN_TO_DANGER_THRESHOLD) {
                proposedState = State.DANGER;
            }
        }

        return proposedState;
    }
}
