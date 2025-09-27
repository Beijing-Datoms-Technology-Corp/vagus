// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/core/ReflexArc.sol";
import "../src/core/AfferentInbox.sol";
import "../src/core/CapabilityIssuer.sol";
import "../src/core/VagalBrake.sol";
import "../src/core/ANSStateManager.sol";
import "../src/core/Types.sol";

contract ReflexArcTest is Test {
    ReflexArc reflex;
    AfferentInbox inbox;
    CapabilityIssuer issuer;
    VagalBrake brake;
    ANSStateManager ans;

    address user = address(0x123);

    function setUp() public {
        inbox = new AfferentInbox();
        ans = new ANSStateManager();
        issuer = new CapabilityIssuer(address(inbox));
        brake = new VagalBrake(address(ans), address(issuer));
        reflex = new ReflexArc(address(inbox), address(issuer));

        // Authorize ReflexArc in CapabilityIssuer
        issuer.setReflexArc(address(reflex));

        // Authorize the deployer as attestor
        inbox.authorizeAttestor(address(this));
    }

    function testManualTrigger() public {
        // Issue some tokens
        Types.Intent memory intent = Types.Intent({
            executorId: 42,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: bytes32(0),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 1000,
            maxEnergyJ: 500,
            planner: address(this),
            nonce: 1
        });

        uint256 tokenId = issuer.issueCapability(intent, keccak256("limits"));

        // The reflex contract owner is the deployer (address(this) in setUp)
        // So manualTrigger should work
        reflex.manualTrigger(42, "manual_test");

        // Token should be revoked
        assertFalse(issuer.isValid(tokenId));
    }

    function testReflexTrigger() public {
        // Post some evidence for executor 999 (which triggers reflex)
        inbox.postAEP(999, keccak256("state"), keccak256("metrics"), "");

        // Issue some tokens for executor 999
        Types.Intent memory intent = Types.Intent({
            executorId: 999,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: bytes32(0),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 1000,
            maxEnergyJ: 500,
            planner: address(this),
            nonce: 1
        });

        uint256 tokenId1 = issuer.issueCapability(intent, keccak256("limits1"));
        intent.nonce = 2;
        uint256 tokenId2 = issuer.issueCapability(intent, keccak256("limits2"));

        // Verify tokens are active
        assertTrue(issuer.isValid(tokenId1));
        assertTrue(issuer.isValid(tokenId2));
        uint256[] memory active = issuer.activeTokensOf(999);
        assertEq(active.length, 2);

        // Trigger reflex analysis
        reflex.analyzeAndTrigger(999);

        // Tokens should be revoked
        assertFalse(issuer.isValid(tokenId1));
        assertFalse(issuer.isValid(tokenId2));
        active = issuer.activeTokensOf(999);
        assertEq(active.length, 0);
    }

    function testRateLimiting() public {
        // Post evidence
        inbox.postAEP(999, keccak256("state"), keccak256("metrics"), "");

        // Issue token
        Types.Intent memory intent = Types.Intent({
            executorId: 999,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: bytes32(0),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 1000,
            maxEnergyJ: 500,
            planner: address(this),
            nonce: 1
        });

        uint256 tokenId = issuer.issueCapability(intent, keccak256("limits"));

        // First trigger should work
        reflex.analyzeAndTrigger(999);
        assertFalse(issuer.isValid(tokenId));

        // Re-issue token
        intent.nonce = 2;
        uint256 tokenId2 = issuer.issueCapability(intent, keccak256("limits2"));

        // Advance time past cooldown
        vm.warp(block.timestamp + 31);

        // Second trigger should work again after cooldown
        reflex.analyzeAndTrigger(999);
        assertFalse(issuer.isValid(tokenId2)); // Should be revoked again
    }

    function testNoEvidence() public {
        // Try to analyze executor with no evidence
        reflex.analyzeAndTrigger(42);
        // Should not revert, just do nothing
    }

    function testNoTriggerCondition() public {
        // Post evidence for executor 42 (which doesn't trigger reflex in MVP)
        inbox.postAEP(42, keccak256("state"), keccak256("metrics"), "");

        // Issue token
        Types.Intent memory intent = Types.Intent({
            executorId: 42,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: bytes32(0),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 1000,
            maxEnergyJ: 500,
            planner: address(this),
            nonce: 1
        });

        uint256 tokenId = issuer.issueCapability(intent, keccak256("limits"));

        // Analysis should not trigger reflex
        reflex.analyzeAndTrigger(42);
        assertTrue(issuer.isValid(tokenId)); // Still valid
    }
}
