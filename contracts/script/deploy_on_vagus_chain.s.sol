// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "../src/core/CapabilityIssuer.sol";
import "../src/core/VagalBrake.sol";
import "../src/core/ReflexArc.sol";

/// @title Deploy on Vagus-Chain
/// @notice Deploys Vagus contracts that use vagus-chain's native precompiled contracts
contract DeployOnVagusChain is Script {
    // Vagus-chain native precompiled contract addresses
    address constant ANS_STATE_MANAGER = 0x0000000000000000000000000000000000000001;
    address constant CAPABILITY_ISSUER_NATIVE = 0x0000000000000000000000000000000000000002;
    address constant VAGAL_BRAKE_NATIVE = 0x0000000000000000000000000000000000000003;
    address constant AFFERENT_INBOX = 0x0000000000000000000000000000000000000004;
    address constant REFLEX_ARC_NATIVE = 0x0000000000000000000000000000000000000005;

    function run() external {
        vm.startBroadcast();

        console.log("Deploying Vagus contracts on vagus-chain...");
        console.log("Using precompiled ANSStateManager at:", ANS_STATE_MANAGER);
        console.log("Using precompiled AfferentInbox at:", AFFERENT_INBOX);

        // 1. Deploy CapabilityIssuer (uses precompiled contracts)
        CapabilityIssuer capabilityIssuer = new CapabilityIssuer(
            AFFERENT_INBOX,        // afferentInbox (precompiled)
            VAGAL_BRAKE_NATIVE,    // vagalBrake (precompiled)
            ANS_STATE_MANAGER,     // ansStateManager (precompiled)
            REFLEX_ARC_NATIVE      // reflexArc (precompiled)
        );
        console.log("CapabilityIssuer deployed at:", address(capabilityIssuer));

        // 2. Deploy VagalBrake (uses precompiled contracts)
        VagalBrake vagalBrake = new VagalBrake(
            ANS_STATE_MANAGER,     // ansStateManager (precompiled)
            CAPABILITY_ISSUER_NATIVE // capabilityIssuer (precompiled)
        );
        console.log("VagalBrake deployed at:", address(vagalBrake));

        // 3. Deploy ReflexArc (uses precompiled contracts)
        ReflexArc reflexArc = new ReflexArc(
            AFFERENT_INBOX,        // afferentInbox (precompiled)
            CAPABILITY_ISSUER_NATIVE, // capabilityIssuer (precompiled)
            ANS_STATE_MANAGER      // ansStateManager (precompiled)
        );
        console.log("ReflexArc deployed at:", address(reflexArc));

        // Note: VagalBrake and ReflexArc are deployed but not used in this setup
        // since the precompiled versions are used instead
        console.log("Note: VagalBrake and ReflexArc deployed but precompiled versions are used");

        // Save deployment addresses to JSON file for client configuration
        string memory json = string(abi.encodePacked(
            '{"chain":"vagus-chain",',
            '"afferentInbox":"', vm.toString(AFFERENT_INBOX), '",',
            '"ansStateManager":"', vm.toString(ANS_STATE_MANAGER), '",',
            '"capabilityIssuer":"', vm.toString(address(capabilityIssuer)), '",',
            '"vagalBrake":"', vm.toString(VAGAL_BRAKE_NATIVE), '",',
            '"reflexArc":"', vm.toString(REFLEX_ARC_NATIVE), '",',
            '"deployedContracts":{',
            '"capabilityIssuerDeployed":"', vm.toString(address(capabilityIssuer)), '",',
            '"vagalBrakeDeployed":"', vm.toString(address(vagalBrake)), '",',
            '"reflexArcDeployed":"', vm.toString(address(reflexArc)), '"',
            '}}'
        ));

        vm.writeFile("./contracts/script/VagusChainDeployment.json", json);
        console.log("Deployment addresses saved to VagusChainDeployment.json");

        vm.stopBroadcast();
    }
}
