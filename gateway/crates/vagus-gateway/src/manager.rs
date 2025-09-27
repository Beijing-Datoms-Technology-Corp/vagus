//! Vagus Gateway Manager
//!
//! Main gateway implementation that coordinates all components.

use anyhow::Result;
use ethers::types::Address;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn, error};

use crate::cbf::{ControlBarrierFunction, BasicCBF, SafetyConditions};
use crate::collector::TelemetryCollector;
use crate::event_watcher::{EventWatcher, GatewayEvent};
use crate::token_manager::TokenManager;
use vagus_crypto::VagusCrypto;
use vagus_telemetry::{AfferentEvidencePacket, SensorReading, VagalToneIndicator};

/// Configuration for the Vagus Gateway
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub executor_id: u64,
    pub websocket_url: String,
    pub afferent_inbox_address: Address,
    pub ans_state_manager_address: Address,
    pub capability_issuer_address: Address,
    pub reflex_arc_address: Address,
    pub window_duration_ms: u64,
    pub evidence_submission_interval_ms: u64,
}

/// Main Vagus Gateway implementation
pub struct VagusGateway {
    config: GatewayConfig,
    crypto: VagusCrypto,
    token_manager: TokenManager,
    telemetry_collector: TelemetryCollector,
    cbf: Box<dyn ControlBarrierFunction>,
    event_sender: Option<mpsc::UnboundedSender<GatewayEvent>>,
    event_receiver: Option<mpsc::UnboundedReceiver<GatewayEvent>>,
}

impl VagusGateway {
    /// Create a new Vagus Gateway
    pub fn new(config: GatewayConfig, crypto: VagusCrypto) -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let cloned_crypto = crypto.clone();
        let window_duration = config.window_duration_ms;

        Self {
            config,
            crypto,
            token_manager: TokenManager::new(cloned_crypto),
            telemetry_collector: TelemetryCollector::new(window_duration),
            cbf: Box::new(BasicCBF::new()),
            event_sender: Some(event_sender),
            event_receiver: Some(event_receiver),
        }
    }

    /// Start the gateway
    pub async fn start(mut self) -> Result<()> {
        info!("Starting Vagus Gateway for executor {}", self.config.executor_id);

        // Start event watcher
        let event_watcher = EventWatcher::new(
            &self.config.websocket_url,
            self.config.afferent_inbox_address,
            self.config.ans_state_manager_address,
            self.config.capability_issuer_address,
            self.config.reflex_arc_address,
        ).await?;

        let event_sender = self.event_sender.take().unwrap();
        tokio::spawn(async move {
            if let Err(e) = event_watcher.start_watching(event_sender).await {
                error!("Event watcher failed: {:?}", e);
            }
        });

        // Start telemetry collection and evidence submission
        self.start_telemetry_loop().await?;
        self.start_evidence_submission_loop().await?;

        info!("Vagus Gateway started successfully");
        Ok(())
    }

    /// Add sensor reading to telemetry collection
    pub async fn add_sensor_reading(&self, reading: SensorReading) -> Result<()> {
        self.telemetry_collector
            .add_reading(self.config.executor_id, reading)
            .await
    }

    /// Get current VTI
    pub async fn get_current_vti(&self) -> Result<Option<VagalToneIndicator>> {
        self.telemetry_collector
            .compute_vti(self.config.executor_id)
            .await
    }

    /// Check if an action is allowed by the CBF
    pub async fn check_safety_guard(&self, setpoint: &vagus_telemetry::Pose) -> Result<vagus_telemetry::SafetyGuard> {
        // Get current sensor data (simplified - in production this would query actual sensors)
        let sensor_data = crate::cbf::SensorData {
            human_distances: vec![500.0], // Mock data
            temperatures: vec![60.0],
            velocities: vec![1.0],
            jerks: vec![0.5],
            battery_level: Some(75.0),
        };

        self.cbf.guard(setpoint, &sensor_data).await
            .map_err(Into::into)
    }

    /// Start telemetry collection loop
    async fn start_telemetry_loop(&self) -> Result<()> {
        let collector = Arc::new(self.telemetry_collector.clone());
        let _executor_id = self.config.executor_id;

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Cleanup old windows periodically
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                if let Err(e) = collector.cleanup_old_windows(current_time, 30000).await {
                    warn!("Failed to cleanup old windows: {:?}", e);
                }
            }
        });

        Ok(())
    }

    /// Start evidence submission loop
    async fn start_evidence_submission_loop(&self) -> Result<()> {
        let collector = Arc::new(self.telemetry_collector.clone());
        let crypto = self.crypto.clone();
        let executor_id = self.config.executor_id;
        let interval = self.config.evidence_submission_interval_ms;

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(interval)).await;

                if let Err(e) = Self::submit_evidence(&collector, &crypto, executor_id).await {
                    warn!("Failed to submit evidence: {:?}", e);
                }
            }
        });

        Ok(())
    }

    /// Submit afferent evidence to the blockchain
    async fn submit_evidence(
        collector: &TelemetryCollector,
        crypto: &VagusCrypto,
        executor_id: u64,
    ) -> Result<()> {
        // Get current metrics
        let metrics = match collector.get_current_metrics(executor_id).await? {
            Some(m) => m,
            None => return Ok(()), // No data to submit
        };

        // Compute VTI
        let vti = VagalToneIndicator::from_metrics(&metrics);

        // Create state root (simplified - in production this would be a Merkle root)
        let state_root = metrics.hash();

        // Create metrics hash
        let metrics_hash = metrics.hash();

        // Create AEP
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let aep = AfferentEvidencePacket {
            executor_id,
            state_root,
            metrics_hash,
            attestation: None, // TODO: Add signature
            timestamp,
        };

        info!("Submitting AEP for executor {}: VTI={:.3}", executor_id, vti.value);

        // TODO: Submit to blockchain via contract call
        // For now, just log the evidence

        Ok(())
    }

    // TODO: Implement event handling when GatewayEvent types are finalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::types::Address;
    use vagus_crypto::VagusDomain;

    fn create_test_config() -> GatewayConfig {
        GatewayConfig {
            executor_id: 42,
            websocket_url: "ws://localhost:8545".to_string(),
            afferent_inbox_address: Address::random(),
            ans_state_manager_address: Address::random(),
            capability_issuer_address: Address::random(),
            reflex_arc_address: Address::random(),
            window_duration_ms: 1000,
            evidence_submission_interval_ms: 5000,
        }
    }

    fn create_test_crypto() -> VagusCrypto {
        let domain = VagusDomain {
            name: "Vagus".to_string(),
            version: "1".to_string(),
            chain_id: 31337,
            verifying_contract: Address::zero(),
        };
        VagusCrypto::new(domain)
    }

    #[test]
    fn test_gateway_creation() {
        let config = create_test_config();
        let crypto = create_test_crypto();

        let gateway = VagusGateway::new(config, crypto);
        assert_eq!(gateway.config.executor_id, 42);
    }

    #[tokio::test]
    async fn test_sensor_reading() {
        let config = create_test_config();
        let crypto = create_test_crypto();

        let gateway = VagusGateway::new(config, crypto);

        let reading = vagus_telemetry::SensorReading {
            sensor_id: "test".to_string(),
            sensor_type: "human_distance".to_string(),
            value: 300.0,
            unit: "mm".to_string(),
            timestamp: 1000,
        };

        gateway.add_sensor_reading(reading).await.unwrap();

        let vti = gateway.get_current_vti().await.unwrap().unwrap();
        assert!(vti.value >= 0.0 && vti.value <= 1.0);
    }
}
