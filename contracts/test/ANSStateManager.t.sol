// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/core/ANSStateManager.sol";

contract ANSStateManagerTest is Test {
    ANSStateManager ans;
    uint256 constant EXECUTOR_ID = 1;

    function setUp() public {
        ans = new ANSStateManager();
    }

    function testInitialState() public {
        (uint8 state, uint32 tone, uint64 updatedAt) = ans.getExecutorState(EXECUTOR_ID);
        assertEq(state, 0); // SAFE
        assertEq(tone, 0);
        assertEq(updatedAt, 0);
    }

    function testUpdateToneSafeToDanger() public {
        // Need 3 consecutive readings above danger threshold (300,000 ppm = 30%)
        // Convert 35% to ppm: 350,000
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - above 30% threshold
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - consecutive reading 2
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - consecutive reading 3 -> should transition

        (uint8 state, uint32 tone, uint64 updatedAt) = ans.getExecutorState(EXECUTOR_ID);
        assertEq(state, 1); // DANGER
        assertEq(tone, 350000);
    }

    function testHysteresisDangerToSafe() public {
        // Go to DANGER state (3 consecutive readings above 30%)
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - reading 1
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - reading 2
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - reading 3 -> DANGER
        (uint8 state,,) = ans.getExecutorState(EXECUTOR_ID);
        assertEq(state, 1); // DANGER

        // Try to go back to SAFE - need 5 consecutive readings below safeExitTone (15%)
        // But hysteresis prevents immediate transition even if we meet the count
        // First, readings in hysteresis zone (16%-29%) should reset counters
        ans.updateTone(EXECUTOR_ID, 200000); // 20% - in hysteresis zone, reset counters
        ans.updateTone(EXECUTOR_ID, 200000); // 20% - still in zone
        (state,,) = ans.getExecutorState(EXECUTOR_ID);
        assertEq(state, 1); // Still DANGER

        // Now get 5 consecutive readings below safeExitTone (15%)
        // Advance time past dwell period first
        vm.warp(block.timestamp + 61); // 60 seconds + 1

        ans.updateTone(EXECUTOR_ID, 100000); // 10% - below 15%, start safe counter
        ans.updateTone(EXECUTOR_ID, 100000); // 10% - counter = 2
        ans.updateTone(EXECUTOR_ID, 100000); // 10% - counter = 3
        ans.updateTone(EXECUTOR_ID, 100000); // 10% - counter = 4
        ans.updateTone(EXECUTOR_ID, 100000); // 10% - counter = 5 -> should transition to SAFE
        (state,,) = ans.getExecutorState(EXECUTOR_ID);
        assertEq(state, 0); // Now SAFE
    }

    function testMinimumDwellTime() public {
        // Go to DANGER state
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - reading 1
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - reading 2
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - reading 3 -> DANGER

        // Immediately try to go to SHUTDOWN - should stay in DANGER due to dwell time
        ans.updateTone(EXECUTOR_ID, 800000); // 80% - above shutdown threshold
        ans.updateTone(EXECUTOR_ID, 800000); // 80% - consecutive reading 2 -> should NOT transition due to dwell time
        (uint8 state,,) = ans.getExecutorState(EXECUTOR_ID);
        assertEq(state, 1); // Still DANGER

        // Wait for dwell time to expire
        vm.warp(block.timestamp + 61); // 60 seconds + 1

        // Now try SHUTDOWN transition again
        ans.updateTone(EXECUTOR_ID, 800000); // 80% - reading 1 (after dwell time)
        ans.updateTone(EXECUTOR_ID, 800000); // 80% - reading 2 -> should transition to SHUTDOWN
        (state,,) = ans.getExecutorState(EXECUTOR_ID);
        assertEq(state, 2); // Now SHUTDOWN
    }

    function testGuardForSafe() public {
        (uint256 scalingFactor, bool allowed) = ans.guardFor(EXECUTOR_ID, keccak256("test_action"));
        assertEq(scalingFactor, 10000); // 100%
        assertTrue(allowed);
    }

    function testGuardForDanger() public {
        // Go to DANGER state
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - reading 1
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - reading 2
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - reading 3 -> DANGER

        (uint256 scalingFactor, bool allowed) = ans.guardFor(EXECUTOR_ID, keccak256("test_action"));
        assertEq(scalingFactor, 6000); // 60%
        assertTrue(allowed);
    }

    function testGuardForShutdown() public {
        // First go to DANGER state
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - reading 1
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - reading 2
        ans.updateTone(EXECUTOR_ID, 350000); // 35% - reading 3 -> DANGER

        // Wait for dwell time before attempting SHUTDOWN transition
        vm.warp(block.timestamp + 61); // 60 seconds + 1

        // Then go to SHUTDOWN state (2 consecutive readings above 70%)
        ans.updateTone(EXECUTOR_ID, 800000); // 80% - reading 1
        ans.updateTone(EXECUTOR_ID, 800000); // 80% - reading 2 -> SHUTDOWN

        (uint256 scalingFactor, bool allowed) = ans.guardFor(EXECUTOR_ID, keccak256("test_action"));
        assertEq(scalingFactor, 0); // 0%
        assertFalse(allowed);
    }
}
