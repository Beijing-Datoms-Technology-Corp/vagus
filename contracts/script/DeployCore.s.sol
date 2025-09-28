// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "../src/core/AfferentInbox.sol";
import "../src/core/ANSStateManager.sol";
import "../src/core/CapabilityIssuer.sol";
import "../src/core/VagalBrake.sol";
import "../src/core/ReflexArc.sol";

/// @title Deploy Core Contracts
/// @notice Deploys the minimum viable Vagus core contracts
contract DeployCore is Script {
    function run() external {
        vm.startBroadcast();

        // Deploy contracts in dependency order

        // 1. Deploy AfferentInbox
        AfferentInbox afferentInbox = new AfferentInbox();
        console.log("AfferentInbox deployed at:", address(afferentInbox));

        // 2. Deploy ANSStateManager
        ANSStateManager ansStateManager = new ANSStateManager();
        console.log("ANSStateManager deployed at:", address(ansStateManager));

        // 3. Deploy CapabilityIssuer (depends on AfferentInbox, vagalBrake will be set later)
        CapabilityIssuer capabilityIssuer = new CapabilityIssuer(address(afferentInbox), address(0)); // Temporary address(0)
        console.log("CapabilityIssuer deployed at:", address(capabilityIssuer));

        // 4. Deploy VagalBrake (depends on ANSStateManager and CapabilityIssuer)
        VagalBrake vagalBrake = new VagalBrake(address(ansStateManager), address(capabilityIssuer));
        console.log("VagalBrake deployed at:", address(vagalBrake));

        // Set VagalBrake address in CapabilityIssuer
        capabilityIssuer.setVagalBrake(address(vagalBrake));
        console.log("VagalBrake address set in CapabilityIssuer");

        // 5. Deploy ReflexArc (depends on AfferentInbox and CapabilityIssuer)
        ReflexArc reflexArc = new ReflexArc(address(afferentInbox), address(capabilityIssuer));
        console.log("ReflexArc deployed at:", address(reflexArc));

        // Authorize ReflexArc in CapabilityIssuer
        capabilityIssuer.setReflexArc(address(reflexArc));
        console.log("ReflexArc authorized in CapabilityIssuer");

        // Authorize deployer as attestor in AfferentInbox
        afferentInbox.authorizeAttestor(msg.sender);
        console.log("Deployer authorized as attestor");

        // Save deployment addresses to JSON file for later use
        string memory json = string(abi.encodePacked(
            '{"afferentInbox":"', vm.toString(address(afferentInbox)), '",',
            '"ansStateManager":"', vm.toString(address(ansStateManager)), '",',
            '"capabilityIssuer":"', vm.toString(address(capabilityIssuer)), '",',
            '"vagalBrake":"', vm.toString(address(vagalBrake)), '",',
            '"reflexArc":"', vm.toString(address(reflexArc)), '"}'
        ));

        vm.writeFile("./contracts/script/DevnetConfig.json", json);
        console.log("Deployment addresses saved to DevnetConfig.json");

        vm.stopBroadcast();
    }
}
