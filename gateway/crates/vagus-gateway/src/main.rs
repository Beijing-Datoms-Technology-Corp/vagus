//! Vagus Gateway Binary
//!
//! Command-line interface for running the Vagus device-side gateway.

use anyhow::Result;
use clap::{Parser, Subcommand};
use ethers::types::Address;
use std::{collections::HashMap, str::FromStr};
use tracing_subscriber;
use vagus_chain::{ChainClient, ChainClientFactory, ChainConfig, ChainType};
use vagus_crypto::VagusDomain;
use vagus_gateway::manager::GatewayConfig;
use vagus_gateway::VagusGateway;

#[derive(Parser)]
#[command(name = "vagus-gateway")]
#[command(about = "Vagus device-side safety gateway")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the gateway
    Start {
        /// Executor ID for this gateway instance
        #[arg(long, default_value = "1")]
        executor_id: u64,

        /// Blockchain type (evm or cosmos)
        #[arg(long, default_value = "evm")]
        chain: String,

        /// RPC/WebSocket URL for blockchain connection
        #[arg(long, default_value = "ws://localhost:8545")]
        rpc_url: String,

        /// AfferentInbox contract address
        #[arg(long)]
        afferent_inbox: String,

        /// ANSStateManager contract address
        #[arg(long)]
        ans_state_manager: String,

        /// CapabilityIssuer contract address
        #[arg(long)]
        capability_issuer: String,

        /// ReflexArc contract address
        #[arg(long)]
        reflex_arc: String,

        /// Telemetry window duration in milliseconds
        #[arg(long, default_value = "1000")]
        window_duration_ms: u64,

        /// Evidence submission interval in milliseconds
        #[arg(long, default_value = "5000")]
        evidence_interval_ms: u64,
    },
    /// Run with simulated sensors for testing
    #[command(name = "sim")]
    Simulate {
        /// Executor ID for this gateway instance
        #[arg(long, default_value = "1")]
        executor_id: u64,

        /// Blockchain type (evm or cosmos)
        #[arg(long, default_value = "evm")]
        chain: String,

        /// RPC/WebSocket URL for blockchain connection
        #[arg(long, default_value = "ws://localhost:8545")]
        rpc_url: String,

        /// AfferentInbox contract address
        #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
        afferent_inbox: String,

        /// ANSStateManager contract address
        #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
        ans_state_manager: String,

        /// CapabilityIssuer contract address
        #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
        capability_issuer: String,

        /// ReflexArc contract address
        #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
        reflex_arc: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Commands::Start {
            executor_id,
            chain,
            rpc_url,
            afferent_inbox,
            ans_state_manager,
            capability_issuer,
            reflex_arc,
            window_duration_ms,
            evidence_interval_ms,
        } => {
            let chain_type = match chain.as_str() {
                "evm" => vagus_chain::ChainType::EVM,
                "cosmos" => vagus_chain::ChainType::Cosmos,
                _ => return Err(anyhow::anyhow!("Unsupported chain type: {}", chain)),
            };

            let mut contract_addresses = std::collections::HashMap::new();
            contract_addresses.insert("afferent_inbox".to_string(), afferent_inbox);
            contract_addresses.insert("ans_state_manager".to_string(), ans_state_manager);
            contract_addresses.insert("capability_issuer".to_string(), capability_issuer);
            contract_addresses.insert("reflex_arc".to_string(), reflex_arc);

            run_multichain_gateway(
                executor_id,
                chain_type,
                rpc_url,
                contract_addresses,
                window_duration_ms,
                evidence_interval_ms,
                false,
            ).await
        }
        Commands::Simulate {
            executor_id,
            chain,
            rpc_url,
            afferent_inbox,
            ans_state_manager,
            capability_issuer,
            reflex_arc,
        } => {
            let chain_type = match chain.as_str() {
                "evm" => vagus_chain::ChainType::EVM,
                "cosmos" => vagus_chain::ChainType::Cosmos,
                _ => return Err(anyhow::anyhow!("Unsupported chain type: {}", chain)),
            };

            let mut contract_addresses = std::collections::HashMap::new();
            contract_addresses.insert("afferent_inbox".to_string(), afferent_inbox);
            contract_addresses.insert("ans_state_manager".to_string(), ans_state_manager);
            contract_addresses.insert("capability_issuer".to_string(), capability_issuer);
            contract_addresses.insert("reflex_arc".to_string(), reflex_arc);

            run_multichain_gateway(
                executor_id,
                chain_type,
                rpc_url,
                contract_addresses,
                1000,
                5000,
                true,
            ).await
        }
    }
}

