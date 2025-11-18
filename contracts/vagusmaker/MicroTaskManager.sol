// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ANSStateManager} from "../../src/core/ANSStateManager.sol";
import {CapabilityIssuer} from "../../src/core/CapabilityIssuer.sol";

/// @title Micro Task Manager for VagusMaker
/// @notice Manages Hanoi Tower micro-tasks with state management
/// @dev Inherits ANSStateManager for autonomic state management
contract MicroTaskManager is ANSStateManager {
    /// @notice Structure representing a Hanoi Tower task
    struct Task {
        bytes initialState;     // Hanoi state: [[20..1],[],[]] encoded as bytes
        uint256 totalReward;    // Total USDC reward in wei
        uint256 currentStep;    // Current step in the solution
        bytes32 currentStateRoot; // IPFS Merkle root of current state
        uint8 k;                // Leading vote threshold (k-ahead-by consensus)
        address creator;        // Task creator address
        bool completed;         // Whether task is completed
    }

    /// @notice Mapping of task ID to task data
    mapping(uint256 => Task) public tasks;

    /// @notice Counter for task IDs
    uint256 public taskCounter;

    /// @notice Event emitted when a new task is created
    event TaskCreated(uint256 indexed taskId, address indexed creator, uint8 k);

    /// @notice Event emitted when a step is completed
    event StepCompleted(uint256 indexed taskId, uint256 step, bytes move);

    /// @notice Create a new Hanoi Tower task
    /// @param initialState Encoded initial state of Hanoi towers
    /// @param totalReward Total reward for completing the task
    /// @param k Leading vote threshold for consensus
    /// @return taskId The ID of the created task
    function createTask(
        bytes calldata initialState,
        uint256 totalReward,
        uint8 k
    ) external payable returns (uint256 taskId) {
        taskId = ++taskCounter;

        tasks[taskId] = Task({
            initialState: initialState,
            totalReward: totalReward,
            currentStep: 0,
            currentStateRoot: keccak256(initialState),
            k: k,
            creator: msg.sender,
            completed: false
        });

        emit TaskCreated(taskId, msg.sender, k);
    }

    /// @notice Get task information
    /// @param taskId The task ID to query
    /// @return task The task structure
    function getTask(uint256 taskId) external view returns (Task memory) {
        return tasks[taskId];
    }

    /// @notice Mark task as completed
    /// @param taskId The task ID to complete
    function completeTask(uint256 taskId) external {
        Task storage task = tasks[taskId];
        require(!task.completed, "Task already completed");
        require(task.creator == msg.sender, "Only creator can complete task");

        task.completed = true;
    }

    /// @notice Update task state after consensus move
    /// @param taskId The task ID
    /// @param newStateRoot New state root after move
    /// @param move The move data
    function updateTaskState(uint256 taskId, bytes32 newStateRoot, bytes calldata move) external {
        Task storage task = tasks[taskId];
        require(!task.completed, "Task already completed");

        task.currentStep++;
        task.currentStateRoot = newStateRoot;

        emit StepCompleted(taskId, task.currentStep, move);
    }

    /// @notice Check if a task exists
    /// @param taskId The task ID to check
    /// @return exists Whether the task exists
    function taskExists(uint256 taskId) external view returns (bool) {
        return tasks[taskId].creator != address(0);
    }
}
