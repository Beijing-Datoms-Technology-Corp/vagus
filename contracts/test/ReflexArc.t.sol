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
        issuer = new CapabilityIssuer(address(inbox), address(0)); // Will set vagalBrake later
        brake = new VagalBrake(address(ans), address(issuer));
        issuer.setVagalBrake(address(brake));
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
            preStateRoot: inbox.latestStateRoot(999),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 1000,
            maxEnergyJ: 500,
            planner: address(this),
            nonce: 1
        });

        // Get the correct scaled limits hash from VagalBrake
        (bytes32 scaledLimitsHash, bool allowed) = brake.previewBrake(intent);
        require(allowed, "Brake should allow this intent");

        uint256 tokenId = issuer.issueCapability(intent, scaledLimitsHash);

        // The reflex contract owner is the deployer (address(this) in setUp)
        // So manualTrigger should work
        reflex.manualTrigger(42, "manual_test");

        // Token should be revoked
        assertFalse(issuer.isValid(tokenId));
    }

    function testReflexTrigger() public {
        // Reset time to ensure clean state
        vm.warp(1000000);

        // Post some evidence for executor 999 (which triggers reflex)
        inbox.postAEP(999, bytes32(uint256(1)), bytes32(uint256(2)), bytes(""));

        // Issue some tokens for executor 999
        Types.Intent memory intent = Types.Intent({
            executorId: 999,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: inbox.latestStateRoot(999),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 1000,
            maxEnergyJ: 500,
            planner: address(this),
            nonce: 1
        });

        // Get the correct scaled limits hash from VagalBrake
        (bytes32 scaledLimitsHash, bool allowed) = brake.previewBrake(intent);
        require(allowed, "Brake should allow this intent");

        uint256 tokenId1 = issuer.issueCapability(intent, scaledLimitsHash);
        intent.nonce = 2;
        (scaledLimitsHash, allowed) = brake.previewBrake(intent);
        require(allowed, "Brake should allow this intent");

        uint256 tokenId2 = issuer.issueCapability(intent, scaledLimitsHash);

        // Verify tokens are active
        assertTrue(issuer.isValid(tokenId1));
        assertTrue(issuer.isValid(tokenId2));
        uint256[] memory active = issuer.activeTokensOf(999);
        assertEq(active.length, 2);

        // Trigger reflex analysis
        reflex.on_aep(999);

        // Tokens should be revoked
        assertFalse(issuer.isValid(tokenId1));
        assertFalse(issuer.isValid(tokenId2));
        active = issuer.activeTokensOf(999);
        assertEq(active.length, 0);
    }

    function testRateLimiting() public {
        // Reset time to ensure clean state
        vm.warp(2000000);

        // Post evidence
        inbox.postAEP(999, bytes32(uint256(1)), bytes32(uint256(2)), bytes(""));

        // Issue token
        Types.Intent memory intent = Types.Intent({
            executorId: 999,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: inbox.latestStateRoot(999),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 1000,
            maxEnergyJ: 500,
            planner: address(this),
            nonce: 1
        });

                // Get the correct scaled limits hash from VagalBrake
        (bytes32 scaledLimitsHash, bool allowed) = brake.previewBrake(intent);
        require(allowed, "Brake should allow this intent");

        uint256 tokenId = issuer.issueCapability(intent, scaledLimitsHash);

        // First trigger should work
        reflex.on_aep(999);
        assertFalse(issuer.isValid(tokenId));

        // Re-issue token
        intent.nonce = 2;
        (scaledLimitsHash, allowed) = brake.previewBrake(intent);
        require(allowed, "Brake should allow this intent");

        uint256 tokenId2 = issuer.issueCapability(intent, scaledLimitsHash);

        // Advance time past cooldown
        vm.warp(block.timestamp + 31);

        // Second trigger should work again after cooldown
        reflex.on_aep(999);
        assertFalse(issuer.isValid(tokenId2)); // Should be revoked again
    }

    function testNoEvidence() public {
        // Try to analyze executor with no evidence
        reflex.on_aep(42);
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
            preStateRoot: inbox.latestStateRoot(42),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 3600),
            maxDurationMs: 1000,
            maxEnergyJ: 500,
            planner: address(this),
            nonce: 1
        });

                // Get the correct scaled limits hash from VagalBrake
        (bytes32 scaledLimitsHash, bool allowed) = brake.previewBrake(intent);
        require(allowed, "Brake should allow this intent");

        uint256 tokenId = issuer.issueCapability(intent, scaledLimitsHash);

        // Analysis should not trigger reflex
        reflex.on_aep(42);
        assertTrue(issuer.isValid(tokenId)); // Still valid
    }
}