async fn run_gateway(config: GatewayConfig) -> Result<()> {
    // Create crypto utilities
    let crypto_domain = VagusDomain {
        name: "Vagus".to_string(),
        version: "1".to_string(),
        chain_id: 31337, // TODO: Make configurable
        verifying_contract: config.afferent_inbox_address,
    };

    let crypto = vagus_crypto::VagusCrypto::new(crypto_domain);

    // Create and start gateway
    let gateway = VagusGateway::new(config, crypto);
    gateway.start().await?;

    // Keep the gateway running
    tokio::signal::ctrl_c().await?;
    println!("Shutting down gateway...");

    Ok(())
}

async fn run_simulation(config: GatewayConfig) -> Result<()> {
    println!("Starting Vagus Gateway in simulation mode");
    println!("Executor ID: {}", config.executor_id);
    println!("This mode generates mock sensor data for testing");

    // Create crypto utilities
    let crypto_domain = VagusDomain {
        name: "Vagus".to_string(),
        version: "1".to_string(),
        chain_id: 31337,
        verifying_contract: config.afferent_inbox_address,
    };

    let crypto = vagus_crypto::VagusCrypto::new(crypto_domain);

    // Create gateway
    let gateway = VagusGateway::new(config.clone(), crypto);

    // Start mock sensor data generation
    start_mock_sensors(gateway, config.executor_id).await?;

    // Keep running
    tokio::signal::ctrl_c().await?;
    println!("Shutting down simulation...");

    Ok(())
}

async fn start_mock_sensors(gateway: VagusGateway, executor_id: u64) -> Result<()> {
    use vagus_gateway::collector::MockSensorDataGenerator;
    use vagus_telemetry::SensorReading;

    let mut generator = MockSensorDataGenerator::new(executor_id);

    tokio::spawn(async move {
        loop {
            // Generate and add sensor readings
            let readings = generator.generate_readings(4);

            for reading in readings {
                if let Err(e) = gateway.add_sensor_reading(reading).await {
                    eprintln!("Error adding sensor reading: {:?}", e);
                }
            }

            // Check current VTI
            match gateway.get_current_vti().await {
                Ok(Some(vti)) => {
                    println!("Current VTI: {:.3} (contributions: {:?})",
                             vti.value, vti.contributions);
                }
                Ok(None) => {
                    println!("No telemetry data available yet");
                }
                Err(e) => {
                    eprintln!("Error getting VTI: {:?}", e);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }
    });

    Ok(())
}

async fn run_multichain_gateway(
    executor_id: u64,
    chain_type: ChainType,
    rpc_url: String,
    contract_addresses: HashMap<String, String>,
    window_duration_ms: u64,
    evidence_interval_ms: u64,
    simulation_mode: bool,
) -> Result<()> {
    println!("Starting Vagus Gateway with chain type: {:?}", chain_type);

    // Create chain client configuration
    let chain_config = ChainConfig {
        chain_type,
        rpc_url: rpc_url.clone(),
        contract_addresses,
        private_key: Some("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string()), // Default anvil key
    };

    // Create chain client
    let chain_client = ChainClientFactory::create_client(chain_config).await?;

    // Create crypto utilities (for now, still using EVM domain)
    let crypto_domain = VagusDomain {
        name: "Vagus".to_string(),
        version: "1".to_string(),
        chain_id: 31337, // TODO: Make configurable per chain
        verifying_contract: Address::zero(), // TODO: Update for multichain
    };

    let crypto = vagus_crypto::VagusCrypto::new(crypto_domain);

    // For now, fall back to legacy gateway implementation
    // TODO: Fully integrate with new ChainClient trait
    if simulation_mode {
        run_simulation_legacy(executor_id, rpc_url).await
    } else {
        run_gateway_legacy(executor_id, rpc_url).await
    }
}

async fn run_gateway_legacy(executor_id: u64, rpc_url: String) -> Result<()> {
    // Placeholder - implement legacy gateway with new chain client
    println!("Legacy gateway mode for executor {} on {}", executor_id, rpc_url);
    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn run_simulation_legacy(executor_id: u64, rpc_url: String) -> Result<()> {
    // Placeholder - implement legacy simulation with new chain client
    println!("Legacy simulation mode for executor {} on {}", executor_id, rpc_url);
    tokio::signal::ctrl_c().await?;
    Ok(())
}
