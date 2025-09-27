//! Vagus Relayer - Cross-chain event synchronization
//!
//! Monitors events on one chain and relays them to another chain with deduplication.

use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{info, warn, error};
use vagus_chain::{ChainClient, ChainClientFactory, ChainConfig, ChainType, Event};

/// CLI arguments
#[derive(Parser)]
#[command(name = "vagus-relayer")]
#[command(about = "Cross-chain event synchronization for Vagus")]
struct Args {
    /// Source chain type (evm or cosmos)
    #[arg(long)]
    source_chain: String,

    /// Source chain RPC URL
    #[arg(long)]
    source_rpc: String,

    /// Target chain type (evm or cosmos)
    #[arg(long)]
    target_chain: String,

    /// Target chain RPC URL
    #[arg(long)]
    target_rpc: String,

    /// Private key for target chain transactions
    #[arg(long, env = "PRIVATE_KEY")]
    private_key: String,

    /// Source contract addresses (contract_name=address)
    #[arg(long, value_parser = parse_contract_address)]
    source_contracts: Vec<(String, String)>,

    /// Target contract addresses (contract_name=address)
    #[arg(long, value_parser = parse_contract_address)]
    target_contracts: Vec<(String, String)>,
}

fn parse_contract_address(s: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = s.split('=').collect();
    if parts.len() == 2 {
        Ok((parts[0].to_string(), parts[1].to_string()))
    } else {
        Err(anyhow::anyhow!("Invalid contract address format"))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("Starting Vagus Relayer");
    info!("Source: {} at {}", args.source_chain, args.source_rpc);
    info!("Target: {} at {}", args.target_chain, args.target_rpc);

    // Parse chain types
    let source_chain_type = parse_chain_type(&args.source_chain)?;
    let target_chain_type = parse_chain_type(&args.target_chain)?;

    // Create chain configurations
    let source_config = create_chain_config(
        source_chain_type,
        args.source_rpc,
        HashMap::new(), // Source doesn't need private key
        args.source_contracts,
    );

    let target_config = create_chain_config(
        target_chain_type,
        args.target_rpc,
        Some(args.private_key),
        args.target_contracts,
    );

    // Create chain clients
    let source_client = ChainClientFactory::create_client(source_config).await?;
    let target_client = ChainClientFactory::create_client(target_config).await?;

    info!("Chain clients initialized successfully");

    // Create event processing channel
    let (event_tx, event_rx) = mpsc::unbounded_channel::<Event>();

    // Start event subscription on source chain
    let source_client_clone = source_client.clone();
    tokio::spawn(async move {
        if let Err(e) = subscribe_to_events(source_client_clone, event_tx).await {
            error!("Event subscription failed: {}", e);
        }
    });

    // Start event processing
    process_events(target_client, event_rx).await?;

    Ok(())
}

fn parse_chain_type(chain_str: &str) -> Result<ChainType> {
    match chain_str.to_lowercase().as_str() {
        "evm" => Ok(ChainType::EVM),
        "cosmos" => Ok(ChainType::Cosmos),
        _ => Err(anyhow::anyhow!("Unsupported chain type: {}", chain_str)),
    }
}

fn create_chain_config(
    chain_type: ChainType,
    rpc_url: String,
    private_key: Option<String>,
    contracts: Vec<(String, String)>,
) -> ChainConfig {
    let mut contract_addresses = HashMap::new();
    for (name, addr) in contracts {
        contract_addresses.insert(name, addr);
    }

    ChainConfig {
        chain_type,
        rpc_url,
        contract_addresses,
        private_key,
    }
}

async fn subscribe_to_events(
    client: Box<dyn ChainClient>,
    event_tx: mpsc::UnboundedSender<Event>,
) -> Result<()> {
    info!("Starting event subscription");

    client.subscribe_events(move |event: Event| {
        if let Err(e) = event_tx.send(event) {
            warn!("Failed to send event to processing queue: {}", e);
        }
    }).await?;

    Ok(())
}

async fn process_events(
    target_client: Box<dyn ChainClient>,
    mut event_rx: mpsc::UnboundedReceiver<Event>,
) -> Result<()> {
    info!("Starting event processing");

    while let Some(event) = event_rx.recv().await {
        if let Err(e) = process_event(&*target_client, &event).await {
            error!("Failed to process event {:?}: {}", event, e);
            // Continue processing other events
        }
    }

    Ok(())
}

async fn process_event(target_client: &dyn ChainClient, event: &Event) -> Result<()> {
    match event.event_name.as_str() {
        "CapabilityIssued" => {
            handle_capability_issued(target_client, event).await
        }
        "CapabilityRevoked" => {
            handle_capability_revoked(target_client, event).await
        }
        "VagalToneUpdated" => {
            handle_tone_updated(target_client, event).await
        }
        "AEPPosted" => {
            handle_aep_posted(target_client, event).await
        }
        "ReflexTriggered" => {
            handle_reflex_triggered(target_client, event).await
        }
        _ => {
            // Ignore unknown events
            Ok(())
        }
    }
}

async fn handle_capability_issued(_target_client: &dyn ChainClient, event: &Event) -> Result<()> {
    info!("Processing CapabilityIssued event: {:?}", event);
    // TODO: Implement cross-chain capability synchronization
    // This would involve checking if the capability already exists on target chain
    // and creating a corresponding entry if not
    Ok(())
}

async fn handle_capability_revoked(_target_client: &dyn ChainClient, event: &Event) -> Result<()> {
    info!("Processing CapabilityRevoked event: {:?}", event);
    // TODO: Implement cross-chain capability revocation
    // This would involve revoking the corresponding capability on target chain
    Ok(())
}

async fn handle_tone_updated(target_client: &dyn ChainClient, event: &Event) -> Result<()> {
    info!("Processing VagalToneUpdated event: {:?}", event);

    // Extract VTI value and suggested state from event
    if let (Some(tone_str), Some(state_str)) = (
        event.data.get("tone"),
        event.data.get("state"),
    ) {
        if let (Some(tone), Some(state)) = (
            tone_str.as_str().and_then(|s| s.parse::<u64>().ok()),
            state_str.as_str(),
        ) {
            let ans_state = match state {
                "SAFE" => vagus_chain::ANSState::SAFE,
                "DANGER" => vagus_chain::ANSState::DANGER,
                "SHUTDOWN" => vagus_chain::ANSState::SHUTDOWN,
                _ => {
                    warn!("Unknown ANS state: {}", state);
                    return Ok(());
                }
            };

            // Update tone on target chain
            target_client.update_tone(tone, ans_state).await?;
            info!("Synchronized tone update: {} -> {:?}", tone, ans_state);
        }
    }

    Ok(())
}

async fn handle_aep_posted(_target_client: &dyn ChainClient, event: &Event) -> Result<()> {
    info!("Processing AEPPosted event: {:?}", event);
    // TODO: Implement cross-chain AEP synchronization
    // This would involve posting the same AEP data to target chain
    Ok(())
}

async fn handle_reflex_triggered(_target_client: &dyn ChainClient, event: &Event) -> Result<()> {
    info!("Processing ReflexTriggered event: {:?}", event);
    // TODO: Implement cross-chain reflex synchronization
    // This would involve triggering reflex actions on target chain
    Ok(())
}
