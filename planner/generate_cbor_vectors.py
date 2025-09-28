#!/usr/bin/env python3
"""
Generate CBOR test vectors for cross-chain verification.
"""

import yaml
import sys
import os

# Add the vagus_planner package to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'vagus_planner'))

from vagus_planner.deterministic_cbor import DeterministicCBOR, create_cbor_test_vectors


def main():
    """Generate CBOR test vectors and save to YAML."""
    test_cases = create_cbor_test_vectors()
    vectors = []

    for case in test_cases:
        cbor_bytes, sha256_hash, keccak_hash = DeterministicCBOR.encode_and_hash(case["data"])

        vector = {
            "name": case["name"],
            "input": case["data"],
            "cbor_hex": cbor_bytes.hex(),
            "sha256_hex": sha256_hash.hex(),
            "keccak_hex": keccak_hash.hex(),
        }
        vectors.append(vector)

    # Save to YAML
    output_path = os.path.join(os.path.dirname(__file__), '..', 'spec', 'vectors', 'cbor_cases.yml')
    with open(output_path, 'w') as f:
        yaml.dump({"version": "1.0", "test_vectors": vectors}, f, default_flow_style=False)

    print(f"Generated {len(vectors)} CBOR test vectors in {output_path}")


if __name__ == "__main__":
    main()
