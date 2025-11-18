import pytest

from vagus_planner.deterministic_cbor import (
    DeterministicCBOR,
    normalize_intent_params,
)


def test_encode_deterministic_uses_canonical_ordering():
    payload = {"z": 1, "a": 2}

    encoded = DeterministicCBOR.encode_deterministic(payload)

    # Canonical CBOR sorts map keys by length, then lex order.
    assert encoded.hex() == "a2616102617a01"


def test_encode_and_hash_matches_known_vectors():
    payload = {"key": "value", "num": 42}

    cbor_bytes, sha256_hash, keccak_hash = DeterministicCBOR.encode_and_hash(payload)

    assert cbor_bytes.hex() == "a2636b65796576616c7565636e756d182a"
    assert (
        sha256_hash.hex()
        == "f393d6414dc119f3be2f478e6880a0ae2bd3cc2e393e0847d4a7794dea1e27ee"
    )
    assert (
        keccak_hash.hex()
        == "064f4786d0996d40aa97e673a657563acb6f276863fe34ebf8069f4d3c048693"
    )


def test_normalize_intent_params_drops_none_and_sorts_keys():
    params = {"velocity": 1000, "unused": None, "accel": 500}

    normalized = normalize_intent_params(params)

    assert normalized == {"accel": 500, "velocity": 1000}


def test_normalize_intent_params_rejects_out_of_range_integer():
    params = {"huge": 2**80}

    with pytest.raises(ValueError):
        normalize_intent_params(params)
