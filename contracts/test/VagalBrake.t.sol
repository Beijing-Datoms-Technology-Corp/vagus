// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/core/VagalBrake.sol";
import "../src/core/ANSStateManager.sol";
import "../src/core/CapabilityIssuer.sol";
import "../src/core/AfferentInbox.sol";
import "../src/core/Types.sol";

contract VagalBrakeTest is Test {
    VagalBrake brake;
    ANSStateManager ans;
    CapabilityIssuer issuer;
    AfferentInbox inbox;

    address user = address(0x123);

    function setUp() public {
        inbox = new AfferentInbox();
        ans = new ANSStateManager();
        issuer = new CapabilityIssuer(address(inbox));
        brake = new VagalBrake(address(ans), address(issuer));
    }

    function testPreviewBrakeSafe() public {
        Types.Intent memory intent = Types.Intent({
            executorId: 42,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: bytes32(0),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 2000, // Will be scaled to 2000 (no scaling in SAFE)
            maxEnergyJ: 800,     // Will be scaled to 800
            planner: user,
            nonce: 1
        });

        (bytes32 limitsHash, bool allowed) = brake.previewBrake(intent);

        assertTrue(allowed);
        assertNotEq(limitsHash, bytes32(0));
    }

    function testPreviewBrakeDanger() public {
        // Put ANS in DANGER state
        vm.warp(block.timestamp + 61);
        ans.updateTone(3500, ANSStateManager.State.DANGER);

        Types.Intent memory intent = Types.Intent({
            executorId: 42,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: bytes32(0),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 2000, // Will be scaled to 1200 (60%)
            maxEnergyJ: 800,     // Will be scaled to 480 (60%)
            planner: user,
            nonce: 1
        });

        (bytes32 limitsHash, bool allowed) = brake.previewBrake(intent);

        assertTrue(allowed);
        assertNotEq(limitsHash, bytes32(0));
    }

    function testPreviewBrakeShutdown() public {
        // Put ANS in SHUTDOWN state
        vm.warp(block.timestamp + 61);
        ans.updateTone(3500, ANSStateManager.State.DANGER);
        vm.warp(block.timestamp + 122);
        ans.updateTone(8000, ANSStateManager.State.SHUTDOWN);

        Types.Intent memory intent = Types.Intent({
            executorId: 42,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: bytes32(0),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 2000,
            maxEnergyJ: 800,
            planner: user,
            nonce: 1
        });

        (bytes32 limitsHash, bool allowed) = brake.previewBrake(intent);

        assertFalse(allowed);
        assertEq(limitsHash, bytes32(0));
    }

    function testIssueWithBrakeSafe() public {
        Types.Intent memory intent = Types.Intent({
            executorId: 42,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: bytes32(0),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 2000,
            maxEnergyJ: 800,
            planner: user,
            nonce: 1
        });

        uint256 tokenId = brake.issueWithBrake(intent);

        assertEq(tokenId, 1);
        assertTrue(issuer.isValid(tokenId));
    }

    function testIssueWithBrakeShutdown() public {
        // Put ANS in SHUTDOWN state
        vm.warp(block.timestamp + 61);
        ans.updateTone(3500, ANSStateManager.State.DANGER);
        vm.warp(block.timestamp + 122);
        ans.updateTone(8000, ANSStateManager.State.SHUTDOWN);

        Types.Intent memory intent = Types.Intent({
            executorId: 42,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: bytes32(0),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 2000,
            maxEnergyJ: 800,
            planner: user,
            nonce: 1
        });

        vm.expectRevert(abi.encodeWithSelector(VagalBrake.ANSBlocked.selector, "ANS:blocked"));
        brake.issueWithBrake(intent);
    }

    function testLimitExceeded() public {
        // Put ANS in DANGER state (60% scaling)
        vm.warp(block.timestamp + 61);
        ans.updateTone(3500, ANSStateManager.State.DANGER);

        Types.Intent memory intent = Types.Intent({
            executorId: 42,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: bytes32(0),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 60000, // 60000 * 0.6 = 36000 > MAX_DURATION_MS (30000)
            maxEnergyJ: 800,
            planner: user,
            nonce: 1
        });

        vm.expectRevert(abi.encodeWithSelector(VagalBrake.ANSLimitExceeded.selector, "maxDurationMs", uint256(36000), uint256(30000)));
        brake.issueWithBrake(intent);
    }
}
