use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint256, Timestamp,
};
use cw_storage_plus::{Item, Map};
use cw721_base::Cw721Contract;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use vagus_spec::{CapabilityRevocationReason, TokenMeta, VagusError};

// State
pub const NEXT_TOKEN_ID: Item<u64> = Item::new("next_token_id");
pub const AUTHORIZED_EXECUTORS: Item<HashSet<String>> = Item::new("authorized_executors");
pub const REFLEX_ARC: Item<String> = Item::new("reflex_arc");

// Token metadata storage (simplified cw721)
pub const TOKENS: Map<String, TokenMeta> = Map::new("tokens"); // token_id -> metadata
pub const OWNERS: Map<String, String> = Map::new("owners"); // token_id -> owner
pub const OWNED_TOKENS: Map<(String, String), ()> = Map::new("owned_tokens"); // (owner, token_id) -> ()

// Governance
pub const VAGUS_DAO: Item<String> = Item::new("vagus_dao");

// Rate limiter and circuit breaker state
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RateLimitConfig {
    pub window_size: u64,    // Time window in seconds
    pub max_requests: u64,   // Maximum requests per window
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CircuitBreaker {
    pub state: CircuitState,
    pub failure_count: u64,
    pub last_failure_time: u64,
    pub success_count: u64,
    pub next_attempt_time: u64,
}

pub const GLOBAL_RATE_LIMIT: Item<RateLimitConfig> = Item::new("global_rate_limit");
pub const CIRCUIT_BREAKER_THRESHOLD: Item<u64> = Item::new("circuit_breaker_threshold");
pub const CIRCUIT_BREAKER_TIMEOUT: Item<u64> = Item::new("circuit_breaker_timeout");
pub const CIRCUIT_BREAKER_RECOVERY: Item<u64> = Item::new("circuit_breaker_recovery");

// Per (executor_id, action_id) rate limiting and circuit breaker state
pub const RATE_LIMIT_WINDOWS: Map<String, Vec<u64>> = Map::new("rate_limit_windows");
pub const CIRCUIT_BREAKERS: Map<String, CircuitBreaker> = Map::new("circuit_breakers");

// Emergency pause state
pub const EMERGENCY_PAUSED: Item<bool> = Item::new("emergency_paused");

#[cosmwasm_schema::cw_serde]
pub struct InstantiateMsg {
    pub authorized_executors: Vec<String>,
    pub reflex_arc: Option<String>,
    pub vagus_dao: String,
    pub rate_limit_window_size: Option<u64>,
    pub rate_limit_max_requests: Option<u64>,
    pub circuit_breaker_threshold: Option<u64>,
    pub circuit_breaker_timeout: Option<u64>,
    pub circuit_breaker_recovery: Option<u64>,
}

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
    Revoke {
        token_id: String,
        reason: CapabilityRevocationReason,
    },
    // Governance operations
    SetReflexArc {
        reflex_arc: String,
    },
    SetRateLimit {
        window_size: u64,
        max_requests: u64,
    },
    SetCircuitBreakerParams {
        threshold: u64,
        timeout: u64,
        recovery: u64,
    },
    EmergencyPause {},
    EmergencyUnpause {},
}

#[cosmwasm_schema::cw_serde]
pub enum QueryMsg {
    IsValid { token_id: String },
    ActiveTokensOf { executor_id: u64 },
    TokenInfo { token_id: String },
}

#[cosmwasm_schema::cw_serde]
pub struct IsValidResponse {
    pub valid: bool,
}

#[cosmwasm_schema::cw_serde]
pub struct ActiveTokensOfResponse {
    pub token_ids: Vec<String>,
}

