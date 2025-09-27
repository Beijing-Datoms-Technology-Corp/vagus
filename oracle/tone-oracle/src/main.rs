//! Tone Oracle - VTI computation and ANS state updates
//!
//! Provides HTTP interface for sensor metrics submission and VTI computation.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use clap::{Parser, Subcommand};
use ethers::types::Address;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

use tone_oracle::{BlockchainConfig, SensorMetrics, ToneOracle, VtiConfig, VtiResult};
use vagus_chain::{ChainClient, ChainClientFactory, ChainConfig, ChainType};

/// HTTP request for submitting sensor metrics
#[derive(Debug, Deserialize)]
struct SubmitMetricsRequest {
    executor_id: u64,
    human_distance_mm: f64,
    temperature_celsius: f64,
    energy_consumption_j: f64,
    jerk_m_s3: f64,
    timestamp_ms: Option<u64>,
}

/// HTTP response for VTI computation
#[derive(Debug, Serialize)]
struct VtiResponse {
    success: bool,
    vti_result: Option<VtiResult>,
    error: Option<String>,
}

/// HTTP response for health check
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

/// Application state
#[derive(Clone)]
struct AppState {
    oracle: Arc<Mutex<ToneOracle>>,
    chain_clients: HashMap<ChainType, Arc<dyn ChainClient>>,
}

/// CLI arguments
#[derive(Parser)]
#[command(name = "tone-oracle")]
#[command(about = "VTI computation and ANS state updates for Vagus")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the oracle server
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "3000")]
        port: u16,

        /// Enable EVM chain integration
        #[arg(long)]
        evm_rpc: Option<String>,

        /// Enable Cosmos chain integration
        #[arg(long)]
        cosmos_rpc: Option<String>,

        /// Private key for blockchain transactions
        #[arg(long, env = "PRIVATE_KEY")]
        private_key: Option<String>,

        /// ANS State Manager contract addresses (chain_name=address)
        #[arg(long, value_parser = parse_contract_addresses)]
        ans_state_managers: Vec<(String, String)>,

        /// Other contract addresses (chain_name=contract_name=address)
        #[arg(long, value_parser = parse_contract_addresses)]
        contracts: Vec<(String, String, String)>,
    },
}

