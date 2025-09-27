//! Vagus Chain Client Abstraction
//!
//! Provides unified interface for interacting with both EVM and CosmWasm chains.
//! Supports submitting AEP, issuing capabilities, revoking tokens, and subscribing to events.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use thiserror::Error;

pub use vagus_telemetry::{AfferentEvidencePacket, Intent, TokenMeta};
pub use vagus_spec::{ANSState, Guard, VagusError};

/// Unified chain client trait
#[async_trait::async_trait]
pub trait ChainClient: Send + Sync {
    /// Submit afferent evidence packet
    async fn submit_aep(&self, aep: &AfferentEvidencePacket) -> Result<String>;

    /// Issue capability with brake (scaled parameters)
    async fn issue_with_brake(
        &self,
        intent: &Intent,
        scaled_limits_hash: &[u8; 32],
        expires_at: u64,
    ) -> Result<String>;

    /// Revoke capability token
    async fn revoke_capability(&self, token_id: &str, reason: u8) -> Result<()>;

    /// Subscribe to chain events
    async fn subscribe_events<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(Event) + Send + Sync + 'static;

    /// Get current ANS guard for action
    async fn get_guard(&self, action_id: &[u8; 32]) -> Result<Guard>;

    /// Get current ANS state
    async fn get_ans_state(&self) -> Result<ANSState>;

    /// Update ANS tone and state
    async fn update_tone(&self, vti: u64, suggested_state: ANSState) -> Result<()>;
}

/// Chain types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainType {
    EVM,
    Cosmos,
}

/// Configuration for chain clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_type: ChainType,
    pub rpc_url: String,
    pub contract_addresses: HashMap<String, String>,
    pub private_key: Option<String>,
}

/// Chain client factory
pub struct ChainClientFactory;

impl ChainClientFactory {
    pub async fn create_client(config: ChainConfig) -> Result<Box<dyn ChainClient>> {
        match config.chain_type {
            ChainType::EVM => {
                #[cfg(feature = "evm")]
                {
                    let client = EVMClient::new(config).await?;
                    Ok(Box::new(client))
                }
                #[cfg(not(feature = "evm"))]
                {
                    Err(anyhow::anyhow!("EVM support not compiled in"))
                }
            }
            ChainType::Cosmos => {
                #[cfg(feature = "cosmos")]
                {
                    let client = CosmosClient::new(config).await?;
                    Ok(Box::new(client))
                }
                #[cfg(not(feature = "cosmos"))]
                {
                    Err(anyhow::anyhow!("Cosmos support not compiled in"))
                }
            }
        }
    }
}

/// Unified event representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub chain_type: ChainType,
    pub contract_address: String,
    pub event_name: String,
    pub topics: Vec<String>,
    pub data: HashMap<String, serde_json::Value>,
    pub block_number: u64,
    pub transaction_hash: String,
    pub log_index: u64,
}

/// EVM client implementation
#[cfg(feature = "evm")]
pub mod evm {
    use super::*;
    use ethers::{
        providers::{Provider, Ws},
        signers::{LocalWallet, Signer},
        middleware::SignerMiddleware,
        contract::Contract,
        types::{Address, U256, H256},
    };

    pub struct EVMClient {
        provider: SignerMiddleware<Provider<Ws>, LocalWallet>,
        contract_addresses: HashMap<String, Address>,
    }

    impl EVMClient {
        pub async fn new(config: ChainConfig) -> Result<Self> {
            let provider = Provider::<Ws>::connect(&config.rpc_url).await?;
            let wallet = config.private_key
                .ok_or_else(|| anyhow::anyhow!("Private key required for EVM client"))?
                .parse::<LocalWallet>()?;

            let provider = SignerMiddleware::new(provider, wallet);

            let mut contract_addresses = HashMap::new();
            for (name, addr_str) in config.contract_addresses {
                let addr: Address = addr_str.parse()?;
                contract_addresses.insert(name, addr);
            }

            Ok(Self {
                provider,
                contract_addresses,
            })
        }
    }

    #[async_trait::async_trait]
    impl ChainClient for EVMClient {
        async fn submit_aep(&self, aep: &AfferentEvidencePacket) -> Result<String> {
            // Implementation would call AfferentInbox.postAEP
            todo!("Implement EVM AEP submission")
        }

