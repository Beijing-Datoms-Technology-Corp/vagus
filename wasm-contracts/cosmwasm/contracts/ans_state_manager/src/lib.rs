use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw_storage_plus::Item;

use vagus_spec::{ANSState, Guard, VagusError, VagalToneIndicator};

// State
pub const CURRENT_STATE: Item<ANSState> = Item::new("current_state");
pub const CURRENT_TONE: Item<VagalToneIndicator> = Item::new("current_tone");
pub const LAST_STATE_CHANGE: Item<u64> = Item::new("last_state_change");

// Configuration
pub const MIN_STATE_RESIDENCY: Item<u64> = Item::new("min_state_residency");
pub const SAFE_THRESHOLD: Item<u64> = Item::new("safe_threshold");     // 8000 (80%)
pub const DANGER_THRESHOLD: Item<u64> = Item::new("danger_threshold"); // 6000 (60%)

#[cosmwasm_schema::cw_serde]
pub struct InstantiateMsg {
    pub initial_state: ANSState,
    pub min_state_residency: u64, // seconds
    pub safe_threshold: u64,      // basis points
    pub danger_threshold: u64,    // basis points
}

#[cosmwasm_schema::cw_serde]
pub enum ExecuteMsg {
    UpdateTone { vti: u64, suggested: ANSState },
}

#[cosmwasm_schema::cw_serde]
pub enum QueryMsg {
    CurrentState {},
    CurrentTone {},
    GuardFor { action_id: Binary },
}

#[cosmwasm_schema::cw_serde]
pub struct CurrentStateResponse {
    pub state: ANSState,
    pub since: u64,
}

#[cosmwasm_schema::cw_serde]
pub struct CurrentToneResponse {
    pub tone: VagalToneIndicator,
}

#[cosmwasm_schema::cw_serde]
pub struct GuardForResponse {
    pub guard: Guard,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, VagusError> {
    // Validate thresholds
    if msg.safe_threshold <= msg.danger_threshold || msg.safe_threshold > 10000 {
        return Err(VagusError::InvalidInput);
    }

    CURRENT_STATE.save(deps.storage, &msg.initial_state)?;
    LAST_STATE_CHANGE.save(deps.storage, &0)?;
    MIN_STATE_RESIDENCY.save(deps.storage, &msg.min_state_residency)?;
    SAFE_THRESHOLD.save(deps.storage, &msg.safe_threshold)?;
    DANGER_THRESHOLD.save(deps.storage, &msg.danger_threshold)?;

    // Initialize tone to neutral
    let initial_tone = VagalToneIndicator {
        value: 7500u64.into(), // 75%
        timestamp: 0u64.into(),
    };
    CURRENT_TONE.save(deps.storage, &initial_tone)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("initial_state", format!("{:?}", msg.initial_state))
        .add_attribute("min_residency", msg.min_state_residency.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, VagusError> {
    match msg {
        ExecuteMsg::UpdateTone { vti, suggested } => {
            execute_update_tone(deps, env, info, vti, suggested)
        }
    }
}

pub fn execute_update_tone(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    vti: u64,
    suggested: ANSState,
) -> Result<Response, VagusError> {
    // Validate VTI range
    if vti > 10000 {
        return Err(VagusError::InvalidToneValue);
    }

    let current_state = CURRENT_STATE.load(deps.storage)?;
    let last_change = LAST_STATE_CHANGE.load(deps.storage)?;
    let min_residency = MIN_STATE_RESIDENCY.load(deps.storage)?;
    let safe_threshold = SAFE_THRESHOLD.load(deps.storage)?;
    let danger_threshold = DANGER_THRESHOLD.load(deps.storage)?;

    // Check hysteresis (prevent rapid state changes)
    let current_time = env.block.time.seconds();
    if last_change != 0 && current_time < last_change + min_residency {
        return Err(VagusError::StateChangeTooFrequent);
    }

    // Determine new state based on VTI and hysteresis
    let new_state = determine_state_with_hysteresis(
        current_state.clone(),
        vti,
        safe_threshold,
        danger_threshold,
    );

    // Override with suggested state if more conservative
    let final_state = if is_more_conservative(&suggested, &new_state) {
        suggested
    } else {
        new_state
    };

    // Update state if changed
    let state_changed = final_state != current_state;
    if state_changed {
        CURRENT_STATE.save(deps.storage, &final_state)?;
        LAST_STATE_CHANGE.save(deps.storage, &current_time)?;
    }

    // Update tone
    let tone = VagalToneIndicator {
        value: vti.into(),
        timestamp: current_time.into(),
    };
    CURRENT_TONE.save(deps.storage, &tone)?;

    let mut response = Response::new()
        .add_attribute("action", "update_tone")
        .add_attribute("vti", vti.to_string())
        .add_attribute("tone", vti.to_string())
        .add_attribute("state", format!("{:?}", final_state))
        .add_attribute("updated_at", current_time.to_string());

    if state_changed {
        response = response.add_attribute("state_changed", "true");
    }

    Ok(response)
}

fn determine_state_with_hysteresis(
    current: ANSState,
    vti: u64,
    safe_threshold: u64,
    danger_threshold: u64,
) -> ANSState {
    match current {
        ANSState::SAFE => {
            if vti < danger_threshold {
                ANSState::DANGER
            } else {
                ANSState::SAFE
            }
        }
        ANSState::DANGER => {
            if vti >= safe_threshold {
                ANSState::SAFE
            } else if vti < danger_threshold / 2 {
                // Very low VTI triggers shutdown
                ANSState::SHUTDOWN
            } else {
                ANSState::DANGER
            }
        }
        ANSState::SHUTDOWN => {
            if vti >= safe_threshold {
                ANSState::SAFE
            } else if vti >= danger_threshold {
                ANSState::DANGER
            } else {
                ANSState::SHUTDOWN
            }
        }
    }
}

fn is_more_conservative(a: &ANSState, b: &ANSState) -> bool {
    // SAFE < DANGER < SHUTDOWN (more conservative)
    let rank = |state: &ANSState| match state {
        ANSState::SAFE => 0,
        ANSState::DANGER => 1,
        ANSState::SHUTDOWN => 2,
    };

    rank(a) > rank(b)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CurrentState {} => to_json_binary(&query_current_state(deps)?),
        QueryMsg::CurrentTone {} => to_json_binary(&query_current_tone(deps)?),
        QueryMsg::GuardFor { action_id } => {
            to_json_binary(&query_guard_for(deps, env, action_id)?)
        }
    }
}

fn query_current_state(deps: Deps) -> StdResult<CurrentStateResponse> {
    let state = CURRENT_STATE.load(deps.storage)?;
    let since = LAST_STATE_CHANGE.load(deps.storage)?;
    Ok(CurrentStateResponse { state, since })
}

fn query_current_tone(deps: Deps) -> StdResult<CurrentToneResponse> {
    let tone = CURRENT_TONE.load(deps.storage)?;
    Ok(CurrentToneResponse { tone })
}

fn query_guard_for(deps: Deps, _env: Env, _action_id: Binary) -> StdResult<GuardForResponse> {
    let state = CURRENT_STATE.load(deps.storage)?;

    // Simplified guard logic - in production this would be action-specific
    let scaling_factor = match state {
        ANSState::SAFE => 10000u64,    // 100%
        ANSState::DANGER => 5000u64,    // 50%
        ANSState::SHUTDOWN => 0u64,     // 0%
    };

    let guard = Guard {
        scalingFactor: scaling_factor.into(),
        allowed: scaling_factor > 0,
    };

    Ok(GuardForResponse { guard })
}
