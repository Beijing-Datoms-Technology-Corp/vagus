// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/core/ANSStateManager.sol";

contract ANSStateManagerTest is Test {
    ANSStateManager ans;

    function setUp() public {
        ans = new ANSStateManager();
    }

    function testInitialState() public {
        assertEq(ans.getCurrentState(), 0); // SAFE
        assertEq(ans.currentTone(), 0);
    }

    function testUpdateToneSafeToDanger() public {
        // Update tone above threshold
        ans.updateTone(3500, ANSStateManager.State.DANGER);

        assertEq(ans.getCurrentState(), 1); // DANGER
        assertEq(ans.currentTone(), 3500);
    }

    function testHysteresisDangerToSafe() public {
        // Go to DANGER state
        ans.updateTone(3500, ANSStateManager.State.DANGER);
        assertEq(ans.getCurrentState(), 1); // DANGER

        // Wait for residency time before trying to change back
        vm.warp(block.timestamp + 61);

        // Try to go back to SAFE - should stay in DANGER due to hysteresis (threshold is 1500)
        ans.updateTone(2000, ANSStateManager.State.SAFE);
        assertEq(ans.getCurrentState(), 1); // Still DANGER

        // Wait again and drop below hysteresis threshold
        vm.warp(block.timestamp + 122);
        ans.updateTone(1000, ANSStateManager.State.SAFE);
        assertEq(ans.getCurrentState(), 0); // Now SAFE
    }

    function testStateChangeTooFrequent() public {
        // Go to DANGER
        ans.updateTone(3500, ANSStateManager.State.DANGER);

        // Try to change state immediately - should revert
        vm.expectRevert(ANSStateManager.StateChangeTooFrequent.selector);
        ans.updateTone(8000, ANSStateManager.State.SHUTDOWN);
    }

    function testGuardForSafe() public {
        ANSStateManager.Guard memory guard = ans.guardFor(keccak256("test_action"));
        assertEq(guard.scalingFactor, 10000); // 100%
        assertTrue(guard.allowed);
    }

    function testGuardForDanger() public {
        // Go to DANGER - need to advance time for each test since setup() resets
        vm.warp(block.timestamp + 61);
        ans.updateTone(3500, ANSStateManager.State.DANGER);

        ANSStateManager.Guard memory guard = ans.guardFor(keccak256("test_action"));
        assertEq(guard.scalingFactor, 6000); // 60%
        assertTrue(guard.allowed);
    }

    function testGuardForShutdown() public {
        // Go to DANGER first
        vm.warp(block.timestamp + 61);
        ans.updateTone(3500, ANSStateManager.State.DANGER);
        // Then to SHUTDOWN
        vm.warp(block.timestamp + 122);
        ans.updateTone(8000, ANSStateManager.State.SHUTDOWN);

        ANSStateManager.Guard memory guard = ans.guardFor(keccak256("test_action"));
        assertEq(guard.scalingFactor, 0); // 0%
        assertFalse(guard.allowed);
    }
}
