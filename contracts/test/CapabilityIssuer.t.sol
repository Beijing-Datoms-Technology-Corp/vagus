// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/core/CapabilityIssuer.sol";
import "../src/core/AfferentInbox.sol";
import "../src/core/Types.sol";

contract CapabilityIssuerTest is Test {
    CapabilityIssuer issuer;
    AfferentInbox inbox;

    address user = address(0x123);

    function setUp() public {
        inbox = new AfferentInbox();
        issuer = new CapabilityIssuer(address(inbox));
    }

    function testIssueCapability() public {
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
            planner: user,
            nonce: 1
        });

        bytes32 scaledLimitsHash = keccak256("limits");

        uint256 tokenId = issuer.issueCapability(intent, scaledLimitsHash);

        assertEq(tokenId, 1);
        assertTrue(issuer.isValid(tokenId));

        // Check token metadata
        Types.TokenMeta memory meta = issuer.getTokenMeta(tokenId);
        assertEq(meta.executorId, 42);
        assertEq(meta.actionId, intent.actionId);
        assertEq(meta.scaledLimitsHash, scaledLimitsHash);
        assertEq(meta.issuer, address(this));

        // Check active tokens
        uint256[] memory active = issuer.activeTokensOf(42);
        assertEq(active.length, 1);
        assertEq(active[0], tokenId);
    }

    function testRevokeCapability() public {
        // Issue a token first
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
            planner: user,
            nonce: 1
        });

        uint256 tokenId = issuer.issueCapability(intent, keccak256("limits"));

        // Revoke it
        issuer.revoke(tokenId, 1);

        assertFalse(issuer.isValid(tokenId));

        // Check it's removed from active tokens
        uint256[] memory active = issuer.activeTokensOf(42);
        assertEq(active.length, 0);
    }

    function testExpiredIntent() public {
        Types.Intent memory intent = Types.Intent({
            executorId: 42,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: bytes32(0),
            notBefore: uint64(block.timestamp + 3600), // Future
            notAfter: uint64(block.timestamp + 7200),
            maxDurationMs: 1000,
            maxEnergyJ: 500,
            planner: user,
            nonce: 1
        });

        vm.expectRevert(CapabilityIssuer.IntentExpired.selector);
        issuer.issueCapability(intent, keccak256("limits"));
    }

    function testIsValidExpired() public {
        Types.Intent memory intent = Types.Intent({
            executorId: 42,
            actionId: keccak256("test_action"),
            params: "",
            envelopeHash: keccak256("envelope"),
            preStateRoot: bytes32(0),
            notBefore: uint64(block.timestamp),
            notAfter: uint64(block.timestamp + 1), // Expires quickly
            maxDurationMs: 1000,
            maxEnergyJ: 500,
            planner: user,
            nonce: 1
        });

        uint256 tokenId = issuer.issueCapability(intent, keccak256("limits"));

        // Fast forward time
        vm.warp(block.timestamp + 2);

        assertFalse(issuer.isValid(tokenId));
    }
}