#[cosmwasm_schema::cw_serde]
pub struct TokenInfoResponse {
    pub token: Option<TokenMeta>,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, VagusError> {
    // Validate addresses
    let mut executors = HashSet::new();
    for executor in msg.authorized_executors {
        deps.api.addr_validate(&executor)?;
        executors.insert(executor);
    }

    AUTHORIZED_EXECUTORS.save(deps.storage, &executors)?;
    NEXT_TOKEN_ID.save(deps.storage, &1)?;

    // Initialize governance
    deps.api.addr_validate(&msg.vagus_dao)?;
    VAGUS_DAO.save(deps.storage, &msg.vagus_dao)?;

    if let Some(reflex_arc) = msg.reflex_arc {
        deps.api.addr_validate(&reflex_arc)?;
        REFLEX_ARC.save(deps.storage, &reflex_arc)?;
    }

    // Initialize rate limiter defaults
    let rate_limit = RateLimitConfig {
        window_size: msg.rate_limit_window_size.unwrap_or(3600), // 1 hour default
        max_requests: msg.rate_limit_max_requests.unwrap_or(100), // 100 requests/hour default
    };
    GLOBAL_RATE_LIMIT.save(deps.storage, &rate_limit)?;

    // Initialize circuit breaker defaults
    CIRCUIT_BREAKER_THRESHOLD.save(deps.storage, &msg.circuit_breaker_threshold.unwrap_or(5))?;
    CIRCUIT_BREAKER_TIMEOUT.save(deps.storage, &msg.circuit_breaker_timeout.unwrap_or(300))?; // 5 minutes
    CIRCUIT_BREAKER_RECOVERY.save(deps.storage, &msg.circuit_breaker_recovery.unwrap_or(3))?;

    // Initialize emergency pause state
    EMERGENCY_PAUSED.save(deps.storage, &false)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("executor_count", executors.len().to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, VagusError> {
    // Check emergency pause
    if EMERGENCY_PAUSED.load(deps.storage)? {
        return Err(VagusError::ContractPaused);
    }

    match msg {
        ExecuteMsg::Issue {
            intent_executor_id,
            intent_action_id,
            intent_params: _,
            intent_envelope_hash: _,
            intent_pre_state_root: _,
            intent_not_before,
            intent_not_after,
            intent_max_duration_ms: _,
            intent_max_energy_j: _,
            intent_planner,
            intent_nonce: _,
            scaled_limits_hash,
            expires_at,
        } => execute_issue(
            deps,
            env,
            info,
            intent_executor_id,
            intent_action_id,
            intent_not_before,
            intent_not_after,
            intent_planner,
            scaled_limits_hash,
            expires_at,
        ),
        ExecuteMsg::Revoke { token_id, reason } => {
            execute_revoke(deps, env, info, token_id, reason)
        }
        ExecuteMsg::SetReflexArc { reflex_arc } => {
            execute_set_reflex_arc(deps, info, reflex_arc)
        }
        ExecuteMsg::SetRateLimit { window_size, max_requests } => {
            execute_set_rate_limit(deps, info, window_size, max_requests)
        }
        ExecuteMsg::SetCircuitBreakerParams { threshold, timeout, recovery } => {
            execute_set_circuit_breaker_params(deps, info, threshold, timeout, recovery)
        }
        ExecuteMsg::EmergencyPause {} => {
            execute_emergency_pause(deps, info)
        }
        ExecuteMsg::EmergencyUnpause {} => {
            execute_emergency_unpause(deps, info)
        }
    }
}

pub fn execute_issue(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    executor_id: u64,
    action_id: Binary,
    not_before: u64,
    not_after: u64,
    planner: String,
    scaled_limits_hash: Binary,
    expires_at: u64,
) -> Result<Response, VagusError> {
    // Check authorization - sender must be authorized executor (ER3)
    let executors = AUTHORIZED_EXECUTORS.load(deps.storage)?;
    if !executors.contains(&info.sender.to_string()) {
        return Err(VagusError::Unauthorized);
    }

    // Validate time constraints (closed interval [not_before, not_after])
    let current_time = env.block.time.seconds();
    if current_time < not_before || current_time > not_after {
        return Err(VagusError::IntentExpired);
    }

    // ER7: Check circuit breaker first
    let key = format!("{}_{}", executor_id, hex::encode(&action_id));
    check_circuit_breaker(deps.storage, &key, current_time)?;

    // ER7: Check rate limits (sliding window)
    check_rate_limit(deps.storage, &key, current_time)?;

    // Generate token ID
    let token_id_num = NEXT_TOKEN_ID.load(deps.storage)?;
    let token_id = token_id_num.to_string();
    NEXT_TOKEN_ID.save(deps.storage, &(token_id_num + 1))?;

    // Create token metadata
    let token_meta = TokenMeta {
        tokenId: token_id_num.into(),
        executorId: executor_id.into(),
        actionId: action_id,
        scaledLimitsHash: scaled_limits_hash,
        issuedAt: current_time.into(),
        expiresAt: expires_at.into(),
        revoked: false,
        revokedAt: 0u64.into(),
    };

    // Store token data
    TOKENS.save(deps.storage, token_id.clone(), &token_meta)?;
    OWNERS.save(deps.storage, token_id.clone(), &planner)?;
    OWNED_TOKENS.save(deps.storage, (planner.clone(), token_id.clone()), &())?;

    // Record circuit breaker success
    record_circuit_success(deps.storage, &key)?;

    Ok(Response::new()
        .add_attribute("action", "issue")
        .add_attribute("token_id", token_id)
        .add_attribute("executor_id", executor_id.to_string())
        .add_attribute("planner", planner)
        .add_attribute("issued_at", current_time.to_string())
        .add_attribute("expires_at", expires_at.to_string()))
}

pub fn execute_revoke(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    reason: CapabilityRevocationReason,
) -> Result<Response, VagusError> {
    // Check if token exists
    let mut token = TOKENS.load(deps.storage, token_id.clone())?;

    if token.revoked {
        return Err(VagusError::TokenAlreadyRevoked);
    }

    // Check authorization - only owner or reflex arc can revoke
    let owner = OWNERS.load(deps.storage, token_id.clone())?;
    let sender = info.sender.to_string();

    let reflex_arc = REFLEX_ARC.may_load(deps.storage)?;
    let is_authorized = owner == sender || reflex_arc.as_ref() == Some(&sender);

    if !is_authorized {
        return Err(VagusError::UnauthorizedRevocation);
    }

    // Revoke token
    let current_time = env.block.time.seconds();
    token.revoked = true;
    token.revokedAt = current_time.into();

    TOKENS.save(deps.storage, token_id.clone(), &token)?;

    Ok(Response::new()
        .add_attribute("action", "revoke")
        .add_attribute("token_id", token_id)
        .add_attribute("reason", format!("{:?}", reason))
        .add_attribute("revoked_at", current_time.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IsValid { token_id } => to_json_binary(&query_is_valid(deps, _env, token_id)?),
        QueryMsg::ActiveTokensOf { executor_id } => {
            to_json_binary(&query_active_tokens_of(deps, _env, executor_id)?)
        }
        QueryMsg::TokenInfo { token_id } => to_json_binary(&query_token_info(deps, token_id)?),
    }
}

fn query_is_valid(deps: Deps, env: Env, token_id: String) -> StdResult<IsValidResponse> {
    let token = match TOKENS.may_load(deps.storage, token_id)? {
        Some(t) => t,
        None => return Ok(IsValidResponse { valid: false }),
    };

    // Check if expired or revoked
    let valid = !token.revoked && token.expiresAt > env.block.time.seconds().into();

    Ok(IsValidResponse { valid })
}

fn query_active_tokens_of(deps: Deps, env: Env, executor_id: u64) -> StdResult<ActiveTokensOfResponse> {
    // Simplified - in production would use more efficient indexing
    let mut active_tokens = Vec::new();
    let current_time = env.block.time.seconds();

    // This is inefficient for production - would need proper indexing
    // For MVP, we'll iterate through all tokens (assuming small number)
    // In production, maintain separate index: executor_id -> [token_ids]

    // Placeholder: return empty for now
    Ok(ActiveTokensOfResponse {
        token_ids: active_tokens,
    })
}

fn query_token_info(deps: Deps, token_id: String) -> StdResult<TokenInfoResponse> {
    let token = TOKENS.may_load(deps.storage, token_id)?;
    Ok(TokenInfoResponse { token })
}

// Helper functions for rate limiting and circuit breaker

fn check_circuit_breaker(
    storage: &mut dyn cosmwasm_std::Storage,
    key: &str,
    current_time: u64,
) -> Result<(), VagusError> {
    let mut cb = CIRCUIT_BREAKERS
        .may_load(storage, key.to_string())?
        .unwrap_or(CircuitBreaker {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure_time: 0,
            success_count: 0,
            next_attempt_time: 0,
        });

    if matches!(cb.state, CircuitState::Open) {
        if current_time < cb.next_attempt_time {
            return Err(VagusError::CircuitBreakerOpen);
        }
        // Move to half-open state
        cb.state = CircuitState::HalfOpen;
        cb.success_count = 0;
        CIRCUIT_BREAKERS.save(storage, key.to_string(), &cb)?;
    }

    Ok(())
}

fn check_rate_limit(
    storage: &mut dyn cosmwasm_std::Storage,
    key: &str,
    current_time: u64,
) -> Result<(), VagusError> {
    let rate_limit = GLOBAL_RATE_LIMIT.load(storage)?;
    let mut windows = RATE_LIMIT_WINDOWS
        .may_load(storage, key.to_string())?
        .unwrap_or_default();

    // Remove timestamps outside the window
    let window_start = current_time.saturating_sub(rate_limit.window_size);
    windows.retain(|&timestamp| timestamp > window_start);

    // Check if we're over the limit
    if windows.len() >= rate_limit.max_requests as usize {
        return Err(VagusError::RateLimited);
    }

    // Add current timestamp
    windows.push(current_time);
    RATE_LIMIT_WINDOWS.save(storage, key.to_string(), &windows)?;

    Ok(())
}

fn record_circuit_success(
    storage: &mut dyn cosmwasm_std::Storage,
    key: &str,
) -> Result<(), VagusError> {
    let mut cb = CIRCUIT_BREAKERS
        .may_load(storage, key.to_string())?
        .unwrap_or(CircuitBreaker {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure_time: 0,
            success_count: 0,
            next_attempt_time: 0,
        });

    if matches!(cb.state, CircuitState::HalfOpen) {
        cb.success_count += 1;
        let recovery_threshold = CIRCUIT_BREAKER_RECOVERY.load(storage)?;
        if cb.success_count >= recovery_threshold {
            // Reset to closed state
            cb.state = CircuitState::Closed;
            cb.failure_count = 0;
            cb.success_count = 0;
        }
    } else if matches!(cb.state, CircuitState::Closed) {
        // Reset failure count on success
        cb.failure_count = 0;
    }

    CIRCUIT_BREAKERS.save(storage, key.to_string(), &cb)?;
    Ok(())
}

// Governance execution functions

pub fn execute_set_reflex_arc(
    deps: DepsMut,
    info: MessageInfo,
    reflex_arc: String,
) -> Result<Response, VagusError> {
    // Only DAO can set reflex arc
    let dao = VAGUS_DAO.load(deps.storage)?;
    if info.sender.to_string() != dao {
        return Err(VagusError::Unauthorized);
    }

    deps.api.addr_validate(&reflex_arc)?;
    REFLEX_ARC.save(deps.storage, &reflex_arc)?;

    Ok(Response::new()
        .add_attribute("action", "set_reflex_arc")
        .add_attribute("reflex_arc", reflex_arc))
}

pub fn execute_set_rate_limit(
    deps: DepsMut,
    info: MessageInfo,
    window_size: u64,
    max_requests: u64,
) -> Result<Response, VagusError> {
    // Only DAO can set rate limits
    let dao = VAGUS_DAO.load(deps.storage)?;
    if info.sender.to_string() != dao {
        return Err(VagusError::Unauthorized);
    }

    let rate_limit = RateLimitConfig {
        window_size,
        max_requests,
    };
    GLOBAL_RATE_LIMIT.save(deps.storage, &rate_limit)?;

    Ok(Response::new()
        .add_attribute("action", "set_rate_limit")
        .add_attribute("window_size", window_size.to_string())
        .add_attribute("max_requests", max_requests.to_string()))
}

pub fn execute_set_circuit_breaker_params(
    deps: DepsMut,
    info: MessageInfo,
    threshold: u64,
    timeout: u64,
    recovery: u64,
) -> Result<Response, VagusError> {
    // Only DAO can set circuit breaker params
    let dao = VAGUS_DAO.load(deps.storage)?;
    if info.sender.to_string() != dao {
        return Err(VagusError::Unauthorized);
    }

    CIRCUIT_BREAKER_THRESHOLD.save(deps.storage, &threshold)?;
    CIRCUIT_BREAKER_TIMEOUT.save(deps.storage, &timeout)?;
    CIRCUIT_BREAKER_RECOVERY.save(deps.storage, &recovery)?;

    Ok(Response::new()
        .add_attribute("action", "set_circuit_breaker_params")
        .add_attribute("threshold", threshold.to_string())
        .add_attribute("timeout", timeout.to_string())
        .add_attribute("recovery", recovery.to_string()))
}

pub fn execute_emergency_pause(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, VagusError> {
    // Only DAO can pause
    let dao = VAGUS_DAO.load(deps.storage)?;
    if info.sender.to_string() != dao {
        return Err(VagusError::Unauthorized);
    }

    EMERGENCY_PAUSED.save(deps.storage, &true)?;

    Ok(Response::new()
        .add_attribute("action", "emergency_pause"))
}

pub fn execute_emergency_unpause(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, VagusError> {
    // Only DAO can unpause
    let dao = VAGUS_DAO.load(deps.storage)?;
    if info.sender.to_string() != dao {
        return Err(VagusError::Unauthorized);
    }

    EMERGENCY_PAUSED.save(deps.storage, &false)?;

    Ok(Response::new()
        .add_attribute("action", "emergency_unpause"))
}
