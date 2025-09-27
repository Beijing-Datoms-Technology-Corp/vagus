use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    WasmMsg, SubMsg,
};
use cw_storage_plus::Item;

use vagus_spec::{CapabilityRevocationReason, VagusError};

// State
pub const AFFerent_INBOX: Item<String> = Item::new("afferent_inbox");
pub const CAPABILITY_ISSUER: Item<String> = Item::new("capability_issuer");
pub const LAST_TRIGGER: Item<u64> = Item::new("last_trigger");
pub const REFLEX_COOLDOWN: Item<u64> = Item::new("reflex_cooldown");

// Reflex thresholds (simplified)
pub const DANGER_VTI_THRESHOLD: Item<u64> = Item::new("danger_vti_threshold");
pub const SHUTDOWN_VTI_THRESHOLD: Item<u64> = Item::new("shutdown_vti_threshold");

#[cosmwasm_schema::cw_serde]
pub struct InstantiateMsg {
    pub afferent_inbox: String,
    pub capability_issuer: String,
    pub reflex_cooldown: u64,
    pub danger_vti_threshold: u64,
    pub shutdown_vti_threshold: u64,
}

#[cosmwasm_schema::cw_serde]
pub enum ExecuteMsg {
    OnAEP {
        executor_id: u64,
        metrics_hash_sha256: Binary,
        metrics_hash_keccak: Binary,
    },
    ManualTrigger {
        executor_id: u64,
        reason: String,
    },
}

