// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Events.sol";
import "./Interfaces.sol";

/// @title Autonomic Nervous System State Manager
/// @notice Manages SAFE/DANGER/SHUTDOWN states with hysteresis per executor
contract ANSStateManager is Events {
    /// @notice ANS states enumeration
    enum State { SAFE, DANGER, SHUTDOWN }

    /// @notice Hysteresis configuration
    struct HysteresisConfig {
        uint32 dangerEnterTone;     // ppm (enter DANGER threshold)
        uint32 safeExitTone;        // ppm (exit DANGER to SAFE, > dangerEnterTone)
        uint32 shutdownEnterTone;   // ppm (enter SHUTDOWN threshold)
        uint8  nDangerEnter;        // consecutive count to enter DANGER
        uint8  nSafeExit;           // consecutive count to exit DANGER
        uint8  nShutdownEnter;      // consecutive count to enter SHUTDOWN
        uint32 dwellMinSec;         // minimum dwell time between transitions
    }

    /// @notice Per-executor state information
    struct ExecutorState {
        uint8  state;               // 0 SAFE, 1 DANGER, 2 SHUTDOWN
        uint32 tone;                // current tone in ppm
        uint64 updatedAt;           // last update timestamp
        uint64 lastTransitionAt;    // last state transition timestamp
        uint8  ctrDanger;           // consecutive danger enter counter
        uint8  ctrSafe;             // consecutive safe exit counter
        uint8  ctrShutdown;         // consecutive shutdown enter counter
    }

    /// @notice Hysteresis configuration (governable)
    HysteresisConfig public cfg;

    /// @notice Per-executor state mapping
    mapping(uint256 => ExecutorState) public executorStates;

    /// @notice Scaling factors per state (basis points)
    uint256 public constant SAFE_SCALING = 10000;    // 100%
    uint256 public constant DANGER_SCALING = 6000;   // 60%
    uint256 public constant SHUTDOWN_SCALING = 0;    // 0%

    /// @notice Contract owner
    address public owner;

    /// @notice ReflexArc contract for state change notifications
    address public reflexArc;

    /// @notice Custom error for too frequent state changes
    error StateChangeTooFrequent();

    /// @notice Constructor initializes with default hysteresis config
    constructor() {
        owner = msg.sender;
        // Set default hysteresis configuration
        cfg = HysteresisConfig({
            dangerEnterTone: 300000,     // 30% (300,000 ppm)
            safeExitTone: 150000,        // 15% (150,000 ppm) - hysteresis
            shutdownEnterTone: 700000,   // 70% (700,000 ppm)
            nDangerEnter: 3,             // 3 consecutive readings
            nSafeExit: 5,                // 5 consecutive readings
            nShutdownEnter: 2,           // 2 consecutive readings
            dwellMinSec: 60              // 60 seconds minimum dwell
        });
    }

    /// @notice Update vagal tone for an executor (oracle/gateway calls this)
    /// @param executorId The executor identifier
    /// @param tone The new tone value in ppm (0-1,000,000)
    function updateTone(uint256 executorId, uint32 tone) external {
        require(msg.sender == owner, "Only owner can update tone");

        ExecutorState storage s = executorStates[executorId];
        uint64 nowS = uint64(block.timestamp);

        // 1) 更新连续计数 - 基于当前状态和tone值
        if (s.state == 0) { // SAFE状态
            if (tone >= cfg.dangerEnterTone) {
                s.ctrDanger = s.ctrDanger < 255 ? s.ctrDanger + 1 : 255;
            } else {
                s.ctrDanger = 0;
            }
            s.ctrSafe = 0; // 在SAFE状态时重置safe计数器
            s.ctrShutdown = 0; // 在SAFE状态时重置shutdown计数器
        } else if (s.state == 1) { // DANGER状态
            if (tone >= cfg.shutdownEnterTone) {
                s.ctrShutdown = s.ctrShutdown < 255 ? s.ctrShutdown + 1 : 255;
            } else {
                s.ctrShutdown = 0;
            }
            if (tone < cfg.safeExitTone) {
                s.ctrSafe = s.ctrSafe < 255 ? s.ctrSafe + 1 : 255;
            } else {
                s.ctrSafe = 0;
            }
            s.ctrDanger = 0; // 在DANGER状态时重置danger计数器
        } else { // SHUTDOWN状态
            // 从SHUTDOWN状态通常不会转换，但可以实现从SHUTDOWN回到DANGER的逻辑
            s.ctrDanger = 0;
            s.ctrSafe = 0;
            s.ctrShutdown = 0;
        }

        bool canTransition = (s.lastTransitionAt == 0) || (nowS - s.lastTransitionAt) >= cfg.dwellMinSec;

        // 2) 按优先级触发：SHUTDOWN > DANGER > SAFE
        if (s.ctrShutdown >= cfg.nShutdownEnter && canTransition && s.state != 2) {
            _setState(executorId, 2 /*SHUTDOWN*/, tone, nowS);
            return;
        }
        if (s.state == 0 /*SAFE*/ && s.ctrDanger >= cfg.nDangerEnter && canTransition) {
            _setState(executorId, 1 /*DANGER*/, tone, nowS);
            return;
        }
        if (s.state == 1 /*DANGER*/ && s.ctrSafe >= cfg.nSafeExit && canTransition) {
            _setState(executorId, 0 /*SAFE*/, tone, nowS);
            return;
        }

        // 3) 无状态变化，仅更新 tone/时间戳
        s.tone = tone;
        s.updatedAt = nowS;
    }

    /// @notice Internal function to set executor state and notify ReflexArc
    /// @param executorId The executor identifier
    /// @param newState The new state (0=SAFE, 1=DANGER, 2=SHUTDOWN)
    /// @param tone The current tone value
    /// @param timestamp The timestamp of the transition
    function _setState(uint256 executorId, uint8 newState, uint32 tone, uint64 timestamp) internal {
        ExecutorState storage s = executorStates[executorId];

        // Update state
        s.state = newState;
        s.tone = tone;
        s.updatedAt = timestamp;
        s.lastTransitionAt = timestamp;

        // Clear counters after transition
        s.ctrDanger = 0;
        s.ctrSafe = 0;
        s.ctrShutdown = 0;

        // Emit canonical events
        emit VagalToneUpdated(uint256(tone), newState, timestamp);

        // Notify ReflexArc if configured (fail silently)
        if (reflexArc != address(0)) {
            try IReflexArc(reflexArc).on_state_change(executorId, newState) {
                // Success - ReflexArc notified
            } catch {
                // Failure - log but don't revert (ReflexArc issues shouldn't break ANS)
            }
        }
    }

    /// @notice Set ReflexArc contract address
    /// @param _reflexArc The ReflexArc contract address
    function setReflexArc(address _reflexArc) external {
        require(msg.sender == owner, "Only owner can set reflex arc");
        reflexArc = _reflexArc;
    }

    /// @notice Get guard information for an action (per executor)
    /// @param executorId The executor identifier
    /// @param actionId The action identifier (unused in MVP)
    /// @return scalingFactor The scaling factor in basis points
    /// @return allowed Whether the action is allowed
    function guardFor(uint256 executorId, bytes32 actionId) external view returns (uint256 scalingFactor, bool allowed) {
        uint8 state = executorStates[executorId].state;

        if (state == 0) { // SAFE
            scalingFactor = SAFE_SCALING;
            allowed = true;
        } else if (state == 1) { // DANGER
            scalingFactor = DANGER_SCALING;
            allowed = true;
        } else { // SHUTDOWN
            scalingFactor = SHUTDOWN_SCALING;
            allowed = false;
        }
    }

    /// @notice Get executor state information
    /// @param executorId The executor identifier
    /// @return state Current state (0=SAFE, 1=DANGER, 2=SHUTDOWN)
    /// @return tone Current tone in ppm
    /// @return updatedAt Last update timestamp
    function getExecutorState(uint256 executorId) external view returns (uint8 state, uint32 tone, uint64 updatedAt) {
        ExecutorState storage s = executorStates[executorId];
        return (s.state, s.tone, s.updatedAt);
    }

    /// @notice Get global current state (deprecated - use getExecutorState)
    /// @return The current state as uint8 (always returns 0 for backward compatibility)
    function getCurrentState() external view returns (uint8) {
        return 0; // SAFE - for backward compatibility
    }
}
