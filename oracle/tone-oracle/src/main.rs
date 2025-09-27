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
use ethers::types::Address;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

use tone_oracle::{BlockchainConfig, SensorMetrics, ToneOracle, VtiConfig, VtiResult};

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create VTI configuration
    let config = VtiConfig::default();

    // Create oracle - check for blockchain integration
    let oracle = if let (Ok(rpc_url), Ok(private_key), Ok(ans_address)) = (
        std::env::var("RPC_URL"),
        std::env::var("PRIVATE_KEY"),
        std::env::var("ANS_STATE_MANAGER_ADDRESS"),
    ) {
        let ans_address: ethers::types::Address = ans_address.parse()?;
        let blockchain_config = BlockchainConfig {
            rpc_url,
            private_key,
            ans_state_manager_address: ans_address,
        };

        tracing::info!("Enabling blockchain integration with ANS State Manager at {:?}", ans_address);
        ToneOracle::new_with_blockchain(config, blockchain_config).await?
    } else {
        tracing::info!("Running without blockchain integration (set RPC_URL, PRIVATE_KEY, and ANS_STATE_MANAGER_ADDRESS to enable)");
        ToneOracle::new(config)
    };

    let state = AppState {
        oracle: Arc::new(Mutex::new(oracle)),
    };

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/vti", post(submit_metrics))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    let addr = "0.0.0.0:3000";
    tracing::info!("Tone Oracle listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
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

    Ok(Json(VtiResponse {
        success: true,
        vti_result: result,
        error: None,
    }))
}
