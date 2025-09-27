use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    WasmMsg, SubMsg,
};
use cw_storage_plus::Item;

use vagus_spec::{ANSState, Guard, VagusError, MAX_DURATION_MS, MAX_ENERGY_J};

// State
pub const ANS_STATE_MANAGER: Item<String> = Item::new("ans_state_manager");
pub const CAPABILITY_ISSUER: Item<String> = Item::new("capability_issuer");

#[cosmwasm_schema::cw_serde]
pub struct InstantiateMsg {
    pub ans_state_manager: String,
    pub capability_issuer: String,
}

#[cosmwasm_schema::cw_serde]
pub enum ExecuteMsg {
    IssueWithBrake {
        intent_executor_id: u64,
        intent_action_id: Binary,
        intent_params: Binary,
        intent_envelope_hash: Binary,
        intent_pre_state_root: Binary,
        intent_not_before: u64,
        intent_not_after: u64,
        intent_max_duration_ms: u64,
        intent_max_energy_j: u64,
        intent_planner: String,
        intent_nonce: u64,
        scaled_limits_hash: Binary,
        expires_at: u64,
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
    deps.api.addr_validate(&msg.ans_state_manager)?;
    deps.api.addr_validate(&msg.capability_issuer)?;

    ANS_STATE_MANAGER.save(deps.storage, &msg.ans_state_manager)?;
    CAPABILITY_ISSUER.save(deps.storage, &msg.capability_issuer)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("ans_state_manager", msg.ans_state_manager)
        .add_attribute("capability_issuer", msg.capability_issuer))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, VagusError> {
    match msg {
        ExecuteMsg::IssueWithBrake {
            intent_executor_id,
            intent_action_id,
            intent_params,
            intent_envelope_hash,
            intent_pre_state_root,
            intent_not_before,
            intent_not_after,
            intent_max_duration_ms,
            intent_max_energy_j,
            intent_planner,
            intent_nonce,
            scaled_limits_hash,
            expires_at,
        } => execute_issue_with_brake(
            deps,
            env,
            info,
            intent_executor_id,
            intent_action_id,
            intent_params,
            intent_envelope_hash,
            intent_pre_state_root,
            intent_not_before,
            intent_not_after,
            intent_max_duration_ms,
            intent_max_energy_j,
            intent_planner,
            intent_nonce,
            scaled_limits_hash,
            expires_at,
        ),
    }
}

pub fn execute_issue_with_brake(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    intent_executor_id: u64,
    intent_action_id: Binary,
    intent_params: Binary,
    intent_envelope_hash: Binary,
    intent_pre_state_root: Binary,
    intent_not_before: u64,
    intent_not_after: u64,
    intent_max_duration_ms: u64,
    intent_max_energy_j: u64,
    intent_planner: String,
    intent_nonce: u64,
    scaled_limits_hash: Binary,
    expires_at: u64,
) -> Result<Response, VagusError> {
    // Query ANS state manager for guard
    let ans_manager = ANS_STATE_MANAGER.load(deps.storage)?;
    let guard: Guard = deps.querier.query_wasm_smart(
        &ans_manager,
        &vagus_spec::ans_state_manager::QueryMsg::GuardFor {
            action_id: intent_action_id.clone(),
        },
    )?;

    // Check if execution is blocked
    if !guard.allowed {
        return Err(VagusError::ANSBlocked);
    }

    // Apply scaling to brakeable parameters
    let scaled_params = apply_scaling(&intent_params, guard.scalingFactor.u128() as u64)?;

    // Validate scaled limits against intent constraints
    validate_scaled_limits(
        &scaled_params,
        intent_max_duration_ms,
        intent_max_energy_j,
        guard.scalingFactor.u128() as u64,
    )?;

    // Issue capability token via CapabilityIssuer
    let capability_issuer = CAPABILITY_ISSUER.load(deps.storage)?;

    let issue_msg = vagus_spec::capability_issuer::ExecuteMsg::Issue {
        intent_executor_id,
        intent_action_id,
        intent_params: scaled_params,
        intent_envelope_hash,
        intent_pre_state_root,
        intent_not_before,
        intent_not_after,
        intent_max_duration_ms,
        intent_max_energy_j,
        intent_planner: intent_planner.clone(),
        intent_nonce,
        scaled_limits_hash,
        expires_at,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: capability_issuer,
        msg: to_json_binary(&issue_msg)?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "issue_with_brake")
        .add_attribute("executor_id", intent_executor_id.to_string())
        .add_attribute("planner", intent_planner)
        .add_attribute("scaling_factor", guard.scalingFactor.to_string())
        .add_attribute("allowed", guard.allowed.to_string()))
}

fn apply_scaling(params: &Binary, scaling_factor: u64) -> Result<Binary, VagusError> {
    // Simplified scaling - in production this would parse and scale specific fields
    // For MVP, just return original params (assume scaling is handled elsewhere)
    // Real implementation would need to parse CBOR/ABI encoded params and scale brakeable fields

    // TODO: Implement actual parameter scaling based on action schema
    // For now, assume params are already properly scaled or scaling factor is 100%

    Ok(params.clone())
}

fn validate_scaled_limits(
    _scaled_params: &Binary,
    max_duration_ms: u64,
    max_energy_j: u64,
    scaling_factor: u64,
) -> Result<(), VagusError> {
    // Check duration limit
    let scaled_duration = (max_duration_ms as u128 * scaling_factor as u128) / 10000;
    if scaled_duration > MAX_DURATION_MS as u128 {
        return Err(VagusError::ANSLimitExceeded);
    }

    // Check energy limit
    let scaled_energy = (max_energy_j as u128 * scaling_factor as u128) / 10000;
    if scaled_energy > MAX_ENERGY_J as u128 {
        return Err(VagusError::ANSLimitExceeded);
    }

    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    // No queries implemented
    Err(cosmwasm_std::StdError::not_found("QueryMsg"))
}

// Helper modules for cross-contract calls
pub mod vagus_spec {
    use super::*;

    pub mod ans_state_manager {
        use super::*;

        #[cosmwasm_schema::cw_serde]
        pub enum QueryMsg {
            GuardFor { action_id: Binary },
        }
    }

    pub mod capability_issuer {
        use super::*;

        #[cosmwasm_schema::cw_serde]
        pub enum ExecuteMsg {
            Issue {
                intent_executor_id: u64,
                intent_action_id: Binary,
                intent_params: Binary,
                intent_envelope_hash: Binary,
                intent_pre_state_root: Binary,
                intent_not_before: u64,
                intent_not_after: u64,
                intent_max_duration_ms: u64,
                intent_max_energy_j: u64,
                intent_planner: String,
                intent_nonce: u64,
                scaled_limits_hash: Binary,
                expires_at: u64,
            },
        }
    }
}
