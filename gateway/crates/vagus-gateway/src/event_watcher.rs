//! Event Watcher
//!
//! Monitors blockchain events related to capability tokens and ANS state changes.

use anyhow::Result;
use ethers::types::Address;
use tokio::sync::mpsc;

/// Events that the gateway needs to monitor
#[derive(Debug, Clone)]
pub enum GatewayEvent {
    /// New capability issued
    CapabilityIssued {
        token_id: ethers::types::U256,
        executor_id: ethers::types::U256,
        action_id: [u8; 32],
        params_hash: [u8; 32],
        expires_at: u64,
    },
    /// Capability revoked
    CapabilityRevoked {
        token_id: ethers::types::U256,
        reason: u8,
    },
    /// Afferent evidence posted
    AepPosted {
        executor_id: ethers::types::U256,
        state_root: [u8; 32],
        metrics_hash: [u8; 32],
    },
    /// Vagal tone updated
    VagalToneUpdated {
        tone: ethers::types::U256,
        state: u8,
    },
    /// Reflex triggered
    ReflexTriggered {
        executor_id: ethers::types::U256,
        reason: [u8; 32],
        revoked_tokens: Vec<ethers::types::U256>,
    },
}

/// Event watcher that monitors blockchain events
pub struct EventWatcher {
    afferent_inbox_address: Address,
    ans_state_manager_address: Address,
    capability_issuer_address: Address,
    reflex_arc_address: Address,
}

impl EventWatcher {
    /// Create a new event watcher
    pub async fn new(
        _ws_url: &str,
        afferent_inbox_address: Address,
        ans_state_manager_address: Address,
        capability_issuer_address: Address,
        reflex_arc_address: Address,
    ) -> Result<Self> {
        // Note: In production, we would connect to WebSocket here
        // For MVP, this is a placeholder that doesn't actually connect

        Ok(Self {
            afferent_inbox_address,
            ans_state_manager_address,
            capability_issuer_address,
            reflex_arc_address,
        })
    }

    /// Start watching events and send them through the channel
    pub async fn start_watching(
        self,
        _event_sender: mpsc::UnboundedSender<GatewayEvent>,
    ) -> Result<()> {
        // TODO: Implement actual event watching with ethers WebSocket provider
        // For MVP, this is a placeholder that just runs indefinitely

        // In production, this would:
        // 1. Connect to WebSocket
        // 2. Set up event filters for all relevant contracts
        // 3. Parse incoming events and send them through the channel

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            // TODO: Check for new events and send them
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::types::Address;

    #[tokio::test]
    async fn test_event_watcher_creation() {
        let watcher = EventWatcher::new(
            "ws://localhost:8545",
            Address::zero(),
            Address::zero(),
            Address::zero(),
            Address::zero(),
        )
        .await
        .unwrap();

        assert_eq!(watcher.afferent_inbox_address, Address::zero());
    }
}
