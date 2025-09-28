"""
Deterministic CBOR encoding for Vagus cross-chain consistency.

Implements RFC 8949 compliant deterministic CBOR encoding with:
- Canonical encoding
- Length-first sorting for maps
- No indefinite length encoding
- IEEE 754 float representation
"""

import hashlib
from typing import Any, Dict, List, Union
import cbor2


class DeterministicCBOR:
    """Deterministic CBOR encoder for Vagus protocol."""

    @staticmethod
    def encode_deterministic(data: Any) -> bytes:
        """
        Encode data to deterministic CBOR bytes.

        Args:
            data: Python data structure to encode

        Returns:
            CBOR-encoded bytes with deterministic encoding
        """
        # Use cbor2 with deterministic options
        return cbor2.dumps(
            data,
            canonical=True,  # Use canonical encoding
            datetime_as_timestamp=True,  # Timestamps as seconds since epoch
        )

    @staticmethod
    def hash_sha256(cbor_bytes: bytes) -> bytes:
        """Compute SHA256 hash of CBOR bytes."""
        return hashlib.sha256(cbor_bytes).digest()

    @staticmethod
    def hash_keccak(cbor_bytes: bytes) -> bytes:
        """Compute Keccak256 hash of CBOR bytes."""
        return hashlib.sha3_256(cbor_bytes).digest()

    @staticmethod
    def encode_and_hash(data: Any) -> tuple[bytes, bytes, bytes]:
        """
        Encode data to CBOR and compute both hashes.

        Returns:
            Tuple of (cbor_bytes, sha256_hash, keccak_hash)
        """
        cbor_bytes = DeterministicCBOR.encode_deterministic(data)
        sha256_hash = DeterministicCBOR.hash_sha256(cbor_bytes)
        keccak_hash = DeterministicCBOR.hash_keccak(cbor_bytes)
        return cbor_bytes, sha256_hash, keccak_hash


def normalize_intent_params(params: Dict[str, Any]) -> Dict[str, Any]:
    """
    Normalize intent parameters for deterministic encoding.

    - Ensure numeric types are consistent
    - Sort dictionary keys
    - Remove null/None values
    """
    normalized = {}

    for key in sorted(params.keys()):
        value = params[key]
        if value is not None:  # Skip null values
            # Ensure consistent numeric types
            if isinstance(value, (int, float)):
                if isinstance(value, float):
                    # Use IEEE 754 representation
                    pass
                elif isinstance(value, int):
                    # Ensure within bounds
                    if -2**63 <= value < 2**64:
                        normalized[key] = value
                    else:
                        raise ValueError(f"Integer value {value} out of range")
            else:
                normalized[key] = value

    return normalized


def create_cbor_test_vectors() -> List[Dict[str, Any]]:
    """
    Create test vectors for CBOR encoding verification.

    Returns:
        List of test cases with input data and expected properties
    """
    test_cases = [
        # Simple values
        {"name": "empty_dict", "data": {}},
        {"name": "simple_dict", "data": {"key": "value", "num": 42}},
        {"name": "sorted_keys", "data": {"z": 1, "a": 2, "m": 3}},
        {"name": "nested_dict", "data": {"outer": {"inner": "value"}}},
        {"name": "array", "data": [1, 2, 3, 4]},
        {"name": "mixed_types", "data": {"int": 123, "float": 45.67, "bool": True, "str": "test"}},

        # Edge cases
        {"name": "zero_values", "data": {"zero": 0, "false": False, "empty": ""}},
        {"name": "large_int", "data": {"big": 2**32 - 1}},
        {"name": "negative_int", "data": {"neg": -123}},

        # Vagus-specific structures
        {"name": "intent_params", "data": {
            "velocity": 1000,
            "acceleration": 500,
            "duration_ms": 30000,
            "energy_j": 100
        }},
        {"name": "state_root", "data": {
            "position": {"x": 100, "y": 200, "z": 50},
            "velocity": {"x": 10, "y": 5, "z": 0}
        }},
    ]

    return test_cases


if __name__ == "__main__":
    # Test the implementation
    test_cases = create_cbor_test_vectors()

    for case in test_cases:
        cbor_bytes, sha256_hash, keccak_hash = DeterministicCBOR.encode_and_hash(case["data"])
        print(f"{case['name']}: CBOR len={len(cbor_bytes)}, SHA256={sha256_hash.hex()[:16]}..., KECCAK={keccak_hash.hex()[:16]}...")
