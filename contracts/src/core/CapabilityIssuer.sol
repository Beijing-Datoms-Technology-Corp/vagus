// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Events.sol";
import "./Types.sol";
import "../../interfaces/IANSStateManager.sol";
import "../../interfaces/ICapabilityIssuer.sol";
import "../../interfaces/IAfferentInbox.sol";
import "../../interfaces/IVagalBrake.sol";
import "../../interfaces/IReflexArc.sol";
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

    /// @notice Contract owner (governance controlled)
    address public owner;

    /// @notice AfferentInbox contract for state root verification
    address public afferentInbox;

    /// @notice VagalBrake contract for safety validation
    address public vagalBrake;

    /// @notice ANS State Manager contract
    address public ansStateManager;

    /// @notice ReflexArc contract authorized to revoke tokens
    address public reflexArc;

    /// @notice Rate limiter configuration
    struct RateLimitConfig {
        uint256 windowSize;    // Time window in seconds
        uint256 maxRequests;   // Maximum requests per window
        uint256 refillRate;    // Tokens refilled per second (for token bucket)
    }

    /// @notice Circuit breaker state
    enum CircuitState { CLOSED, OPEN, HALF_OPEN }
    struct CircuitBreaker {
        CircuitState state;
        uint256 failureCount;
        uint256 lastFailureTime;
        uint256 successCount;
        uint256 nextAttemptTime;
    }

    /// @notice Rate limiter state per (executorId, actionId) pair
    mapping(bytes32 => uint256[]) public rateLimitWindows; // Sliding window timestamps
    mapping(bytes32 => RateLimitConfig) public rateLimitConfigs;
    mapping(bytes32 => CircuitBreaker) public circuitBreakers;

    /// @notice Global rate limit configuration
    RateLimitConfig public globalRateLimit;
    uint256 public circuitBreakerThreshold = 5; // Failures before opening
    uint256 public circuitBreakerTimeout = 300; // 5 minutes timeout
    uint256 public circuitBreakerRecovery = 3;  // Successes needed in half-open

    /// @notice Constructor with dependency injection
    /// @param _afferentInbox Address of the AfferentInbox contract
    /// @param _vagalBrake Address of the VagalBrake contract
    /// @param _ansStateManager Address of the ANS State Manager contract
    /// @param _reflexArc Address of the ReflexArc contract
    constructor(
        address _afferentInbox,
        address _vagalBrake,
        address _ansStateManager,
        address _reflexArc
    ) {
        owner = msg.sender;
        afferentInbox = _afferentInbox;
        vagalBrake = _vagalBrake;
        ansStateManager = _ansStateManager;
        reflexArc = _reflexArc;

        // Set default rate limits (1000 requests per hour)
        globalRateLimit = RateLimitConfig({
            windowSize: 3600, // 1 hour
            maxRequests: 1000, // 1000 requests per hour
            refillRate: 0 // Not used in sliding window mode
        });
    }

    /// @notice Issue a capability token for an intent
    /// @param intent The intent to issue a capability for
    /// @param scaledLimitsHash Hash of the scaled limits from VagalBrake
    /// @return tokenId The issued token ID
    function issueCapability(
        Types.Intent calldata intent,
        bytes32 scaledLimitsHash
    ) external returns (uint256 tokenId) {
        // ER7: Check circuit breaker first
        bytes32 key = keccak256(abi.encodePacked(intent.executorId, intent.actionId));
        _checkCircuitBreaker(key);

        // ER7: Check rate limits (sliding window)
        _checkRateLimit(key);

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

        // Record circuit breaker success
        _recordCircuitSuccess(intent.executorId, intent.actionId);

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

    /// @notice Internal function to check circuit breaker state
    /// @param key The rate limit key (keccak256(executorId, actionId))
    function _checkCircuitBreaker(bytes32 key) internal {
        CircuitBreaker storage cb = circuitBreakers[key];

        if (cb.state == CircuitState.OPEN) {
            if (block.timestamp < cb.nextAttemptTime) {
                revert CircuitBreakerOpen("capability_issuer", cb.nextAttemptTime);
            }
            // Move to half-open state
            cb.state = CircuitState.HALF_OPEN;
            cb.successCount = 0;
        }
    }

    /// @notice Internal function to check and update rate limits (sliding window)
    /// @param key The rate limit key (keccak256(executorId, actionId))
    function _checkRateLimit(bytes32 key) internal {
        uint256[] storage windows = rateLimitWindows[key];
        uint256 currentTime = block.timestamp;

        // Remove timestamps outside the window
        while (windows.length > 0 && windows[0] <= currentTime - globalRateLimit.windowSize) {
            // Remove oldest timestamp
            for (uint256 i = 0; i < windows.length - 1; i++) {
                windows[i] = windows[i + 1];
            }
            windows.pop();
        }

        // Check if we're over the limit
        if (windows.length >= globalRateLimit.maxRequests) {
            revert RateLimited("capability_issuer", currentTime + globalRateLimit.windowSize);
        }

        // Add current timestamp
        windows.push(currentTime);
    }

    /// @notice Record a circuit breaker failure
    /// @param executorId The executor ID
    /// @param actionId The action ID
    function _recordCircuitFailure(uint256 executorId, bytes32 actionId) internal {
        bytes32 key = keccak256(abi.encodePacked(executorId, actionId));
        CircuitBreaker storage cb = circuitBreakers[key];

        cb.failureCount++;
        cb.lastFailureTime = block.timestamp;

        if (cb.failureCount >= circuitBreakerThreshold) {
            cb.state = CircuitState.OPEN;
            cb.nextAttemptTime = block.timestamp + circuitBreakerTimeout;
        }
    }

    /// @notice Record a circuit breaker success
    /// @param executorId The executor ID
    /// @param actionId The action ID
    function _recordCircuitSuccess(uint256 executorId, bytes32 actionId) internal {
        bytes32 key = keccak256(abi.encodePacked(executorId, actionId));
        CircuitBreaker storage cb = circuitBreakers[key];

        if (cb.state == CircuitState.HALF_OPEN) {
            cb.successCount++;
            if (cb.successCount >= circuitBreakerRecovery) {
                // Reset to closed state
                cb.state = CircuitState.CLOSED;
                cb.failureCount = 0;
                cb.successCount = 0;
            }
        } else if (cb.state == CircuitState.CLOSED) {
            // Reset failure count on success
            cb.failureCount = 0;
        }
    }

    /// @notice Set global rate limit configuration
    /// @param windowSize Time window in seconds
    /// @param maxRequests Maximum requests per window
    function setGlobalRateLimit(uint256 windowSize, uint256 maxRequests) external {
        require(msg.sender == owner, "Only owner can set rate limits");
        globalRateLimit = RateLimitConfig({
            windowSize: windowSize,
            maxRequests: maxRequests,
            refillRate: 0 // Not used in sliding window mode
        });
    }

    /// @notice Set circuit breaker parameters
    /// @param threshold Failure threshold
    /// @param timeout Recovery timeout in seconds
    /// @param recovery Success count needed in half-open state
    function setCircuitBreakerParams(uint256 threshold, uint256 timeout, uint256 recovery) external {
        require(msg.sender == owner, "Only owner can set circuit breaker params");
        circuitBreakerThreshold = threshold;
        circuitBreakerTimeout = timeout;
        circuitBreakerRecovery = recovery;
    }

    /// @notice Get circuit breaker state for an executor-action pair
    /// @param executorId The executor ID
    /// @param actionId The action ID
    /// @return state Current circuit breaker state
    /// @return failureCount Number of recent failures
    /// @return nextAttemptTime When circuit breaker will allow attempts again
    function getCircuitBreakerState(uint256 executorId, uint256 actionId)
        external
        view
        returns (CircuitState state, uint256 failureCount, uint256 nextAttemptTime)
    {
        bytes32 key = keccak256(abi.encodePacked(executorId, actionId));
        CircuitBreaker storage cb = circuitBreakers[key];
        return (cb.state, cb.failureCount, cb.nextAttemptTime);
    }

    /// @notice Get rate limit state for an executor-action pair
    /// @param executorId The executor ID
    /// @param actionId The action ID
    /// @return requestCount Number of requests in current window
    /// @return windowStart Start time of current window
    function getRateLimitState(uint256 executorId, uint256 actionId)
        external
        view
        returns (uint256 requestCount, uint256 windowStart)
    {
        bytes32 key = keccak256(abi.encodePacked(executorId, actionId));
        uint256[] storage windows = rateLimitWindows[key];
        uint256 currentTime = block.timestamp;
        uint256 count = 0;

        // Count requests in current window
        for (uint256 i = 0; i < windows.length; i++) {
            if (windows[i] > currentTime - globalRateLimit.windowSize) {
                count++;
            }
        }

        return (count, currentTime - globalRateLimit.windowSize);
    }
}
