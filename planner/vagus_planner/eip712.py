# EIP-712 signing utilities

import hashlib
from typing import Dict, Any, Optional
from eth_utils import keccak


class EIP712Domain:
    """EIP-712 domain separator"""

    def __init__(self, name: str, version: str, chain_id: int, verifying_contract: str):
        self.name = name
        self.version = version
        self.chain_id = chain_id
        self.verifying_contract = verifying_contract

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for encoding"""
        return {
            "name": self.name,
            "version": self.version,
            "chainId": self.chain_id,
            "verifyingContract": self.verifying_contract,
        }

    def separator_hash(self) -> bytes:
        """Compute domain separator hash"""
        domain_type = "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
        domain_data = [
            keccak(text=domain_type),
            keccak(text=self.name),
            keccak(text=self.version),
            self.chain_id.to_bytes(32, 'big'),
            bytes.fromhex(self.verifying_contract[2:]),  # Remove 0x prefix
        ]

        return keccak(b''.join(domain_data))


class EIP712Encoder:
    """EIP-712 encoder for Vagus intents"""

    def __init__(self, domain: EIP712Domain):
        self.domain = domain

    def encode_intent(self, intent_dict: Dict[str, Any]) -> bytes:
        """Encode an intent for EIP-712 signing"""
        # Intent type definition
        intent_type = (
            "Intent("
            "uint256 executorId,"
            "bytes32 actionId,"
            "bytes params,"
            "bytes32 envelopeHash,"
            "bytes32 preStateRoot,"
            "uint256 notBefore,"
            "uint256 notAfter,"
            "uint32 maxDurationMs,"
            "uint32 maxEnergyJ,"
            "address planner,"
            "uint256 nonce"
            ")"
        )

        # Encode the intent data
        encoded_data = [
            keccak(text=intent_type),
            intent_dict["executorId"].to_bytes(32, 'big'),
            bytes.fromhex(intent_dict["actionId"][2:]),
            keccak(intent_dict["params"]),
            bytes.fromhex(intent_dict["envelopeHash"][2:]),
            bytes.fromhex(intent_dict["preStateRoot"][2:]),
            intent_dict["notBefore"].to_bytes(32, 'big'),
            intent_dict["notAfter"].to_bytes(32, 'big'),
            intent_dict["maxDurationMs"].to_bytes(32, 'big'),
            intent_dict["maxEnergyJ"].to_bytes(32, 'big'),
            bytes.fromhex(intent_dict["planner"][2:]),
            intent_dict["nonce"].to_bytes(32, 'big'),
        ]

        intent_hash = keccak(b''.join(encoded_data))

        # Final EIP-712 digest
        digest_input = b'\x19\x01' + self.domain.separator_hash() + intent_hash
        return keccak(digest_input)

    def encode_typed_data(self, intent_dict: Dict[str, Any]) -> Dict[str, Any]:
        """Create the full typed data structure for EIP-712 signing"""
        return {
            "types": {
                "EIP712Domain": [
                    {"name": "name", "type": "string"},
                    {"name": "version", "type": "string"},
                    {"name": "chainId", "type": "uint256"},
                    {"name": "verifyingContract", "type": "address"},
                ],
                "Intent": [
                    {"name": "executorId", "type": "uint256"},
                    {"name": "actionId", "type": "bytes32"},
                    {"name": "params", "type": "bytes"},
                    {"name": "envelopeHash", "type": "bytes32"},
                    {"name": "preStateRoot", "type": "bytes32"},
                    {"name": "notBefore", "type": "uint256"},
                    {"name": "notAfter", "type": "uint256"},
                    {"name": "maxDurationMs", "type": "uint32"},
                    {"name": "maxEnergyJ", "type": "uint32"},
                    {"name": "planner", "type": "address"},
                    {"name": "nonce", "type": "uint256"},
                ],
            },
            "primaryType": "Intent",
            "domain": self.domain.to_dict(),
            "message": intent_dict,
        }


# Default Vagus domain
VAGUS_DOMAIN = EIP712Domain(
    name="Vagus",
    version="1",
    chain_id=31337,  # Anvil default
    verifying_contract="0x0000000000000000000000000000000000000000",  # Placeholder
)

# Default encoder
default_encoder = EIP712Encoder(VAGUS_DOMAIN)


def encode_intent_for_signing(intent_dict: Dict[str, Any]) -> bytes:
    """Encode an intent for EIP-712 signing"""
    return default_encoder.encode_intent(intent_dict)


def create_typed_data(intent_dict: Dict[str, Any]) -> Dict[str, Any]:
    """Create typed data structure for EIP-712 signing"""
    return default_encoder.encode_typed_data(intent_dict)
