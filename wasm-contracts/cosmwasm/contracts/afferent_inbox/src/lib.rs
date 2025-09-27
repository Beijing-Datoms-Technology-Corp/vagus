use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw_storage_plus::Item;
use cw_utils::nonpayable;

use vagus_spec::{
    AfferentEvidencePacket, CapabilityRevocationReason, VagusError,
};

// State
pub const LATEST_AEP: Item<AfferentEvidencePacket> = Item::new("latest_aep");

// Authorized attestors (oracle/gateway addresses)
pub const AUTHORIZED_ATTESTORS: Item<Vec<String>> = Item::new("authorized_attestors");

#[cosmwasm_schema::cw_serde]
pub struct InstantiateMsg {
    pub authorized_attestors: Vec<String>,
}

#[cosmwasm_schema::cw_serde]
pub enum ExecuteMsg {
    PostAEP {
        executor_id: u64,
        state_root_sha256: Binary,   // 32 bytes
        state_root_keccak: Binary,   // 32 bytes
        metrics_hash_sha256: Binary, // 32 bytes
        metrics_hash_keccak: Binary, // 32 bytes
        attestation: Binary,         // Optional attestation data
    },
    SetAuthorizedAttestors {
        attestors: Vec<String>,
    },
}

#[cosmwasm_schema::cw_serde]
pub enum QueryMsg {
    LatestAEP { executor_id: u64 },
    IsAuthorized { attestor: String },
}

#[cosmwasm_schema::cw_serde]
pub struct LatestAEPResponse {
    pub aep: Option<AfferentEvidencePacket>,
}

#[cosmwasm_schema::cw_serde]
pub struct IsAuthorizedResponse {
    pub authorized: bool,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, VagusError> {
    // Validate addresses
    let mut validated_attestors = Vec::new();
    for attestor in msg.authorized_attestors {
        deps.api.addr_validate(&attestor)?;
        validated_attestors.push(attestor);
    }

    AUTHORIZED_ATTESTORS.save(deps.storage, &validated_attestors)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("attestor_count", validated_attestors.len().to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, VagusError> {
    match msg {
        ExecuteMsg::PostAEP {
            executor_id,
            state_root_sha256,
            state_root_keccak,
            metrics_hash_sha256,
            metrics_hash_keccak,
            attestation,
        } => execute_post_aep(
            deps,
            env,
            info,
            executor_id,
            state_root_sha256,
            state_root_keccak,
            metrics_hash_sha256,
            metrics_hash_keccak,
            attestation,
        ),
        ExecuteMsg::SetAuthorizedAttestors { attestors } => {
            execute_set_authorized_attestors(deps, info, attestors)
        }
    }
}

pub fn execute_post_aep(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    executor_id: u64,
    state_root_sha256: Binary,
    state_root_keccak: Binary,
    metrics_hash_sha256: Binary,
    metrics_hash_keccak: Binary,
    _attestation: Binary,
) -> Result<Response, VagusError> {
    // Check authorization
    let attestors = AUTHORIZED_ATTESTORS.load(deps.storage)?;
    if !attestors.contains(&info.sender.to_string()) {
        return Err(VagusError::UnauthorizedAttestor);
    }

    // Validate hash lengths (32 bytes)
    if state_root_sha256.len() != 32
        || state_root_keccak.len() != 32
        || metrics_hash_sha256.len() != 32
        || metrics_hash_keccak.len() != 32
    {
        return Err(VagusError::InvalidInput);
    }

    let aep = AfferentEvidencePacket {
        executorId: executor_id.into(),
        stateRootSha256: state_root_sha256.clone(),
        stateRootKeccak: state_root_keccak.clone(),
        metricsHashSha256: metrics_hash_sha256.clone(),
        metricsHashKeccak: metrics_hash_keccak.clone(),
        timestamp: env.block.time.seconds().into(),
    };

    // Store the latest AEP (simplified - in production would store history)
    LATEST_AEP.save(deps.storage, &aep)?;

    Ok(Response::new()
        .add_attribute("action", "post_aep")
        .add_attribute("executor_id", executor_id.to_string())
        .add_attribute("state_root_sha256", hex::encode(&state_root_sha256))
        .add_attribute("state_root_keccak", hex::encode(&state_root_keccak))
        .add_attribute("metrics_hash_sha256", hex::encode(&metrics_hash_sha256))
        .add_attribute("metrics_hash_keccak", hex::encode(&metrics_hash_keccak))
        .add_attribute("timestamp", env.block.time.seconds().to_string()))
}

pub fn execute_set_authorized_attestors(
    deps: DepsMut,
    info: MessageInfo,
    attestors: Vec<String>,
) -> Result<Response, VagusError> {
    // Only contract admin can change attestors (simplified)
    // In production, this would check for admin privileges

    let mut validated_attestors = Vec::new();
    for attestor in attestors {
        deps.api.addr_validate(&attestor)?;
        validated_attestors.push(attestor);
    }

    AUTHORIZED_ATTESTORS.save(deps.storage, &validated_attestors)?;

    Ok(Response::new()
        .add_attribute("action", "set_authorized_attestors")
        .add_attribute("attestor_count", validated_attestors.len().to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::LatestAEP { executor_id } => {
            to_json_binary(&query_latest_aep(deps, executor_id)?)
        }
        QueryMsg::IsAuthorized { attestor } => {
            to_json_binary(&query_is_authorized(deps, attestor)?)
        }
    }
}

fn query_latest_aep(deps: Deps, _executor_id: u64) -> StdResult<LatestAEPResponse> {
    // Simplified - doesn't filter by executor_id, just returns latest
    let aep = LATEST_AEP.may_load(deps.storage)?;
    Ok(LatestAEPResponse { aep })
}

fn query_is_authorized(deps: Deps, attestor: String) -> StdResult<IsAuthorizedResponse> {
    let attestors = AUTHORIZED_ATTESTORS.load(deps.storage)?;
    let authorized = attestors.contains(&attestor);
    Ok(IsAuthorizedResponse { authorized })
}
