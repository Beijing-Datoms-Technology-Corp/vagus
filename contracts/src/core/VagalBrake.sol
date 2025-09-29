// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Events.sol";
import "./Types.sol";
import "./Interfaces.sol";

/// @title Vagal Brake
/// @notice Applies dynamic scaling to intents based on ANS state
contract VagalBrake is Events {
    /// @notice ANS State Manager contract
    address public ansStateManager;

    /// @notice Capability Issuer contract
    address public capabilityIssuer;

    /// @notice Contract owner
    address public owner;

    /// @notice Custom error for blocked action
    error ANSBlocked(string reason);

    /// @notice Custom error for limit exceeded
    error ANSLimitExceeded(string field, uint256 requested, uint256 allowed);

    /// @notice Constructor
    /// @param _ansStateManager Address of the ANS State Manager
    /// @param _capabilityIssuer Address of the Capability Issuer
    constructor(address _ansStateManager, address _capabilityIssuer) {
        owner = msg.sender;
        ansStateManager = _ansStateManager;
        capabilityIssuer = _capabilityIssuer;
    }

    /// @notice Issue capability with vagal brake applied
    /// @param intent The intent to process
    /// @return tokenId The issued capability token ID
    function issueWithBrake(Types.Intent calldata intent) external returns (uint256 tokenId) {
        // Get guard information from ANS State Manager
        (uint256 scalingFactor, bool allowed) = IANSStateManager(ansStateManager).guardFor(intent.executorId, intent.actionId);

        // Check if action is allowed
        if (!allowed) {
            revert ANSBlocked("ANS:blocked");
        }

        // Scale brakeable parameters and validate limits
        bytes32 scaledLimitsHash = _scaleAndValidateIntent(intent, scalingFactor);

        // Issue capability through the issuer
        tokenId = ICapabilityIssuer(capabilityIssuer).issueCapability(intent, scaledLimitsHash);

        return tokenId;
    }

    /// @notice Preview scaled limits without issuing
    /// @param intent The intent to preview
    /// @return scaledLimitsHash Hash of the scaled limits
    /// @return allowed Whether the intent would be allowed
    function previewBrake(Types.Intent calldata intent) external view returns (bytes32 scaledLimitsHash, bool allowed) {
        // Get guard information from ANS State Manager
        (uint256 scalingFactor, bool actionAllowed) = IANSStateManager(ansStateManager).guardFor(intent.executorId, intent.actionId);

        allowed = actionAllowed;
        if (!allowed) {
            return (bytes32(0), false);
        }

        // Scale brakeable parameters and validate limits
        scaledLimitsHash = _scaleAndValidateIntentPreview(intent, scalingFactor);

        return (scaledLimitsHash, true);
    }

    /// @notice Internal function to scale and validate intent parameters
    /// @param intent The intent to process
    /// @param scalingFactor The scaling factor from ANS
    /// @return scaledLimitsHash Hash of scaled limits
    function _scaleAndValidateIntent(Types.Intent calldata intent, uint256 scalingFactor) internal pure returns (bytes32) {
        // For MVP, we implement scaling for mechanical arm actions
        // In a full implementation, this would parse intent.params and scale brakeable fields

        // TODO: Parse intent.params JSON/ABI encoded data and scale brakeable fields
        // For now, we assume the intent parameters are already validated and just hash them

        // Apply scaling to brakeable limits
        uint256 scaledMaxDuration = (intent.maxDurationMs * scalingFactor) / 10000;
        uint256 scaledMaxEnergy = (intent.maxEnergyJ * scalingFactor) / 10000;

        // Validate against absolute limits
        if (scaledMaxDuration > Types.MAX_DURATION_MS) {
            revert ANSLimitExceeded("maxDurationMs", scaledMaxDuration, Types.MAX_DURATION_MS);
        }
        if (scaledMaxEnergy > Types.MAX_ENERGY_J) {
            revert ANSLimitExceeded("maxEnergyJ", scaledMaxEnergy, Types.MAX_ENERGY_J);
        }

        // Create scaled limits hash (simplified for MVP)
        return keccak256(abi.encodePacked(
            intent.actionId,
            scaledMaxDuration,
            scaledMaxEnergy,
            scalingFactor
        ));
    }

    /// @notice Internal function to preview scaled limits (view version)
    /// @param intent The intent to process
    /// @param scalingFactor The scaling factor from ANS
    /// @return scaledLimitsHash Hash of scaled limits
    function _scaleAndValidateIntentPreview(Types.Intent calldata intent, uint256 scalingFactor) internal pure returns (bytes32) {
        // Apply scaling to brakeable limits
        uint256 scaledMaxDuration = (intent.maxDurationMs * scalingFactor) / 10000;
        uint256 scaledMaxEnergy = (intent.maxEnergyJ * scalingFactor) / 10000;

        // For preview, we don't revert on limits - just return the hash
        return keccak256(abi.encodePacked(
            intent.actionId,
            scaledMaxDuration,
            scaledMaxEnergy,
            scalingFactor
        ));
    }
}
