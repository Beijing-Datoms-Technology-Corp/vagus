//! Capability Token Manager
//!
//! Tracks active capability tokens for each executor and validates them locally.

use ethers::types::{Address, U256};
use std::collections::HashMap;
use vagus_crypto::VagusCrypto;

/// Capability token information
#[derive(Debug, Clone)]
pub struct CapabilityToken {
    pub token_id: U256,
    pub executor_id: U256,
    pub action_id: [u8; 32],
    pub scaled_limits_hash: [u8; 32],
    pub issued_at: u64,
    pub expires_at: u64,
    pub revoked: bool,
}

/// Token manager for tracking active capabilities
pub struct TokenManager {
    /// Active tokens per executor
    active_tokens: HashMap<U256, Vec<CapabilityToken>>,
    /// Crypto utilities for validation
    crypto: VagusCrypto,
}

impl TokenManager {
    /// Create a new token manager
    pub fn new(crypto: VagusCrypto) -> Self {
        Self {
            active_tokens: HashMap::new(),
            crypto,
        }
    }

    /// Add a new capability token
    pub fn add_token(&mut self, token: CapabilityToken) {
        let executor_id = token.executor_id;
        self.active_tokens
            .entry(executor_id)
            .or_insert_with(Vec::new)
            .push(token);
    }

    /// Revoke a capability token
    pub fn revoke_token(&mut self, token_id: U256) -> bool {
        for (_executor_id, tokens) in &mut self.active_tokens {
            if let Some(pos) = tokens.iter().position(|t| t.token_id == token_id) {
                tokens[pos].revoked = true;
                return true;
            }
        }
        false
    }

    /// Check if a token is valid (not expired, not revoked)
    pub fn is_token_valid(&self, token_id: U256, current_time: u64) -> bool {
        for (_executor_id, tokens) in &self.active_tokens {
            if let Some(token) = tokens.iter().find(|t| t.token_id == token_id) {
                return !token.revoked && current_time <= token.expires_at;
            }
        }
        false
    }

    /// Get all active (valid) tokens for an executor
    pub fn get_active_tokens(&self, executor_id: U256, current_time: u64) -> Vec<&CapabilityToken> {
        self.active_tokens
            .get(&executor_id)
            .map(|tokens| {
                tokens
                    .iter()
                    .filter(|token| !token.revoked && current_time <= token.expires_at)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Validate scaling limits hash for a token
    pub fn validate_scaling_limits(
        &self,
        token_id: U256,
        provided_hash: &[u8; 32],
    ) -> bool {
        for (_executor_id, tokens) in &self.active_tokens {
            if let Some(token) = tokens.iter().find(|t| t.token_id == token_id) {
                return token.scaled_limits_hash == *provided_hash;
            }
        }
        false
    }

    /// Clean up expired tokens
    pub fn cleanup_expired(&mut self, current_time: u64) {
        for (_executor_id, tokens) in &mut self.active_tokens {
            tokens.retain(|token| current_time <= token.expires_at);
        }

        // Remove empty executor entries
        self.active_tokens.retain(|_executor_id, tokens| !tokens.is_empty());
    }

    /// Get token count per executor
    pub fn get_token_count(&self, executor_id: U256) -> usize {
        self.active_tokens
            .get(&executor_id)
            .map(|tokens| tokens.len())
            .unwrap_or(0)
    }
}

// Note: Conversion from telemetry TokenMeta would go here
// when TokenMeta type is available in vagus-telemetry

#[cfg(test)]
mod tests {
    use super::*;
    use vagus_crypto::VagusDomain;
    use ethers::types::Address;

    fn create_test_crypto() -> VagusCrypto {
        let domain = VagusDomain {
            name: "Vagus".to_string(),
            version: "1".to_string(),
            chain_id: 31337,
            verifying_contract: Address::zero(),
        };
        VagusCrypto::new(domain)
    }

    #[test]
    fn test_token_management() {
        let crypto = create_test_crypto();
        let mut manager = TokenManager::new(crypto);

        let token = CapabilityToken {
            token_id: 1.into(),
            executor_id: 42.into(),
            action_id: [1u8; 32],
            scaled_limits_hash: [2u8; 32],
            issued_at: 1000,
            expires_at: 2000,
            revoked: false,
        };

        // Add token
        manager.add_token(token.clone());
        assert_eq!(manager.get_token_count(42.into()), 1);

        // Check validity
        assert!(manager.is_token_valid(1.into(), 1500));

        // Check scaling limits
        assert!(manager.validate_scaling_limits(1.into(), &[2u8; 32]));
        assert!(!manager.validate_scaling_limits(1.into(), &[3u8; 32]));

        // Get active tokens
        let active = manager.get_active_tokens(42.into(), 1500);
        assert_eq!(active.len(), 1);

        // Revoke token
        assert!(manager.revoke_token(1.into()));
        assert!(!manager.is_token_valid(1.into(), 1500));

        // Check expired token
        assert!(!manager.is_token_valid(1.into(), 2500));
    }

    #[test]
    fn test_cleanup_expired() {
        let crypto = create_test_crypto();
        let mut manager = TokenManager::new(crypto);

        let token1 = CapabilityToken {
            token_id: 1.into(),
            executor_id: 42.into(),
            action_id: [1u8; 32],
            scaled_limits_hash: [1u8; 32],
            issued_at: 1000,
            expires_at: 1500, // Expires early
            revoked: false,
        };

        let token2 = CapabilityToken {
            token_id: 2.into(),
            executor_id: 42.into(),
            action_id: [2u8; 32],
            scaled_limits_hash: [2u8; 32],
            issued_at: 1000,
            expires_at: 2500, // Expires later
            revoked: false,
        };

        manager.add_token(token1);
        manager.add_token(token2);

        assert_eq!(manager.get_token_count(42.into()), 2);

        // Clean up at time 2000
        manager.cleanup_expired(2000);

        // Only token2 should remain
        assert_eq!(manager.get_token_count(42.into()), 1);
        let active = manager.get_active_tokens(42.into(), 2000);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].token_id, 2.into());
    }
}
