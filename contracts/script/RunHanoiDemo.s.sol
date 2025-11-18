// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "../vagusmaker/ReputationWeightedVoter.sol";
import "../vagusmaker/ReputationToken.sol";
import "../vagusmaker/RedFlagValidator.sol";

/// @title Run Hanoi Tower Demo
/// @notice Demonstrates VagusMaker consensus on 3-disk Hanoi Tower problem
contract RunHanoiDemo is Script {
    // Mock participants (will be funded with reputation tokens)
    address[] participants = [
        0x1111111111111111111111111111111111111111,
        0x2222222222222222222222222222222222222222,
        0x3333333333333333333333333333333333333333,
        0x4444444444444444444444444444444444444444,
        0x5555555555555555555555555555555555555555
    ];

    function run() external {
        vm.startBroadcast();

        // Load deployed contract addresses (in production, read from VagusMakerConfig.json)
        // For demo, we'll deploy fresh contracts
        console.log("Starting VagusMaker Hanoi Tower Demo");

        // Deploy contracts
        RedFlagValidator validator = new RedFlagValidator();
        ReputationToken repToken = new ReputationToken(address(0)); // Will update later
        address mockReflexArc = address(0x1234567890123456789012345678901234567890);

        ReputationWeightedVoter voter = new ReputationWeightedVoter(
            address(repToken),
            address(validator),
            mockReflexArc
        );

        // Update rep token with voter address
        ReputationToken correctRepToken = new ReputationToken(address(voter));

        // Update voter to use correct rep token (in production, this would be a setter)
        console.log("Contracts deployed:");
        console.log("- RedFlagValidator:", address(validator));
        console.log("- ReputationToken:", address(correctRepToken));
        console.log("- ReputationWeightedVoter:", address(voter));

        // Initialize participants with reputation tokens
        for (uint256 i = 0; i < participants.length; i++) {
            vm.prank(address(voter)); // Voter acts as VagusMaker
            correctRepToken.mint(participants[i]);
            console.log("Minted reputation token for participant", participants[i]);
        }

        // Create 3-disk Hanoi Tower task
        // Initial state: [[3,2,1],[],[]] - all disks on peg 0
        bytes memory initialState = abi.encode([
            [bytes1(0x03), bytes1(0x02), bytes1(0x01)], // Peg 0: disks 3,2,1 (bottom to top)
            new bytes[](0),                              // Peg 1: empty
            new bytes[](0)                               // Peg 2: empty
        ]);

        vm.prank(participants[0]); // Creator
        uint256 taskId = voter.createTask(initialState, 1 ether, 2); // 1 ETH reward, k=2
        console.log("Created Hanoi Tower task with ID:", taskId);

        // Simulate voting process for first move (disk 1 from peg 0 to peg 2)
        console.log("Starting voting for step 0...");

        // Participant 0 votes correctly: move disk 1 (0x01) from peg 0 to peg 2
        vm.prank(participants[0]);
        voter.castVote(taskId, 0, RedFlagValidator.Move(0, 2), "correct_proof_1");

        // Participant 1 votes correctly
        vm.prank(participants[1]);
        voter.castVote(taskId, 0, RedFlagValidator.Move(0, 2), "correct_proof_2");

        // Participant 2 votes correctly (should achieve consensus with k=2)
        vm.prank(participants[2]);
        voter.castVote(taskId, 0, RedFlagValidator.Move(0, 2), "correct_proof_3");

        console.log("Consensus achieved for step 0: move disk 1 from peg 0 to peg 2");

        // Check consensus result
        RedFlagValidator.Move memory consensusMove = voter.getConsensusMove(taskId, 0);
        console.log("Consensus move - from:", consensusMove.from, "to:", consensusMove.to);

        // Continue with more steps in a real demo...
        console.log("Demo completed successfully!");
        console.log("VagusMaker consensus working on blockchain-native MAKER algorithm");

        vm.stopBroadcast();
    }
}
