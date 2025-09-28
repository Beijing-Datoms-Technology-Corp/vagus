//! Golden Test Runner
//!
//! Runs cross-chain invariant and equivalence tests against EVM and CosmWasm implementations.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use vagus_chain::{ChainConfig, ChainType};

mod lib;
use lib::{GoldenTestHarness, scenarios};

#[derive(Parser)]
#[command(name = "golden-tests")]
#[command(about = "Vagus cross-chain golden test suite")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all golden tests
    Run {
        /// EVM RPC URL
        #[arg(long, default_value = "http://localhost:8545")]
        evm_rpc: String,

        /// Cosmos RPC URL
        #[arg(long, default_value = "http://localhost:26657")]
        cosmos_rpc: String,

        /// Private key for transactions
        #[arg(long, env = "PRIVATE_KEY")]
        private_key: Option<String>,

        /// Contract addresses (format: chain=contract=address)
        #[arg(long)]
        contracts: Vec<String>,
    },
    /// List available test scenarios
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Commands::Run { evm_rpc, cosmos_rpc, private_key, contracts } => {
            run_tests(evm_rpc, cosmos_rpc, private_key, contracts).await
        }
        Commands::List => {
            list_scenarios()
        }
    }
}

async fn run_tests(
    evm_rpc: String,
    cosmos_rpc: String,
    private_key: Option<String>,
    contract_specs: Vec<String>,
) -> Result<()> {
    println!("🧪 Starting Vagus Golden Test Suite");
    println!("===================================");

    // Parse contract addresses
    let mut contract_addresses = HashMap::new();
    for spec in contract_specs {
        let parts: Vec<&str> = spec.split('=').collect();
        if parts.len() == 3 {
            let chain = parts[0];
            let contract = parts[1];
            let address = parts[2];

            contract_addresses.insert(
                format!("{}_{}", chain, contract),
                address.to_string(),
            );
        }
    }

    // Default contract addresses for testing
    let default_private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string();
    let private_key = private_key.unwrap_or(default_private_key);

    // Create EVM config
    let mut evm_contracts = HashMap::new();
    evm_contracts.insert("afferent_inbox".to_string(), "0x0000000000000000000000000000000000000000".to_string());
    evm_contracts.insert("ans_state_manager".to_string(), "0x0000000000000000000000000000000000000000".to_string());
    evm_contracts.insert("capability_issuer".to_string(), "0x0000000000000000000000000000000000000000".to_string());
    evm_contracts.insert("reflex_arc".to_string(), "0x0000000000000000000000000000000000000000".to_string());

    let evm_config = ChainConfig {
        chain_type: ChainType::EVM,
        rpc_url: evm_rpc,
        contract_addresses: evm_contracts,
        private_key: Some(private_key.clone()),
    };

    // Create Cosmos config
    let mut cosmos_contracts = HashMap::new();
    cosmos_contracts.insert("afferent_inbox".to_string(), "vagus1afferentinbox".to_string());
    cosmos_contracts.insert("ans_state_manager".to_string(), "vagus1ansstatemanager".to_string());
    cosmos_contracts.insert("capability_issuer".to_string(), "vagus1capabilityissuer".to_string());
    cosmos_contracts.insert("reflex_arc".to_string(), "vagus1reflexarc".to_string());

    let cosmos_config = ChainConfig {
        chain_type: ChainType::Cosmos,
        rpc_url: cosmos_rpc,
        contract_addresses: cosmos_contracts,
        private_key: Some(private_key),
    };

    // Create test harness
    let harness = GoldenTestHarness::new(evm_config, cosmos_config).await?;

    // Run test scenarios
    let test_scenarios = vec![
        scenarios::basic_state_transitions(),
        scenarios::reflex_arc_triggering(),
    ];

    let mut all_passed = true;
    for scenario in test_scenarios {
        println!("\n🎯 Running scenario: {}", scenario.name);
        println!("   {}", scenario.description);

        match harness.run_scenario(&scenario).await {
            Ok(results) => {
                if results.passed() {
                    println!("   ✅ PASSED");
                } else {
                    println!("   ❌ FAILED");
                    all_passed = false;

                    for result in &results.invariant_results {
                        if !result.evm_passed || !result.cosmos_passed {
                            println!("      Invariant: {:?}", result.invariant);
                            if !result.evm_passed {
                                println!("        EVM: ❌ {:?}", result.evm_error);
                            }
                            if !result.cosmos_passed {
                                println!("        Cosmos: ❌ {:?}", result.cosmos_error);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("   ❌ ERROR: {}", e);
                all_passed = false;
            }
        }
    }

    println!("\n===================================");
    if all_passed {
        println!("🎉 All golden tests PASSED!");
        std::process::exit(0);
    } else {
        println!("💥 Some golden tests FAILED!");
        std::process::exit(1);
    }
}

fn list_scenarios() {
    println!("📋 Available Test Scenarios:");
    println!("============================");

    let scenarios = vec![
        scenarios::basic_state_transitions(),
        scenarios::reflex_arc_triggering(),
    ];

    for (i, scenario) in scenarios.iter().enumerate() {
        println!("{}. {}", i + 1, scenario.name);
        println!("   {}", scenario.description);
        println!("   Actions: {}", scenario.setup_actions.len());
        println!("   Invariants: {}", scenario.invariant_checks.len());
        println!();
    }
}
