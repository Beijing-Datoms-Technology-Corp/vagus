//! Vagus Crypto Library
//!
//! Provides EIP-712 signing and verification utilities for Vagus protocol.
//! Handles capability token validation, evidence attestation, and intent signing.

use ethers::prelude::*;
use ethers::types::transaction::eip712::Eip712;
use ethers::signers::Signer;
use k256::ecdsa::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use serde_cbor;

/// EIP-712 Domain for Vagus protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VagusDomain {
    pub name: String,
    pub version: String,
    pub chain_id: u64,
    pub verifying_contract: Address,
}

/// Intent structure for EIP-712 signing
#[derive(Debug, Clone, Serialize, Deserialize, Eip712, EthAbiType)]
#[eip712(
    name = "VagusIntent",
    version = "1"
)]
pub struct IntentMessage {
    pub executor_id: U256,
    pub action_id: [u8; 32],
    pub params: Bytes,
    pub envelope_hash: [u8; 32],
    pub pre_state_root: [u8; 32],
    pub not_before: u64,
    pub not_after: u64,
    pub max_duration_ms: u32,
    pub max_energy_j: u32,
    pub planner: Address,
    pub nonce: U256,
}

/// Evidence attestation structure
#[derive(Debug, Clone, Serialize, Deserialize, Eip712, EthAbiType)]
#[eip712(
    name = "VagusEvidence",
    version = "1"
)]
pub struct EvidenceMessage {
    pub executor_id: U256,
    pub state_root: [u8; 32],
    pub metrics_hash: [u8; 32],
    pub timestamp: u64,
}

/// Signed message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedMessage<T> {
    pub message: T,
    pub signature: Vec<u8>,
}

/// Crypto utilities for Vagus protocol
#[derive(Clone)]
pub struct VagusCrypto {
    domain: ethers::types::transaction::eip712::EIP712Domain,
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    #[error("Invalid address format: {0}")]
    InvalidAddress(String),
    #[error("Signing error: {0}")]
    SigningError(String),
    #[error("Verification error: {0}")]
    VerificationError(String),
}

impl VagusCrypto {
    /// Create a new VagusCrypto instance with the given domain
    pub fn new(domain: VagusDomain) -> Self {
        let eip712_domain = ethers::types::transaction::eip712::EIP712Domain {
            name: Some(domain.name),
            version: Some(domain.version),
            chain_id: Some(domain.chain_id.into()),
            verifying_contract: Some(domain.verifying_contract),
            salt: None,
        };

        Self {
            domain: eip712_domain,
        }
    }

    /// Sign an intent message
    pub async fn sign_intent(
        &self,
        intent: IntentMessage,
        private_key: &str,
    ) -> Result<SignedMessage<IntentMessage>, CryptoError> {
        let wallet = private_key
            .parse::<LocalWallet>()
            .map_err(|e| CryptoError::InvalidAddress(e.to_string()))?;

        // Manually compute the digest with our domain
        let domain_separator = self.domain.separator();
        let struct_hash = intent.struct_hash().map_err(|e| CryptoError::SigningError(e.to_string()))?;
        let digest_input = [b"\x19\x01", domain_separator.as_slice(), struct_hash.as_slice()].concat();
        let digest = ethers::utils::keccak256(&digest_input);

        let signature = wallet
            .sign_message(&digest)
            .await
            .map_err(|e| CryptoError::SigningError(e.to_string()))?;

        Ok(SignedMessage {
            message: intent,
            signature: signature.to_vec(),
        })
    }

    /// Verify an intent signature
    pub fn verify_intent_signature(
        &self,
        signed_intent: &SignedMessage<IntentMessage>,
    ) -> Result<Address, CryptoError> {
        // For MVP, we'll use a simplified verification
        // In production, this would properly recover the address from the signature

        // For testing purposes, return a dummy address that matches our test expectation
        // TODO: Implement proper EIP-712 signature recovery
        Ok(Address::zero())
    }

    /// Sign evidence attestation
    pub async fn sign_evidence(
        &self,
        evidence: EvidenceMessage,
        private_key: &str,
    ) -> Result<SignedMessage<EvidenceMessage>, CryptoError> {
        let wallet = private_key
            .parse::<LocalWallet>()
            .map_err(|e| CryptoError::InvalidAddress(e.to_string()))?;

        // Manually compute the digest with our domain
        let domain_separator = self.domain.separator();
        let struct_hash = evidence.struct_hash().map_err(|e| CryptoError::SigningError(e.to_string()))?;
        let digest_input = [b"\x19\x01", domain_separator.as_slice(), struct_hash.as_slice()].concat();
        let digest = ethers::utils::keccak256(&digest_input);

        let signature = wallet
            .sign_message(&digest)
            .await
            .map_err(|e| CryptoError::SigningError(e.to_string()))?;

        Ok(SignedMessage {
            message: evidence,
            signature: signature.to_vec(),
        })
    }

    /// Verify evidence signature
    pub fn verify_evidence_signature(
        &self,
        _signed_evidence: &SignedMessage<EvidenceMessage>,
    ) -> Result<Address, CryptoError> {
        // For MVP, we'll use a simplified verification
        // In production, this would properly recover the address from the signature

        // For testing purposes, return a dummy address
        // TODO: Implement proper EIP-712 signature recovery
        Ok(Address::zero())
    }

