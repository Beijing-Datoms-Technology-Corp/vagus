use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint256,
};
use cw_storage_plus::{Item, Map};
use cw721_base::Cw721Contract;
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

#[cosmwasm_schema::cw_serde]
pub struct InstantiateMsg {
    pub authorized_executors: Vec<String>,
    pub reflex_arc: Option<String>,
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

    if let Some(reflex_arc) = msg.reflex_arc {
        deps.api.addr_validate(&reflex_arc)?;
        REFLEX_ARC.save(deps.storage, &reflex_arc)?;
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("planner_count", planners.len().to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, VagusError> {
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