fn parse_contract_addresses(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.split('=').collect();
    match parts.len() {
        2 => Ok((parts[0].to_string(), parts[1].to_string())),
        3 => Ok((parts[0].to_string(), format!("{}={}", parts[1], parts[2]))),
        _ => Err("Invalid contract address format".to_string()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Commands::Serve {
            port,
            evm_rpc,
            cosmos_rpc,
            private_key,
            ans_state_managers,
            contracts,
        } => {
            run_server(port, evm_rpc, cosmos_rpc, private_key, ans_state_managers, contracts).await
        }
    }
}

async fn run_server(
    port: u16,
    evm_rpc: Option<String>,
    cosmos_rpc: Option<String>,
    private_key: Option<String>,
    ans_state_managers: Vec<(String, String)>,
    contracts: Vec<(String, String, String)>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create VTI configuration
    let config = VtiConfig::default();

    // Create oracle - for now use legacy blockchain config if EVM is enabled
    let oracle = if let (Some(rpc_url), Some(private_key), Some(ans_addr)) = (
        evm_rpc.as_ref(),
        private_key.as_ref(),
        ans_state_managers.iter().find(|(chain, _)| chain == "evm").map(|(_, addr)| addr)
    ) {
        let ans_address: ethers::types::Address = ans_addr.parse()?;
        let blockchain_config = BlockchainConfig {
            rpc_url: rpc_url.clone(),
            private_key: private_key.clone(),
            ans_state_manager_address: ans_address,
        };

        tracing::info!("Enabling EVM blockchain integration with ANS State Manager at {:?}", ans_address);
        ToneOracle::new_with_blockchain(config, blockchain_config).await?
    } else {
        tracing::info!("Running without blockchain integration");
        ToneOracle::new(config)
    };

    // Create chain clients
    let mut chain_clients = HashMap::new();

    // Create EVM client if configured
    if let (Some(rpc_url), Some(private_key)) = (evm_rpc, private_key.clone()) {
        let mut contract_addresses = HashMap::new();

        // Add ANS state manager
        if let Some((_, addr)) = ans_state_managers.iter().find(|(chain, _)| chain == "evm") {
            contract_addresses.insert("ans_state_manager".to_string(), addr.clone());
        }

        // Add other contracts for EVM
        for (chain, contract_name, addr) in &contracts {
            if chain == "evm" {
                contract_addresses.insert(contract_name.clone(), addr.clone());
            }
        }

        let chain_config = ChainConfig {
            chain_type: ChainType::EVM,
            rpc_url,
            contract_addresses,
            private_key,
        };

        match ChainClientFactory::create_client(chain_config).await {
            Ok(client) => {
                chain_clients.insert(ChainType::EVM, Arc::from(client) as Arc<dyn ChainClient>);
                tracing::info!("EVM chain client initialized");
            }
            Err(e) => {
                tracing::warn!("Failed to create EVM chain client: {}", e);
            }
        }
    }

    // Create Cosmos client if configured
    if let (Some(rpc_url), Some(private_key)) = (cosmos_rpc, private_key) {
        let mut contract_addresses = HashMap::new();

        // Add ANS state manager
        if let Some((_, addr)) = ans_state_managers.iter().find(|(chain, _)| chain == "cosmos") {
            contract_addresses.insert("ans_state_manager".to_string(), addr.clone());
        }

        // Add other contracts for Cosmos
        for (chain, contract_name, addr) in &contracts {
            if chain == "cosmos" {
                contract_addresses.insert(contract_name.clone(), addr.clone());
            }
        }

        let chain_config = ChainConfig {
            chain_type: ChainType::Cosmos,
            rpc_url,
            contract_addresses,
            private_key,
        };

        match ChainClientFactory::create_client(chain_config).await {
            Ok(client) => {
                chain_clients.insert(ChainType::Cosmos, Arc::from(client) as Arc<dyn ChainClient>);
                tracing::info!("Cosmos chain client initialized");
            }
            Err(e) => {
                tracing::warn!("Failed to create Cosmos chain client: {}", e);
            }
        }
    }

    let state = AppState {
        oracle: Arc::new(Mutex::new(oracle)),
        chain_clients,
    };

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/vti", post(submit_metrics))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Tone Oracle listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Submit sensor metrics and get VTI result
async fn submit_metrics(
    State(state): State<AppState>,
    Json(request): Json<SubmitMetricsRequest>,
) -> Result<Json<VtiResponse>, StatusCode> {
    // Convert request to SensorMetrics
    let metrics = SensorMetrics {
        executor_id: request.executor_id,
        human_distance_mm: request.human_distance_mm,
        temperature_celsius: request.temperature_celsius,
        energy_consumption_j: request.energy_consumption_j,
        jerk_m_s3: request.jerk_m_s3,
        timestamp_ms: request.timestamp_ms.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        }),
    };

    // Process metrics (now async due to potential blockchain calls)
    let mut oracle = state.oracle.lock().await;
    let result = match oracle.process_metrics(metrics).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Failed to process metrics: {}", e);
            return Ok(Json(VtiResponse {
                success: false,
                vti_result: None,
                error: Some(format!("Processing failed: {}", e)),
            }));
        }
    };

    // Update ANS state on all configured chains
    for (chain_type, client) in &state.chain_clients {
        if let Some(vti_result) = &result {
            // Convert string to ANSState enum
            let suggested_state = match vti_result.suggested_state.as_str() {
                "SAFE" => vagus_chain::ANSState::SAFE,
                "DANGER" => vagus_chain::ANSState::DANGER,
                "SHUTDOWN" => vagus_chain::ANSState::SHUTDOWN,
                _ => {
                    tracing::warn!("Unknown ANS state: {}", vti_result.suggested_state);
                    continue;
                }
            };

            match client.update_tone(vti_result.vti_value, suggested_state).await {
                Ok(_) => {
                    tracing::info!("Updated ANS state on {:?} chain", chain_type);
                }
                Err(e) => {
                    tracing::warn!("Failed to update ANS state on {:?} chain: {}", chain_type, e);
                    // Don't fail the request if one chain update fails
                }
            }
        }
    }

    Ok(Json(VtiResponse {
        success: true,
        vti_result: result,
        error: None,
    }))
}