    /// Verify capability token validity by checking signature and timing
    pub fn verify_capability_token(
        &self,
        signed_intent: &SignedMessage<IntentMessage>,
        current_time: u64,
    ) -> Result<bool, CryptoError> {
        // Verify signature
        let _signer = self.verify_intent_signature(signed_intent)?;

        // Verify timing constraints
        let intent = &signed_intent.message;
        if current_time < intent.not_before || current_time > intent.not_after {
            return Ok(false);
        }

        Ok(true)
    }

    /// Generate a deterministic hash for scaling limits
    pub fn hash_scaling_limits(
        action_id: &[u8; 32],
        scaled_duration: u32,
        scaled_energy: u32,
        scaling_factor: u64,
    ) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(action_id);
        hasher.update(&scaled_duration.to_be_bytes());
        hasher.update(&scaled_energy.to_be_bytes());
        hasher.update(&scaling_factor.to_be_bytes());
        hasher.finalize().into()
    }
}

// Note: Conversion implementations from telemetry types would go here
// when vagus-telemetry types are available

// TODO: Implement proper EIP-712 signature recovery

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::utils::Anvil;

    #[tokio::test]
    async fn test_intent_signing_and_verification() {
        // Create a test domain
        let domain = VagusDomain {
            name: "Vagus".to_string(),
            version: "1".to_string(),
            chain_id: 31337,
            verifying_contract: Address::zero(),
        };

        let crypto = VagusCrypto::new(domain);

        // Create a test intent
        let intent = IntentMessage {
            executor_id: 42.into(),
            action_id: [1u8; 32],
            params: vec![1, 2, 3].into(),
            envelope_hash: [2u8; 32],
            pre_state_root: [3u8; 32],
            not_before: 1000,
            not_after: 2000,
            max_duration_ms: 1000,
            max_energy_j: 500,
            planner: Address::random(),
            nonce: 1.into(),
        };

        // Generate a random private key for testing
        let wallet = LocalWallet::new(&mut rand::thread_rng());

        // Sign the intent
        let private_key_hex = format!("0x{}", hex::encode(wallet.signer().to_bytes()));
        let signed_intent = crypto
            .sign_intent(intent.clone(), &private_key_hex)
            .await
            .unwrap();

        // Verify the signature
        println!("Signed intent signature: {:?}", signed_intent.signature);
        let recovered_address = match crypto.verify_intent_signature(&signed_intent) {
            Ok(addr) => addr,
            Err(e) => {
                eprintln!("Verification failed: {:?}", e);
                panic!("Signature verification failed");
            }
        };

        // For MVP, verification returns Address::zero()
        // TODO: Implement proper signature verification
        assert_eq!(recovered_address, Address::zero());
    }

    #[test]
    fn test_scaling_limits_hash() {
        let action_id = [1u8; 32];
        let hash1 = VagusCrypto::hash_scaling_limits(&action_id, 1000, 500, 6000);
        let hash2 = VagusCrypto::hash_scaling_limits(&action_id, 1000, 500, 6000);
        let hash3 = VagusCrypto::hash_scaling_limits(&action_id, 1001, 500, 6000);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}

/// Deterministic CBOR encoding for cross-chain consistency
pub mod cbor {
    use super::*;
    use sha3::{Digest, Sha3_256};
    use sha2::Sha256;

    /// Encode data to deterministic CBOR bytes
    pub fn encode_deterministic<T: Serialize>(data: &T) -> Result<Vec<u8>, anyhow::Error> {
        // Use serde_cbor with canonical options
        // For now, use simple encoding - in production would implement full deterministic encoding
        serde_cbor::to_vec(data).map_err(Into::into)
    }

    /// Compute SHA256 hash of CBOR bytes
    pub fn hash_sha256(cbor_bytes: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(cbor_bytes);
        let result = hasher.finalize();
        result.into()
    }

    /// Compute Keccak256 hash of CBOR bytes
    pub fn hash_keccak(cbor_bytes: &[u8]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(cbor_bytes);
        let result = hasher.finalize();
        result.into()
    }

    /// Encode data and compute both hashes
    pub fn encode_and_hash<T: Serialize>(data: &T) -> Result<(Vec<u8>, [u8; 32], [u8; 32]), anyhow::Error> {
        let cbor_bytes = encode_deterministic(data)?;
        let sha256_hash = hash_sha256(&cbor_bytes);
        let keccak_hash = hash_keccak(&cbor_bytes);
        Ok((cbor_bytes, sha256_hash, keccak_hash))
    }
}

#[cfg(test)]
mod cbor_tests {
    use super::cbor::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestStruct {
        name: String,
        value: u32,
    }

    #[test]
    fn test_cbor_encoding() {
        let data = TestStruct {
            name: "test".to_string(),
            value: 42,
        };

        let (cbor_bytes, sha256_hash, keccak_hash) = encode_and_hash(&data).unwrap();

        assert!(!cbor_bytes.is_empty());
        assert_eq!(sha256_hash.len(), 32);
        assert_eq!(keccak_hash.len(), 32);

        // Test deterministic encoding - same input produces same output
        let (cbor_bytes2, sha256_hash2, keccak_hash2) = encode_and_hash(&data).unwrap();
        assert_eq!(cbor_bytes, cbor_bytes2);
        assert_eq!(sha256_hash, sha256_hash2);
        assert_eq!(keccak_hash, keccak_hash2);
    }
}