        async fn issue_with_brake(
            &self,
            intent: &Intent,
            scaled_limits_hash: &[u8; 32],
            expires_at: u64,
        ) -> Result<String> {
            // Implementation would call VagalBrake.issueWithBrake
            todo!("Implement EVM capability issuance")
        }

        async fn revoke_capability(&self, token_id: &str, reason: u8) -> Result<()> {
            // Implementation would call CapabilityIssuer.revoke
            todo!("Implement EVM capability revocation")
        }

        async fn subscribe_events<F>(&self, _callback: F) -> Result<()>
        where
            F: Fn(Event) + Send + Sync + 'static,
        {
            // Implementation would subscribe to contract events
            todo!("Implement EVM event subscription")
        }

        async fn get_guard(&self, action_id: &[u8; 32]) -> Result<Guard> {
            // Implementation would call ANSStateManager.guardFor
            todo!("Implement EVM guard query")
        }

        async fn get_ans_state(&self) -> Result<ANSState> {
            // Implementation would query ANSStateManager.currentState
            todo!("Implement EVM ANS state query")
        }

        async fn update_tone(&self, vti: u64, suggested_state: ANSState) -> Result<()> {
            // Implementation would call ANSStateManager.updateTone
            todo!("Implement EVM tone update")
        }
    }
}

/// Cosmos client implementation
#[cfg(feature = "cosmos")]
pub mod cosmos {
    use super::*;
    use cosmrs::{
        tx::{Msg, SignDoc, SignerInfo},
        rpc::HttpClient,
        crypto::secp256k1::SigningKey,
        AccountId,
    };
    use tendermint_rpc::{Client, WebSocketClient};

    pub struct CosmosClient {
        rpc_client: HttpClient,
        ws_client: WebSocketClient,
        signer: SigningKey,
        account_id: AccountId,
        contract_addresses: HashMap<String, String>,
    }

    impl CosmosClient {
        pub async fn new(config: ChainConfig) -> Result<Self> {
            let rpc_client = HttpClient::new(&config.rpc_url)?;
            let (ws_client, _) = WebSocketClient::new(&config.rpc_url).await?;

            let signer = config.private_key
                .ok_or_else(|| anyhow::anyhow!("Private key required for Cosmos client"))?
                .parse::<SigningKey>()?;

            let account_id = signer.public_key().account_id("cosmos")?;

            Ok(Self {
                rpc_client,
                ws_client,
                signer,
                account_id,
                contract_addresses: config.contract_addresses,
            })
        }
    }

    #[async_trait::async_trait]
    impl ChainClient for CosmosClient {
        async fn submit_aep(&self, aep: &AfferentEvidencePacket) -> Result<String> {
            // Implementation would submit PostAEP message to AfferentInbox contract
            todo!("Implement Cosmos AEP submission")
        }

        async fn issue_with_brake(
            &self,
            intent: &Intent,
            scaled_limits_hash: &[u8; 32],
            expires_at: u64,
        ) -> Result<String> {
            // Implementation would submit IssueWithBrake message to VagalBrake contract
            todo!("Implement Cosmos capability issuance")
        }

        async fn revoke_capability(&self, token_id: &str, reason: u8) -> Result<()> {
            // Implementation would submit Revoke message to CapabilityIssuer contract
            todo!("Implement Cosmos capability revocation")
        }

        async fn subscribe_events<F>(&self, _callback: F) -> Result<()>
        where
            F: Fn(Event) + Send + Sync + 'static,
        {
            // Implementation would subscribe to contract events via WebSocket
            todo!("Implement Cosmos event subscription")
        }

        async fn get_guard(&self, action_id: &[u8; 32]) -> Result<Guard> {
            // Implementation would query ANSStateManager contract
            todo!("Implement Cosmos guard query")
        }

        async fn get_ans_state(&self) -> Result<ANSState> {
            // Implementation would query ANSStateManager contract
            todo!("Implement Cosmos ANS state query")
        }

        async fn update_tone(&self, vti: u64, suggested_state: ANSState) -> Result<()> {
            // Implementation would submit UpdateTone message to ANSStateManager contract
            todo!("Implement Cosmos tone update")
        }
    }
}

// Re-export clients for easier importing
#[cfg(feature = "evm")]
pub use evm::EVMClient;

#[cfg(feature = "cosmos")]
pub use cosmos::CosmosClient;

/// Error types
#[derive(Error, Debug)]
pub enum ChainError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Contract error: {0}")]
    Contract(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),
}
