// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title Red Flag Validator for Hanoi Tower Moves
/// @notice Validates proposed moves against Hanoi Tower rules and current state
contract RedFlagValidator {
    /// @notice Structure representing a Hanoi Tower move
    struct Move {
        uint8 from; // Source peg (0-2)
        uint8 to;   // Destination peg (0-2)
    }

    /// @notice Validate a proposed move against Hanoi Tower rules
    /// @param currentState Current state of the towers as bytes
    /// @param proposed Proposed move to validate
    /// @param proof Proof data (IPFS CID or JSON) - for future use
    /// @return valid Whether the move is valid
    /// @return reason Reason for invalidity (empty if valid)
    function validate(
        bytes calldata currentState,
        Move calldata proposed,
        bytes calldata proof
    ) external pure returns (bool valid, string memory reason) {
        // 1. Basic proof format check (simplified - in production would verify IPFS hash)
        if (proof.length < 20) {
            return (false, "Invalid proof format");
        }

        // 2. Parse current state - expect format: [[disks...],[disks...],[disks...]]
        bytes[][] memory pegs = abi.decode(currentState, (bytes[][]));

        // 3. Validate peg indices
        if (proposed.from >= 3 || proposed.to >= 3) {
            return (false, "Invalid peg index");
        }

        if (proposed.from == proposed.to) {
            return (false, "Cannot move to same peg");
        }

        // 4. Check source peg is not empty
        if (pegs[proposed.from].length == 0) {
            return (false, "Source peg is empty");
        }

        // 5. Get disk size from top of source peg
        uint8 diskSize = uint8(pegs[proposed.from][pegs[proposed.from].length - 1]);

        // 6. Check destination peg rules (cannot place larger disk on smaller disk)
        if (pegs[proposed.to].length > 0) {
            uint8 topDisk = uint8(pegs[proposed.to][pegs[proposed.to].length - 1]);
            if (diskSize >= topDisk) {
                return (false, "Cannot place larger disk on smaller disk");
            }
        }

        // 7. All validations passed
        return (true, "");
    }

    /// @notice Apply a validated move to create new state
    /// @param currentState Current state of the towers
    /// @param move Validated move to apply
    /// @return newState New state after applying the move
    function applyMove(
        bytes calldata currentState,
        Move calldata move
    ) external pure returns (bytes memory newState) {
        // Parse current state
        bytes[][] memory pegs = abi.decode(currentState, (bytes[][]));

        // Create new state array
        bytes[][] memory newPegs = new bytes[][](3);
        for (uint8 i = 0; i < 3; i++) {
            newPegs[i] = new bytes[](pegs[i].length);
            for (uint256 j = 0; j < pegs[i].length; j++) {
                newPegs[i][j] = pegs[i][j];
            }
        }

        // Apply the move: remove from source, add to destination
        uint8 diskSize = uint8(newPegs[move.from][newPegs[move.from].length - 1]);

        // Remove from source
        bytes[] memory sourcePeg = new bytes[](newPegs[move.from].length - 1);
        for (uint256 i = 0; i < sourcePeg.length; i++) {
            sourcePeg[i] = newPegs[move.from][i];
        }
        newPegs[move.from] = sourcePeg;

        // Add to destination
        bytes[] memory destPeg = new bytes[](newPegs[move.to].length + 1);
        for (uint256 i = 0; i < newPegs[move.to].length; i++) {
            destPeg[i] = newPegs[move.to][i];
        }
        destPeg[newPegs[move.to].length] = bytes1(diskSize);
        newPegs[move.to] = destPeg;

        // Encode new state
        return abi.encode(newPegs);
    }

    /// @notice Check if a state represents solved Hanoi Tower (all disks on target peg)
    /// @param state Current state to check
    /// @param targetPeg Target peg index (usually 2)
    /// @param numDisks Total number of disks
    /// @return solved Whether the puzzle is solved
    function isSolved(
        bytes calldata state,
        uint8 targetPeg,
        uint8 numDisks
    ) external pure returns (bool solved) {
        bytes[][] memory pegs = abi.decode(state, (bytes[][]));

        // Check if target peg has all disks
        if (pegs[targetPeg].length != numDisks) {
            return false;
        }

        // Check if other pegs are empty
        for (uint8 i = 0; i < 3; i++) {
            if (i != targetPeg && pegs[i].length > 0) {
                return false;
            }
        }

        // Check if disks are in correct order (largest at bottom)
        for (uint256 i = 0; i < numDisks; i++) {
            uint8 expectedSize = uint8(numDisks - i); // Largest at bottom
            uint8 actualSize = uint8(pegs[targetPeg][i]);
            if (actualSize != expectedSize) {
                return false;
            }
        }

        return true;
    }
}
