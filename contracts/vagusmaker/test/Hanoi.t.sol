// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../ReputationToken.sol";
import "../MicroTaskManager.sol";
import "../RedFlagValidator.sol";
import "../ReputationWeightedVoter.sol";

contract HanoiTest is Test {
    ReputationToken repToken;
    MicroTaskManager taskManager;
    RedFlagValidator validator;
    ReputationWeightedVoter voter;

    address[] participants;
    address mockReflexArc = address(0x1234567890123456789012345678901234567890);

    function setUp() public {
        // Create participants
        participants = new address[](5);
        for (uint256 i = 0; i < 5; i++) {
            participants[i] = address(uint160(0x1111 + i));
        }

        // Deploy contracts
        validator = new RedFlagValidator();
        taskManager = new MicroTaskManager();
        repToken = new ReputationToken(address(0)); // Will set voter address later

        voter = new ReputationWeightedVoter(
            address(repToken),
            address(validator),
            mockReflexArc
        );

        // Mint reputation tokens for participants
        for (uint256 i = 0; i < participants.length; i++) {
            vm.prank(address(voter)); // Voter acts as VagusMaker
            repToken.mint(participants[i]);
        }
    }

    function testInitialSetup() public {
        // Check reputation tokens were minted
        for (uint256 i = 0; i < participants.length; i++) {
            assertEq(repToken.balanceOf(participants[i]), uint256(uint160(participants[i])));
            assertEq(repToken.reputation(participants[i]), 100);
        }
    }

    function testCreateHanoiTask() public {
        // Create 3-disk Hanoi Tower task
        bytes memory initialState = abi.encode([
            [bytes1(0x03), bytes1(0x02), bytes1(0x01)], // Peg 0: disks 3,2,1
            new bytes[](0),                              // Peg 1: empty
            new bytes[](0)                               // Peg 2: empty
        ]);

        vm.prank(participants[0]);
        uint256 taskId = voter.createTask(initialState, 1 ether, 2);

        // Verify task was created
        assertTrue(voter.taskExists(taskId));

        MicroTaskManager.Task memory task = voter.getTask(taskId);
        assertEq(task.creator, participants[0]);
        assertEq(task.totalReward, 1 ether);
        assertEq(task.currentStep, 0);
        assertEq(task.k, 2);
        assertFalse(task.completed);
    }

    function testRedFlagValidation() public {
        // Test valid moves
        bytes memory state = abi.encode([
            [bytes1(0x03), bytes1(0x02), bytes1(0x01)], // Peg 0: disks 3,2,1
            new bytes[](0),                              // Peg 1: empty
            new bytes[](0)                               // Peg 2: empty
        ]);

        // Valid: move disk 1 from peg 0 to peg 1
        (bool valid, string memory reason) = validator.validate(
            state,
            RedFlagValidator.Move(0, 1),
            "valid_proof"
        );
        assertTrue(valid);
        assertEq(reason, "");

        // Invalid: move from empty peg
        (valid, reason) = validator.validate(
            state,
            RedFlagValidator.Move(1, 2),
            "invalid_proof"
        );
        assertFalse(valid);
        assertEq(reason, "Source peg is empty");

        // Invalid: move to same peg
        (valid, reason) = validator.validate(
            state,
            RedFlagValidator.Move(0, 0),
            "invalid_proof"
        );
        assertFalse(valid);
        assertEq(reason, "Cannot move to same peg");

        // Invalid: larger disk on smaller disk
        bytes memory state2 = abi.encode([
            [bytes1(0x03), bytes1(0x02)], // Peg 0: disks 3,2
            [bytes1(0x01)],               // Peg 1: disk 1
            new bytes[](0)                // Peg 2: empty
        ]);

        (valid, reason) = validator.validate(
            state2,
            RedFlagValidator.Move(0, 1), // Try to place disk 2 on disk 1
            "invalid_proof"
        );
        assertFalse(valid);
        assertEq(reason, "Cannot place larger disk on smaller disk");
    }

    function testApplyMove() public {
        bytes memory initialState = abi.encode([
            [bytes1(0x03), bytes1(0x02), bytes1(0x01)], // Peg 0: disks 3,2,1
            new bytes[](0),                              // Peg 1: empty
            new bytes[](0)                               // Peg 2: empty
        ]);

        // Apply move: disk 1 from peg 0 to peg 2
        bytes memory newState = validator.applyMove(
            initialState,
            RedFlagValidator.Move(0, 2)
        );

        // Verify new state
        bytes[][] memory pegs = abi.decode(newState, (bytes[][]));
        assertEq(pegs[0].length, 2); // Peg 0 should have 2 disks
        assertEq(pegs[1].length, 0); // Peg 1 should be empty
        assertEq(pegs[2].length, 1); // Peg 2 should have 1 disk

        assertEq(uint8(pegs[0][0]), 3); // Peg 0: disk 3
        assertEq(uint8(pegs[0][1]), 2); // Peg 0: disk 2
        assertEq(uint8(pegs[2][0]), 1); // Peg 2: disk 1
    }

    function testIsSolved() public {
        // Unsolved state
        bytes memory unsolvedState = abi.encode([
            [bytes1(0x03), bytes1(0x02), bytes1(0x01)], // Peg 0: disks 3,2,1
            new bytes[](0),                              // Peg 1: empty
            new bytes[](0)                               // Peg 2: empty
        ]);

        bool solved = validator.isSolved(unsolvedState, 2, 3);
        assertFalse(solved);

        // Solved state: all disks on peg 2
        bytes memory solvedState = abi.encode([
            new bytes[](0),                              // Peg 0: empty
            new bytes[](0),                              // Peg 1: empty
            [bytes1(0x01), bytes1(0x02), bytes1(0x03)]   // Peg 2: disks 1,2,3 (top to bottom)
        ]);

        solved = validator.isSolved(solvedState, 2, 3);
        assertTrue(solved);
    }

    function testConsensusVoting() public {
        // Create task
        bytes memory initialState = abi.encode([
            [bytes1(0x03), bytes1(0x02), bytes1(0x01)], // Peg 0: disks 3,2,1
            new bytes[](0),                              // Peg 1: empty
            new bytes[](0)                               // Peg 2: empty
        ]);

        vm.prank(participants[0]);
        uint256 taskId = voter.createTask(initialState, 1 ether, 2);

        // Cast votes - correct move: disk 1 from peg 0 to peg 2
        vm.prank(participants[0]);
        voter.castVote(taskId, 0, RedFlagValidator.Move(0, 2), "proof1");

        vm.prank(participants[1]);
        voter.castVote(taskId, 0, RedFlagValidator.Move(0, 2), "proof2");

        // Third vote should achieve consensus (k=2)
        vm.prank(participants[2]);
        voter.castVote(taskId, 0, RedFlagValidator.Move(0, 2), "proof3");

        // Check consensus was achieved
        RedFlagValidator.Move memory consensusMove = voter.getConsensusMove(taskId, 0);
        assertEq(consensusMove.from, 0);
        assertEq(consensusMove.to, 2);

        // Check task was updated
        MicroTaskManager.Task memory task = voter.getTask(taskId);
        assertEq(task.currentStep, 1);
    }

    function testInvalidVotePenalty() public {
        // Create task
        bytes memory initialState = abi.encode([
            [bytes1(0x03), bytes1(0x02), bytes1(0x01)], // Peg 0: disks 3,2,1
            new bytes[](0),                              // Peg 1: empty
            new bytes[](0)                               // Peg 2: empty
        ]);

        vm.prank(participants[0]);
        uint256 taskId = voter.createTask(initialState, 1 ether, 2);

        // Cast invalid vote (move from empty peg)
        uint256 initialRep = repToken.reputation(participants[0]);
        vm.prank(participants[0]);
        voter.castVote(taskId, 0, RedFlagValidator.Move(1, 2), "invalid_proof");

        // Check reputation was penalized
        assertEq(repToken.reputation(participants[0]), initialRep - 100);
    }

    function testReputationWeightedVoting() public {
        // Create task
        bytes memory initialState = abi.encode([
            [bytes1(0x03), bytes1(0x02), bytes1(0x01)], // Peg 0: disks 3,2,1
            new bytes[](0),                              // Peg 1: empty
            new bytes[](0)                               // Peg 2: empty
        ]);

        vm.prank(participants[0]);
        uint256 taskId = voter.createTask(initialState, 1 ether, 3); // k=3 for stricter consensus

        // Give participant 0 higher reputation
        vm.prank(address(voter));
        repToken.updateReputation(participants[0], 900); // Total: 1000

        // Participant 0 votes first (high weight)
        vm.prank(participants[0]);
        voter.castVote(taskId, 0, RedFlagValidator.Move(0, 2), "proof_high_weight");

        // Multiple lower weight participants vote for different move
        for (uint256 i = 1; i < 4; i++) {
            vm.prank(participants[i]);
            voter.castVote(taskId, 0, RedFlagValidator.Move(0, 1), "proof_low_weight");
        }

        // Check that high-weight vote wins
        RedFlagValidator.Move memory consensusMove = voter.getConsensusMove(taskId, 0);
        assertEq(consensusMove.from, 0);
        assertEq(consensusMove.to, 2); // Should be the high-weight choice
    }
}
