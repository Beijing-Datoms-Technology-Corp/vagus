// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title ERC-5192 Soulbound Token Interface
/// @notice Interface for non-transferable tokens
interface IERC5192 {
    /// @notice Emitted when the locking status is changed to locked
    event Locked(uint256 tokenId);
    /// @notice Emitted when the locking status is changed to unlocked
    event Unlocked(uint256 tokenId);

    /// @notice Returns the locking status of an NFT
    /// @dev SBTs assigned to dead addresses or other smart contracts should be considered locked
    function locked(uint256 tokenId) external view returns (bool);
}

/// @title VagusMaker Reputation Token - Soulbound ERC-5192
/// @notice Soulbound reputation token for VagusMaker participants
/// @dev Implements ERC5192 to ensure tokens are non-transferable and permanent
contract ReputationToken is IERC5192 {
    /// @notice Mapping of token ID to owner
    mapping(uint256 => address) public ownerOf;

    /// @notice Mapping of owner to token ID (1:1 relationship)
    mapping(address => uint256) public tokenOf;

    /// @notice Mapping of address to reputation score
    mapping(address => uint256) public reputation;

    /// @notice VagusMaker contract that can mint and update reputation
    address public immutable vagusMaker;

    /// @notice Token name
    string public constant name = "VagusMaker Reputation";

    /// @notice Token symbol
    string public constant symbol = "VMREP";

    /// @notice Total supply of tokens
    uint256 public totalSupply;

    /// @notice Constructor sets the VagusMaker contract
    /// @param _vagusMaker Address of the VagusMaker contract
    constructor(address _vagusMaker) {
        vagusMaker = _vagusMaker;
    }

    /// @notice Modifier to restrict access to VagusMaker contract
    modifier onlyVagusMaker() {
        require(msg.sender == vagusMaker, "Only VagusMaker");
        _;
    }

    /// @notice Mint a reputation token for a participant
    /// @param to Address to receive the token
    function mint(address to) external onlyVagusMaker {
        require(ownerOf[uint256(uint160(to))] == address(0), "Token already exists");
        require(tokenOf[to] == 0, "Address already has token");

        uint256 tokenId = uint256(uint160(to));
        ownerOf[tokenId] = to;
        tokenOf[to] = tokenId;
        totalSupply++;

        reputation[to] = 100; // Initial reputation score

        emit Transfer(address(0), to, tokenId);
        emit Locked(tokenId);
    }

    /// @notice Get token balance of an address
    /// @param owner Address to check
    /// @return Balance (0 or 1)
    function balanceOf(address owner) external view returns (uint256) {
        return tokenOf[owner] > 0 ? 1 : 0;
    }

    /// @notice ERC721 transfer function (disabled for soulbound tokens)
    function transferFrom(address, address, uint256) external pure {
        revert("Soulbound token: non-transferable");
    }

    /// @notice ERC721 safe transfer function (disabled for soulbound tokens)
    function safeTransferFrom(address, address, uint256, bytes calldata) external pure {
        revert("Soulbound token: non-transferable");
    }

    /// @notice ERC721 safe transfer function (disabled for soulbound tokens)
    function safeTransferFrom(address, address, uint256) external pure {
        revert("Soulbound token: non-transferable");
    }

    /// @notice ERC721 approve function (disabled for soulbound tokens)
    function approve(address, uint256) external pure {
        revert("Soulbound token: non-approvable");
    }

    /// @notice ERC721 setApprovalForAll function (disabled for soulbound tokens)
    function setApprovalForAll(address, bool) external pure {
        revert("Soulbound token: non-approvable");
    }

    /// @notice ERC721 getApproved function
    function getApproved(uint256) external pure returns (address) {
        return address(0);
    }

    /// @notice ERC721 isApprovedForAll function
    function isApprovedForAll(address, address) external pure returns (bool) {
        return false;
    }

    /// @notice ERC721 supportsInterface function
    function supportsInterface(bytes4 interfaceId) external pure returns (bool) {
        return interfaceId == 0x01ffc9a7 || // ERC165
               interfaceId == 0x80ac58cd || // ERC721
               interfaceId == 0xb45a3c0e;   // ERC5192
    }

    /// @notice ERC721 Transfer event
    event Transfer(address indexed from, address indexed to, uint256 indexed tokenId);

    /// @notice Update reputation score for a participant
    /// @param user Address of the participant
    /// @param delta Reputation change (can be negative)
    function updateReputation(address user, int256 delta) external onlyVagusMaker {
        int256 newRep = int256(reputation[user]) + delta;
        reputation[user] = newRep > 0 ? uint256(newRep) : 0;
    }

    /// @notice Override locked function from ERC5192
    /// @param tokenId Token ID to check
    /// @return Always returns true (soulbound tokens are permanently locked)
    function locked(uint256 tokenId) external pure override returns (bool) {
        return true; // Permanently locked, non-transferable
    }
}