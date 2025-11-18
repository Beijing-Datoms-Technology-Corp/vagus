// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "../vagusmaker/ReputationToken.sol";
import "../vagusmaker/MicroTaskManager.sol";
import "../vagusmaker/RedFlagValidator.sol";
import "../vagusmaker/ReputationWeightedVoter.sol";

/// @title Deploy VagusMaker Contracts
/// @notice Deploys the VagusMaker system for blockchain-native MAKER consensus
contract DeployVagusMaker is Script {
    function run() external {
        vm.startBroadcast();

        // Deploy contracts in dependency order

        // 1. Deploy RedFlagValidator (pure functions, no dependencies)
        RedFlagValidator validator = new RedFlagValidator();
        console.log("RedFlagValidator deployed at:", address(validator));

        // 2. Deploy ReputationToken (only needs VagusMaker address which we'll set later)
        ReputationToken repToken = new ReputationToken(address(0)); // Temporary address
        console.log("ReputationToken deployed at:", address(repToken));

        // 3. Deploy MicroTaskManager (inherits ANSStateManager)
        MicroTaskManager taskManager = new MicroTaskManager();
        console.log("MicroTaskManager deployed at:", address(taskManager));

        // Note: In production, we would need to deploy the core Vagus contracts first
        // For MVP demo, we'll use mock addresses for ReflexArc
        address mockReflexArc = address(0x1234567890123456789012345678901234567890);

        // 4. Deploy ReputationWeightedVoter (depends on all above)
        ReputationWeightedVoter voter = new ReputationWeightedVoter(
            address(repToken),
            address(validator),
            mockReflexArc
        );
        console.log("ReputationWeightedVoter deployed at:", address(voter));

        // 5. Set VagusMaker address in ReputationToken
        // Create a new token instance with correct address
        ReputationToken correctRepToken = new ReputationToken(address(voter));
        console.log("ReputationToken (corrected) deployed at:", address(correctRepToken));

        // Note: In production, you would need to update the voter contract to use the correct rep token
        // For this demo, we'll proceed with the initial deployment

        // Save deployment addresses to JSON file
        string memory json = string(abi.encodePacked(
            '{"reputationToken":"', vm.toString(address(repToken)), '",',
            '"microTaskManager":"', vm.toString(address(taskManager)), '",',
            '"redFlagValidator":"', vm.toString(address(validator)), '",',
            '"reputationWeightedVoter":"', vm.toString(address(voter)), '",',
            '"reputationTokenCorrected":"', vm.toString(address(correctRepToken)), '"}'
        ));

        vm.writeFile("./contracts/script/VagusMakerConfig.json", json);
        console.log("VagusMaker deployment addresses saved to VagusMakerConfig.json");

        vm.stopBroadcast();
    }
}
