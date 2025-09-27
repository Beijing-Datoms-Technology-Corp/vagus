//! Tone Oracle Library
//!
//! Provides VTI (Vagal Tone Indicator) computation and ANS state updates
//! based on sensor telemetry data.

use ethers::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

/// Sensor metrics input for VTI calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorMetrics {
    pub executor_id: u64,
    pub human_distance_mm: f64,
    pub temperature_celsius: f64,
    pub energy_consumption_j: f64,
    pub jerk_m_s3: f64,
    pub timestamp_ms: u64,
}

/// VTI calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VtiResult {
    pub vti_value: u64, // 0-10000 (basis points)
    pub suggested_state: String, // "SAFE", "DANGER", "SHUTDOWN"
}

/// Configuration for VTI computation
#[derive(Debug, Clone)]
pub struct VtiConfig {
    pub window_size: usize, // Number of metrics to keep for averaging
    pub safe_threshold: f64,
    pub danger_threshold: f64,
    pub shutdown_threshold: f64,
    pub hysteresis_margin: f64,
}

impl Default for VtiConfig {
    fn default() -> Self {
        Self {
            window_size: 10,
            safe_threshold: 3000.0,    // 30%
            danger_threshold: 7000.0,  // 70%
            shutdown_threshold: 9000.0, // 90%
            hysteresis_margin: 500.0,  // 5%
        }
    }
}

/// VTI Calculator with sliding window
pub struct VtiCalculator {
    config: VtiConfig,
    metrics_window: VecDeque<SensorMetrics>,
}

impl VtiCalculator {
    pub fn new(config: VtiConfig) -> Self {
        let window_size = config.window_size;
        Self {
            config,
            metrics_window: VecDeque::with_capacity(window_size),
        }
    }

    /// Add new sensor metrics to the window
    pub fn add_metrics(&mut self, metrics: SensorMetrics) {
        if self.metrics_window.len() >= self.config.window_size {
            self.metrics_window.pop_front();
        }
        self.metrics_window.push_back(metrics);
    }

    /// Compute VTI from current metrics window
    pub fn compute_vti(&self) -> Option<VtiResult> {
        if self.metrics_window.is_empty() {
            return None;
        }

        // Compute average metrics
        let mut total_distance = 0.0;
        let mut total_temp = 0.0;
        let mut total_energy = 0.0;
        let mut total_jerk = 0.0;

        for metrics in &self.metrics_window {
            total_distance += metrics.human_distance_mm;
            total_temp += metrics.temperature_celsius;
            total_energy += metrics.energy_consumption_j;
            total_jerk += metrics.jerk_m_s3;
        }

        let count = self.metrics_window.len() as f64;
        let avg_distance = total_distance / count;
        let avg_temp = total_temp / count;
        let avg_energy = total_energy / count;
        let avg_jerk = total_jerk / count;

        // Simple VTI calculation (MVP)
        // Higher risk factors increase VTI:
        // - Close human distance (< 500mm)
        // - High temperature (> 50°C)
        // - High energy consumption
        // - High jerk (sudden movements)

        let mut risk_score = 0.0;

        // Distance risk (inverse relationship)
        if avg_distance < 500.0 {
            risk_score += (500.0 - avg_distance) / 500.0 * 30.0;
        }

        // Temperature risk
        if avg_temp > 50.0 {
            risk_score += (avg_temp - 50.0) / 50.0 * 20.0;
        }

        // Energy risk (normalize by some baseline)
        let energy_risk = (avg_energy / 1000.0).min(1.0) * 25.0;
        risk_score += energy_risk;

        // Jerk risk
        let jerk_risk = (avg_jerk / 10.0).min(1.0) * 25.0;
        risk_score += jerk_risk;

        // Clamp to 0-100
        let clamped_risk = risk_score.max(0.0).min(100.0);

        // Convert to basis points (0-10000)
        let vti_value = (clamped_risk * 100.0) as u64;

        // Determine suggested state with hysteresis
        let suggested_state = if vti_value >= (self.config.shutdown_threshold as u64) {
            "SHUTDOWN"
        } else if vti_value >= (self.config.danger_threshold as u64) {
            "DANGER"
        } else if vti_value <= ((self.config.safe_threshold - self.config.hysteresis_margin) as u64) {
            "SAFE"
        } else {
            // Stay in current state for hysteresis
            "UNKNOWN" // Will be resolved by ANS contract
        };

        Some(VtiResult {
            vti_value,
            suggested_state: suggested_state.to_string(),
        })
    }
}

