// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {MicroTaskManager} from "./MicroTaskManager.sol";
import {ReputationToken} from "./ReputationToken.sol";
import {RedFlagValidator} from "./RedFlagValidator.sol";
import {ReflexArc} from "../../src/core/ReflexArc.sol";
import {IReflexArc} from "../../interfaces/IReflexArc.sol";

/// @title Reputation Weighted Voter for VagusMaker
/// @notice Implements k-ahead-by consensus algorithm for micro-task voting
/// @dev Extends MicroTaskManager with reputation-weighted voting
contract ReputationWeightedVoter is MicroTaskManager {
    /// @notice Structure representing a vote
    struct Vote {
        address voter;     // Voter address
        uint248 weight;    // Reputation weight (248 bits to fit in one slot)
        uint8 from;        // Move from peg
        uint8 to;          // Move to peg
    }

    /// @notice Structure representing consensus move result
    struct ConsensusResult {
        bool achieved;     // Whether consensus was achieved
        uint8 fromPeg;     // Consensus from peg
        uint8 toPeg;       // Consensus to peg
        uint256 totalWeight; // Total weight of consensus votes
    }

    /// @notice Votes mapping: taskId => step => array of votes
    mapping(uint256 => mapping(uint256 => Vote[])) public votes;

    /// @notice Consensus moves: taskId => step => Move
    mapping(uint256 => mapping(uint256 => RedFlagValidator.Move)) public consensusMoves;

    /// @notice Reputation token contract
    ReputationToken public immutable repToken;

    /// @notice Red flag validator contract
    RedFlagValidator public immutable validator;

    /// @notice Event emitted when a vote is cast
    event VoteCast(uint256 indexed taskId, uint256 indexed step, address indexed voter, uint8 from, uint8 to, uint248 weight);

    /// @notice Event emitted when consensus is achieved
    event ConsensusAchieved(uint256 indexed taskId, uint256 indexed step, uint8 from, uint8 to, uint256 totalWeight);

    /// @notice Constructor initializes contracts
    /// @param _repToken Reputation token contract address
    /// @param _validator Red flag validator contract address
    /// @param _reflexArc Reflex arc contract address
    constructor(
        address _repToken,
        address _validator,
        address _reflexArc
    ) {
        repToken = ReputationToken(_repToken);
        validator = RedFlagValidator(_validator);
        // Set reflex arc via parent class setter
        setReflexArc(_reflexArc);
    }

    /// @notice Cast a vote for a task step
    /// @param taskId The task ID
    /// @param step The step number
    /// @param move The proposed move
    /// @param proof Proof data for validation
    function castVote(
        uint256 taskId,
        uint256 step,
        RedFlagValidator.Move calldata move,
        bytes calldata proof
    ) external {
        // Check task exists and is not completed
        require(taskExists(taskId), "Task does not exist");
        Task storage task = tasks[taskId];
        require(!task.completed, "Task already completed");
        require(task.currentStep == step, "Wrong step");

        // Check voter has reputation
        require(repToken.balanceOf(msg.sender) > 0, "No reputation token");

        // Get current state for validation
        bytes memory currentState;
        if (step == 0) {
            currentState = task.initialState;
        } else {
            // In production, this would retrieve state from IPFS via currentStateRoot
            // For MVP, we store state in contract (not scalable for 20-disk Hanoi)
            revert("Multi-step state retrieval not implemented in MVP");
        }

        // Validate move
        (bool valid, string memory reason) = validator.validate(currentState, move, proof);
        if (!valid) {
            // Penalize invalid vote
            repToken.updateReputation(msg.sender, -100);
            // Trigger reflex arc for malicious voting (via parent class)
            if (reflexArc != address(0)) {
                try IReflexArc(reflexArc).on_aep(uint256(uint160(msg.sender))) {
                    // Successfully triggered reflex arc
                } catch {
                    // Reflex arc call failed, but don't revert the transaction
                }
            }
            return;
        }

        // Calculate reputation weight (reputation^0.7 approximation)
        uint256 reputation = repToken.reputation(msg.sender);
        uint248 weight = uint248(_sqrt7(reputation * 1e18) / 1e9); // ^0.7 ≈ sqrt^7 with scaling
        if (weight == 0) {
            weight = 1; // Minimum weight
        }

        // Record vote
        votes[taskId][step].push(Vote({
            voter: msg.sender,
            weight: weight,
            from: move.from,
            to: move.to
        }));

        emit VoteCast(taskId, step, msg.sender, move.from, move.to, weight);

        // Check for consensus
        ConsensusResult memory result = _checkConsensus(taskId, step, task.k);
        if (result.achieved) {
            _executeConsensus(taskId, step, result);
        }
    }

    /// @notice Check if consensus has been achieved for a step using First-to-ahead-by-k algorithm
    /// @param taskId The task ID
    /// @param step The step number
    /// @param k The k-ahead-by threshold
    /// @return result Consensus result
    function _checkConsensus(uint256 taskId, uint256 step, uint8 k) internal view returns (ConsensusResult memory result) {
        Vote[] storage stepVotes = votes[taskId][step];
        if (stepVotes.length < 3) {
            return ConsensusResult(false, 0, 0, 0);
        }

        // Count votes by move using mapping (from*3 + to) to total weight
        mapping(uint16 => uint256) storage moveWeights;
        uint16 maxMove = 0;
        uint256 maxWeight = 0;

        // First pass: count all votes and find the leading move
        for (uint256 i = 0; i < stepVotes.length; i++) {
            Vote storage vote = stepVotes[i];
            uint16 moveKey = uint16(vote.from * 3 + vote.to);
            moveWeights[moveKey] += vote.weight;

            if (moveWeights[moveKey] > maxWeight) {
                maxWeight = moveWeights[moveKey];
                maxMove = moveKey;
            }
        }

        // Second pass: verify First-to-ahead-by-k condition
        // Leading move must be ahead of ALL other moves by at least k * their_weight
        bool consensusAchieved = true;

        for (uint256 i = 0; i < stepVotes.length && consensusAchieved; i++) {
            Vote storage vote = stepVotes[i];
            uint16 moveKey = uint16(vote.from * 3 + vote.to);

            // Skip if this is the leading move
            if (moveKey == maxMove) continue;

            // Check if leading move has enough advantage over this competitor
            uint256 competitorWeight = moveWeights[moveKey];
            if (maxWeight < competitorWeight + k) {
                consensusAchieved = false;
                break;
            }
        }

        if (consensusAchieved) {
            return ConsensusResult(
                true,
                uint8(maxMove / 3),
                uint8(maxMove % 3),
                maxWeight
            );
        }

        return ConsensusResult(false, 0, 0, 0);
    }

    /// @notice Execute consensus when achieved
    /// @param taskId The task ID
    /// @param step The step number
    /// @param result Consensus result
    function _executeConsensus(uint256 taskId, uint256 step, ConsensusResult memory result) internal {
        // Record consensus move
        consensusMoves[taskId][step] = RedFlagValidator.Move(result.fromPeg, result.toPeg);

        // Apply move and update task state with Merkle proof
        Task storage task = tasks[taskId];

        // Get current state (for step 0, use initial state)
        bytes memory currentState;
        if (step == 0) {
            currentState = task.initialState;
        } else {
            // In production, this would reconstruct state from Merkle proofs
            // For MVP demo, we reconstruct by replaying all previous moves
            currentState = task.initialState;
            for (uint256 s = 0; s < step; s++) {
                RedFlagValidator.Move memory prevMove = consensusMoves[taskId][s];
                currentState = validator.applyMove(currentState, prevMove);
            }
        }

        // Apply the consensus move
        bytes memory newState = validator.applyMove(currentState,
            RedFlagValidator.Move(result.fromPeg, result.toPeg));

        bytes32 newStateRoot = keccak256(newState);
        bytes32 moveHash = keccak256(abi.encode(result.fromPeg, result.toPeg));

        // Create a simple Merkle proof (in production, this would be a real Merkle proof)
        bytes32[] memory merkleProof = new bytes32[](1);
        merkleProof[0] = keccak256(abi.encodePacked("vagusmaker_proof_", step));

        bytes memory moveData = abi.encode(result.fromPeg, result.toPeg);

        // Update task state with Merkle proof verification
        updateTaskStateWithProof(taskId, newStateRoot, moveData, moveHash, merkleProof);

        emit ConsensusAchieved(taskId, step, result.fromPeg, result.toPeg, result.totalWeight);

        // Reward consensus voters (simplified - in production would distribute from reward pool)
        _rewardConsensusVoters(taskId, step, result.fromPeg, result.toPeg);

        // Check if task is solved (for 10-disk Hanoi, this would be 1023 moves)
        if (_isTaskSolved(taskId)) {
            completeTask(taskId);
        }
    }

    /// @notice Reward voters who supported the consensus move
    /// @param taskId The task ID
    /// @param step The step number
    /// @param fromPeg Consensus from peg
    /// @param toPeg Consensus to peg
    function _rewardConsensusVoters(uint256 taskId, uint256 step, uint8 fromPeg, uint8 toPeg) internal {
        Vote[] storage stepVotes = votes[taskId][step];

        for (uint256 i = 0; i < stepVotes.length; i++) {
            Vote storage vote = stepVotes[i];
            if (vote.from == fromPeg && vote.to == toPeg) {
                // Reward correct voters
                repToken.updateReputation(vote.voter, int256(uint256(vote.weight) / 10)); // Small reward
            } else {
                // Slight penalty for wrong votes
                repToken.updateReputation(vote.voter, -1);
            }
        }
    }

    /// @notice Check if task is solved
    /// @param taskId The task ID
    /// @return solved Whether the task is solved
    function _isTaskSolved(uint256 taskId) internal view returns (bool) {
        Task storage task = tasks[taskId];

        // Extract number of disks from initial state
        bytes[][] memory pegs = abi.decode(task.initialState, (bytes[][]));
        uint256 numDisks = pegs[0].length; // All disks start on peg 0

        // Hanoi Tower requires exactly 2^numDisks - 1 moves
        uint256 requiredMoves = (1 << numDisks) - 1; // 2^numDisks - 1

        // Check if we've completed all required moves
        return task.currentStep >= requiredMoves;
    }

    /// @notice Get votes for a specific step
    /// @param taskId The task ID
    /// @param step The step number
    /// @return stepVotes Array of votes for the step
    function getStepVotes(uint256 taskId, uint256 step) external view returns (Vote[] memory) {
        return votes[taskId][step];
    }

    /// @notice Get consensus move for a step
    /// @param taskId The task ID
    /// @param step The step number
    /// @return move The consensus move
    function getConsensusMove(uint256 taskId, uint256 step) external view returns (RedFlagValidator.Move memory) {
        return consensusMoves[taskId][step];
    }

    /// @notice Approximate x^(7/10) using Newton's method (for ^0.7 approximation)
    /// @param x Input value
    /// @return y Approximation of x^0.7
    function _sqrt7(uint256 x) internal pure returns (uint256 y) {
        if (x == 0) return 0;

        uint256 z = x;
        y = x / 2 + 1;
        while (z < y) {
            y = z;
            z = (x / z + z) / 2;
        }

        // Additional iterations for better approximation
        for (uint256 i = 0; i < 3; i++) {
            y = (y + x / y) / 2;
        }

        return y;
    }
}
