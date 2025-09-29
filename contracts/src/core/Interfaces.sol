// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Types.sol";

/// @title Core Contract Interfaces
/// @notice Shared interfaces for Vagus contracts

/// @title ANS State Manager Interface
interface IANSStateManager {
    function guardFor(uint256 executorId, bytes32 actionId) external view returns (uint256 scalingFactor, bool allowed);
}

/// @title Capability Issuer Interface
interface ICapabilityIssuer {
    function activeTokensOf(uint256 executorId) external view returns (uint256[] memory);
    function isValid(uint256 tokenId) external view returns (bool);
    function revoke(uint256 tokenId, uint8 reason) external;
    function issueCapability(Types.Intent calldata intent, bytes32 scaledLimitsHash) external returns (uint256 tokenId);
}

/// @title AfferentInbox Interface
interface IAfferentInbox {
    function latestStateRoot(uint256 executorId) external view returns (bytes32);
}

/// @title VagalBrake Interface
interface IVagalBrake {
    function previewBrake(Types.Intent calldata intent) external view returns (bytes32 scaledLimitsHash, bool allowed);
}

/// @title ReflexArc Interface
interface IReflexArc {
    function on_state_change(uint256 executorId, uint8 newState) external;
}