/// Oracle service state
pub struct ToneOracle {
    calculator: VtiCalculator,
    config: VtiConfig,
    blockchain: Option<BlockchainOracle>,
}

impl ToneOracle {
    /// Create a new oracle without blockchain integration
    pub fn new(config: VtiConfig) -> Self {
        Self {
            calculator: VtiCalculator::new(config.clone()),
            config,
            blockchain: None,
        }
    }

    /// Create a new oracle with blockchain integration
    pub async fn new_with_blockchain(
        config: VtiConfig,
        blockchain_config: BlockchainConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let blockchain = Some(BlockchainOracle::new(&blockchain_config).await?);

        Ok(Self {
            calculator: VtiCalculator::new(config.clone()),
            config,
            blockchain,
        })
    }

    /// Process sensor metrics and compute VTI, optionally updating blockchain
    pub async fn process_metrics(&mut self, metrics: SensorMetrics) -> Result<Option<VtiResult>, Box<dyn std::error::Error>> {
        self.calculator.add_metrics(metrics);

        if let Some(result) = self.calculator.compute_vti() {
            // If blockchain integration is enabled, update the contract
            if let Some(blockchain) = &self.blockchain {
                if result.suggested_state != "UNKNOWN" {
                    blockchain.update_tone(result.vti_value, &result.suggested_state).await?;
                }
            }
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &VtiConfig {
        &self.config
    }

    /// Check if blockchain integration is enabled
    pub fn has_blockchain(&self) -> bool {
        self.blockchain.is_some()
    }
}

/// Blockchain configuration for the oracle
#[derive(Debug, Clone)]
pub struct BlockchainConfig {
    pub rpc_url: String,
    pub private_key: String,
    pub ans_state_manager_address: Address,
}

/// Blockchain integration for the oracle
pub struct BlockchainOracle {
    provider: Provider<Http>,
    wallet: LocalWallet,
    ans_contract: SimpleAnsStateManager<Provider<Http>>,
}

impl BlockchainOracle {
    /// Create a new blockchain oracle
    pub async fn new(config: &BlockchainConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let provider = Provider::<Http>::try_from(&config.rpc_url)?;
        let wallet: LocalWallet = config.private_key.parse()?;
        let wallet = wallet.with_chain_id(31337u64); // Anvil default chain ID

        let ans_contract = SimpleAnsStateManager::new(
            config.ans_state_manager_address,
            Arc::new(provider.clone()),
        );

        Ok(Self {
            provider,
            wallet,
            ans_contract,
        })
    }

    /// Update the ANS state with computed VTI
    pub async fn update_tone(&self, vti_value: u64, suggested_state: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Convert suggested state string to enum
        let state_value = match suggested_state {
            "SAFE" => 0u8,
            "DANGER" => 1u8,
            "SHUTDOWN" => 2u8,
            _ => 0u8, // Default to SAFE for unknown states
        };

        // Create a client with signer
        let client = SignerMiddleware::new(self.provider.clone(), self.wallet.clone());

        // Call updateTone on the contract
        let contract = SimpleAnsStateManager::new(
            self.ans_contract.address(),
            Arc::new(client),
        );

        let tx = contract.update_tone(vti_value.into(), state_value);
        let pending_tx = tx.send().await?;
        let receipt = pending_tx.confirmations(1).await?;

        tracing::info!("Updated ANS tone: VTI={}, State={}, TxHash={:?}",
                      vti_value, suggested_state, receipt.unwrap().transaction_hash);

        Ok(())
    }
}

// Minimal ANS State Manager contract interface
abigen!(
    SimpleAnsStateManager,
    r#"[
        function updateTone(uint256 tone, uint8 suggested) external
        function getCurrentState() external view returns (uint8)
        function currentTone() external view returns (uint256)
    ]"#,
);
