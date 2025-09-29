//! Auto-generated from spec/types.yml
//! DO NOT EDIT MANUALLY

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Uint256};
use thiserror::Error;

#[cw_serde]
pub enum ANSState {
    SAFE,
    DANGER,
    SHUTDOWN,
}

#[cw_serde]
pub enum CapabilityRevocationReason {
    OWNER_REVOCATION,
    REFLEX_TRIGGER,
    EXPIRATION,
}

#[cw_serde]
pub struct Intent {
    pub executorId: Uint256,
    pub actionId: Binary,
    pub params: Binary,
    pub envelopeHash: Binary,
    pub preStateRoot: Binary,
    pub notBefore: Uint256,
    pub notAfter: Uint256,
    pub maxDurationMs: Uint256,
    pub maxEnergyJ: Uint256,
    pub planner: String,
    pub nonce: Uint256,
}

#[cw_serde]
pub struct TokenMeta {
    pub tokenId: Uint256,
    pub executorId: Uint256,
    pub actionId: Binary,
    pub scaledLimitsHash: Binary,
    pub issuedAt: Uint256,
    pub expiresAt: Uint256,
    pub revoked: bool,
    pub revokedAt: Uint256,
}

#[cw_serde]
pub struct Guard {
    pub scalingFactor: Uint256,
    pub allowed: bool,
}

#[cw_serde]
pub struct AfferentEvidencePacket {
    pub executorId: Uint256,
    pub stateRootSha256: Binary,
    pub stateRootKeccak: Binary,
    pub metricsHashSha256: Binary,
    pub metricsHashKeccak: Binary,
    pub timestamp: Uint256,
}

#[cw_serde]
pub struct VagalToneIndicator {
    pub value: Uint256,
    pub timestamp: Uint256,
}

pub const MAX_DURATION_MS: u64 = 30000;
pub const MAX_ENERGY_J: u64 = 1000;
pub const MIN_STATE_RESIDENCY: u64 = 60;
pub const REFLEX_COOLDOWN: u64 = 30;

#[derive(Error, Debug)]
pub enum VagusError {
    #[error("{0}")]
    Std(#[from] cosmwasm_std::StdError),
    #[error("State change attempted too soon after previous change")]
    StateChangeTooFrequent,
    #[error("Tone value outside valid range (0-10000)")]
    InvalidToneValue,
    #[error("Intent execution time window has expired")]
    IntentExpired,
    #[error("Pre-execution state root mismatch")]
    InvalidPreState,
    #[error("Intent nonce has already been used")]
    NonceAlreadyUsed,
    #[error("Capability token does not exist")]
    TokenNotFound,
    #[error("Capability token is already revoked")]
    TokenAlreadyRevoked,
    #[error("Caller not authorized to revoke this token")]
    UnauthorizedRevocation,
    #[error("Execution blocked by ANS shutdown state")]
    ANSBlocked,
    #[error("Scaled parameter exceeds ANS limits")]
    ANSLimitExceeded,
    #[error("Caller not authorized to post evidence")]
    UnauthorizedAttestor,
    #[error("Evidence packet format is invalid")]
    InvalidEvidenceFormat,
    #[error("Request rate exceeds configured limits")]
    RateLimited,
    #[error("Circuit breaker is in open state, blocking requests")]
    CircuitBreakerOpen,
    #[error("CBOR normalized input produces different hashes across stacks")]
    CBORHashMismatch,
    #[error("Pre-execution state root does not match AfferentInbox latest")]
    StateMismatch,
    #[error("Time-to-live has expired")]
    TTLExpired,
    #[error("Caller not authorized for this operation")]
    Unauthorized,
    #[error("Input parameters are invalid")]
    InvalidInput,
    #[error("Contract is currently paused for emergency maintenance")]
    ContractPaused,
}