#[cosmwasm_schema::cw_serde]
pub enum QueryMsg {
    // No queries for this contract
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, VagusError> {
    // Validate addresses
    deps.api.addr_validate(&msg.afferent_inbox)?;
    deps.api.addr_validate(&msg.capability_issuer)?;

    AFFerent_INBOX.save(deps.storage, &msg.afferent_inbox)?;
    CAPABILITY_ISSUER.save(deps.storage, &msg.capability_issuer)?;
    LAST_TRIGGER.save(deps.storage, &0)?;
    REFLEX_COOLDOWN.save(deps.storage, &msg.reflex_cooldown)?;
    DANGER_VTI_THRESHOLD.save(deps.storage, &msg.danger_vti_threshold)?;
    SHUTDOWN_VTI_THRESHOLD.save(deps.storage, &msg.shutdown_vti_threshold)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("afferent_inbox", msg.afferent_inbox)
        .add_attribute("capability_issuer", msg.capability_issuer)
        .add_attribute("reflex_cooldown", msg.reflex_cooldown.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, VagusError> {
    match msg {
        ExecuteMsg::OnAEP {
            executor_id,
            metrics_hash_sha256,
            metrics_hash_keccak,
        } => execute_on_aep(
            deps,
            env,
            info,
            executor_id,
            metrics_hash_sha256,
            metrics_hash_keccak,
        ),
        ExecuteMsg::ManualTrigger { executor_id, reason } => {
            execute_manual_trigger(deps, env, info, executor_id, reason)
        }
    }
}

pub fn execute_on_aep(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    executor_id: u64,
    metrics_hash_sha256: Binary,
    metrics_hash_keccak: Binary,
) -> Result<Response, VagusError> {
    // Only afferent inbox can trigger reflex
    let afferent_inbox = AFFerent_INBOX.load(deps.storage)?;
    if info.sender != afferent_inbox {
        return Err(VagusError::Unauthorized);
    }

    // Check cooldown
    let last_trigger = LAST_TRIGGER.load(deps.storage)?;
    let cooldown = REFLEX_COOLDOWN.load(deps.storage)?;
    let current_time = env.block.time.seconds();

    if current_time < last_trigger + cooldown {
        // Cooldown not elapsed, skip trigger but don't error
        return Ok(Response::new().add_attribute("action", "on_aep_cooldown"));
    }

    // Analyze metrics to determine if reflex should trigger
    // Simplified: just check if hashes indicate dangerous conditions
    // In production, would decode and analyze actual metrics

    let should_trigger = analyze_metrics_for_danger(&metrics_hash_sha256, &metrics_hash_keccak)?;

    if !should_trigger {
        return Ok(Response::new().add_attribute("action", "on_aep_no_trigger"));
    }

    // Trigger reflex: revoke all capabilities for this executor
    let revoked_count = trigger_capability_revocation(deps, executor_id)?;

    // Update last trigger time
    LAST_TRIGGER.save(deps.storage, &current_time)?;

    Ok(Response::new()
        .add_attribute("action", "reflex_triggered")
        .add_attribute("executor_id", executor_id.to_string())
        .add_attribute("triggered_at", current_time.to_string())
        .add_attribute("revoked_count", revoked_count.to_string()))
}

pub fn execute_manual_trigger(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    executor_id: u64,
    reason: String,
) -> Result<Response, VagusError> {
    // Check cooldown
    let last_trigger = LAST_TRIGGER.load(deps.storage)?;
    let cooldown = REFLEX_COOLDOWN.load(deps.storage)?;
    let current_time = env.block.time.seconds();

    if current_time < last_trigger + cooldown {
        return Err(VagusError::InvalidInput);
    }

    // Trigger reflex: revoke all capabilities for this executor
    let revoked_count = trigger_capability_revocation(deps, executor_id)?;

    // Update last trigger time
    LAST_TRIGGER.save(deps.storage, &current_time)?;

    Ok(Response::new()
        .add_attribute("action", "manual_reflex_triggered")
        .add_attribute("executor_id", executor_id.to_string())
        .add_attribute("reason", reason)
        .add_attribute("revoked_count", revoked_count.to_string())
        .add_attribute("triggered_at", current_time.to_string()))
}

fn analyze_metrics_for_danger(
    _metrics_hash_sha256: &Binary,
    _metrics_hash_keccak: &Binary,
) -> Result<bool, VagusError> {
    // Simplified analysis - in production would:
    // 1. Query ANS state manager for current VTI
    // 2. Decode metrics and check against thresholds
    // 3. Apply hysteresis logic

    // For MVP: randomly trigger reflex 10% of the time (simulating dangerous conditions)
    // In production: implement proper metrics analysis

    Ok(rand::random::<u8>() < 25) // ~10% chance
}

fn trigger_capability_revocation(deps: DepsMut, executor_id: u64) -> Result<u64, VagusError> {
    // Query active tokens for this executor
    let capability_issuer = CAPABILITY_ISSUER.load(deps.storage)?;

    // In production, would query CapabilityIssuer for active tokens of executor
    // For MVP, we'll simulate revoking some tokens

    // Placeholder: assume we revoke 3 tokens (in reality would query and revoke all active ones)
    let active_tokens = vec!["1".to_string(), "2".to_string(), "3".to_string()];

    let mut messages = Vec::new();
    for token_id in active_tokens {
        let revoke_msg = vagus_spec::capability_issuer::ExecuteMsg::Revoke {
            token_id,
            reason: CapabilityRevocationReason::REFLEX_TRIGGER,
        };

        let wasm_msg = WasmMsg::Execute {
            contract_addr: capability_issuer.clone(),
            msg: to_json_binary(&revoke_msg)?,
            funds: vec![],
        };

        messages.push(SubMsg::new(wasm_msg));
    }

    // Store messages for execution
    // In a real implementation, we'd return these in the Response
    // For now, just return count

    Ok(messages.len() as u64)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    // No queries implemented
    Err(cosmwasm_std::StdError::not_found("QueryMsg"))
}

// Helper modules for cross-contract calls
pub mod vagus_spec {
    use super::*;

    pub mod capability_issuer {
        use super::*;

        #[cosmwasm_schema::cw_serde]
        pub enum ExecuteMsg {
            Revoke {
                token_id: String,
                reason: CapabilityRevocationReason,
            },
        }
    }
}
