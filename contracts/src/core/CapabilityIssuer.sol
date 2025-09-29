// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Events.sol";
import "./Types.sol";
import "./Interfaces.sol";
import "./GeneratedTypes.sol";

/// @title Capability Token Issuer
/// @notice Issues and manages revocable capability tokens (ERC721-like semantics)
contract CapabilityIssuer is Events {
    using Types for Types.Intent;
    using Types for Types.TokenMeta;

    /// @notice Mapping from token ID to token metadata
    mapping(uint256 => Types.TokenMeta) public tokenMeta;

    /// @notice Mapping from executor ID to list of active token IDs
    mapping(uint256 => uint256[]) public activeTokens;

    /// @notice Next token ID to mint
    uint256 public nextTokenId = 1;

    /// @notice Contract owner
    address public owner;

    /// @notice AfferentInbox contract for state root verification
    address public afferentInbox;

    /// @notice ReflexArc contract authorized to revoke tokens
    address public reflexArc;

    /// @notice VagalBrake contract for safety validation
    address public vagalBrake;

    /// @notice Constructor
    /// @param _afferentInbox Address of the AfferentInbox contract
    /// @param _vagalBrake Address of the VagalBrake contract
    constructor(address _afferentInbox, address _vagalBrake) {
        owner = msg.sender;
        afferentInbox = _afferentInbox;
        vagalBrake = _vagalBrake;
    }

    /// @notice Issue a capability token for an intent
    /// @param intent The intent to issue a capability for
    /// @param scaledLimitsHash Hash of the scaled limits from VagalBrake
    /// @return tokenId The issued token ID
    function issueCapability(
        Types.Intent calldata intent,
        bytes32 scaledLimitsHash
    ) external returns (uint256 tokenId) {
        // Validate intent timing
        if (block.timestamp < intent.notBefore || block.timestamp > intent.notAfter) {
            revert IntentExpired();
        }

        // ER1: Validate that scaledLimitsHash comes from VagalBrake
        (bytes32 expectedScaledLimitsHash, bool brakeAllowed) = IVagalBrake(vagalBrake).previewBrake(intent);
        if (!brakeAllowed) {
            revert ANSBlocked("VagalBrake validation failed");
        }
        if (scaledLimitsHash != expectedScaledLimitsHash) {
            revert InvalidInput("scaledLimitsHash mismatch");
        }

        // ER6: Validate pre-state root against AfferentInbox
        bytes32 latestStateRoot = IAfferentInbox(afferentInbox).latestStateRoot(intent.executorId);
        if (intent.preStateRoot != latestStateRoot) {
            revert StateMismatch(intent.preStateRoot, latestStateRoot);
        }

        // ER4: Validate CBOR hashes for params and preStateRoot (placeholder - full implementation in T-5)
        // For now, just ensure hashes are provided and non-zero
        if (scaledLimitsHash == bytes32(0)) {
            revert InvalidInput("scaledLimitsHash cannot be zero");
        }

        // TODO: Check nonce uniqueness per planner
        // For MVP, we skip this check

        // Mint new token
        tokenId = nextTokenId++;
        tokenMeta[tokenId] = Types.TokenMeta({
            executorId: intent.executorId,
            actionId: intent.actionId,
            scaledLimitsHash: scaledLimitsHash,
            issuedAt: uint64(block.timestamp),
            expiresAt: intent.notAfter,
            revoked: false,
            issuer: msg.sender
        });

        // Add to active tokens list
        activeTokens[intent.executorId].push(tokenId);

        // ER4/I24: Include dual hashes in event
        emit CapabilityIssued(
            tokenId,
            intent.executorId,
            intent.planner,
            intent.actionId,
            intent.notAfter,
            bytes32(0), // paramsHashSha256 - placeholder for T-5 full implementation
            bytes32(0), // paramsHashKeccak - placeholder for T-5 full implementation
            intent.preStateRoot, // preStateRootSha256 - simplified for now
            intent.preStateRoot  // preStateRootKeccak - simplified for now
        );

        return tokenId;
    }

    /// @notice Revoke a capability token
    /// @param tokenId The token ID to revoke
    /// @param reason The revocation reason code
    function revoke(uint256 tokenId, uint8 reason) external {
        require(msg.sender == owner || msg.sender == reflexArc, "Only owner or reflex arc can revoke tokens");

        Types.TokenMeta storage meta = tokenMeta[tokenId];
        require(meta.issuedAt != 0, "Token does not exist");
        require(!meta.revoked, "Token already revoked");

        meta.revoked = true;

        // Remove from active tokens list
        _removeFromActiveTokens(meta.executorId, tokenId);

        emit CapabilityRevoked(tokenId, reason);
    }

    /// @notice Set the ReflexArc contract address
    /// @param _reflexArc The ReflexArc contract address
    function setReflexArc(address _reflexArc) external {
        require(msg.sender == owner, "Only owner can set reflex arc");
        reflexArc = _reflexArc;
    }

    /// @notice Set the VagalBrake contract address
    /// @param _vagalBrake The VagalBrake contract address
    function setVagalBrake(address _vagalBrake) external {
        require(msg.sender == owner, "Only owner can set vagal brake");
        vagalBrake = _vagalBrake;
    }

    /// @notice Check if a token is valid (not expired, not revoked)
    /// @param tokenId The token ID to check
    /// @return True if the token is valid
    function isValid(uint256 tokenId) external view returns (bool) {
        Types.TokenMeta storage meta = tokenMeta[tokenId];
        if (meta.issuedAt == 0) return false; // Token doesn't exist
        if (meta.revoked) return false; // Token revoked
        if (block.timestamp > meta.expiresAt) return false; // Token expired
        return true;
    }

    /// @notice Get active tokens for an executor
    /// @param executorId The executor ID
    /// @return Array of active token IDs
    function activeTokensOf(uint256 executorId) external view returns (uint256[] memory) {
        return activeTokens[executorId];
    }

    /// @notice Get token metadata
    /// @param tokenId The token ID
    /// @return The token metadata
    function getTokenMeta(uint256 tokenId) external view returns (Types.TokenMeta memory) {
        return tokenMeta[tokenId];
    }

    /// @notice Internal function to remove token from active tokens list
    /// @param executorId The executor ID
    /// @param tokenId The token ID to remove
    function _removeFromActiveTokens(uint256 executorId, uint256 tokenId) internal {
        uint256[] storage tokens = activeTokens[executorId];
        for (uint256 i = 0; i < tokens.length; i++) {
            if (tokens[i] == tokenId) {
                tokens[i] = tokens[tokens.length - 1];
                tokens.pop();
                break;
            }
        }
    }
}
