//! Vagus Gateway Binary
//!
//! Command-line interface for running the Vagus device-side gateway.

use anyhow::Result;
use clap::{Parser, Subcommand};
use ethers::types::Address;
use std::str::FromStr;
use tracing_subscriber;
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

        /// WebSocket URL for blockchain connection
        #[arg(long, default_value = "ws://localhost:8545")]
        ws_url: String,

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

        /// WebSocket URL for blockchain connection
        #[arg(long, default_value = "ws://localhost:8545")]
        ws_url: String,

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
            ws_url,
            afferent_inbox,
            ans_state_manager,
            capability_issuer,
            reflex_arc,
            window_duration_ms,
            evidence_interval_ms,
        } => {
            run_gateway(GatewayConfig {
                executor_id,
                websocket_url: ws_url,
                afferent_inbox_address: Address::from_str(&afferent_inbox)?,
                ans_state_manager_address: Address::from_str(&ans_state_manager)?,
                capability_issuer_address: Address::from_str(&capability_issuer)?,
                reflex_arc_address: Address::from_str(&reflex_arc)?,
                window_duration_ms,
                evidence_submission_interval_ms: evidence_interval_ms,
            }).await
        }
        Commands::Simulate {
            executor_id,
            ws_url,
            afferent_inbox,
            ans_state_manager,
            capability_issuer,
            reflex_arc,
        } => {
            run_simulation(GatewayConfig {
                executor_id,
                websocket_url: ws_url,
                afferent_inbox_address: Address::from_str(&afferent_inbox)?,
                ans_state_manager_address: Address::from_str(&ans_state_manager)?,
                capability_issuer_address: Address::from_str(&capability_issuer)?,
                reflex_arc_address: Address::from_str(&reflex_arc)?,
                window_duration_ms: 1000,
                evidence_submission_interval_ms: 5000,
            }).await
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
