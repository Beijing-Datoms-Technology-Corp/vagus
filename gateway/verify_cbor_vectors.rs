use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use vagus_crypto::cbor::{encode_and_hash, encode_deterministic};

#[derive(Debug, Serialize, Deserialize)]
struct TestVector {
    name: String,
    input: serde_json::Value,
    cbor_hex: String,
    sha256_hex: String,
    keccak_hex: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TestVectors {
    version: String,
    test_vectors: Vec<TestVector>,
}

fn main() -> Result<()> {
    // Load test vectors from YAML
    let yaml_content = fs::read_to_string("../../spec/vectors/cbor_cases.yml")?;
    let vectors: TestVectors = serde_yaml::from_str(&yaml_content)?;

    println!("Verifying {} CBOR test vectors...", vectors.test_vectors.len());

    let mut passed = 0;
    let mut failed = 0;

    for vector in &vectors.test_vectors {
        println!("Testing: {}", vector.name);

        // Encode the input
        let (cbor_bytes, sha256_hash, keccak_hash) = encode_and_hash(&vector.input)?;

        // Compare CBOR bytes
        let expected_cbor = hex::decode(&vector.cbor_hex)?;
        if cbor_bytes != expected_cbor {
            println!("  ❌ CBOR mismatch for {}", vector.name);
            println!("    Expected: {}", hex::encode(&expected_cbor));
            println!("    Got:      {}", hex::encode(&cbor_bytes));
            failed += 1;
            continue;
        }

        // Compare SHA256 hash
        let expected_sha256 = hex::decode(&vector.sha256_hex)?;
        if sha256_hash.to_vec() != expected_sha256 {
            println!("  ❌ SHA256 mismatch for {}", vector.name);
            failed += 1;
            continue;
        }

        // Compare Keccak hash
        let expected_keccak = hex::decode(&vector.keccak_hex)?;
        if keccak_hash.to_vec() != expected_keccak {
            println!("  ❌ Keccak mismatch for {}", vector.name);
            failed += 1;
            continue;
        }

        println!("  ✅ {}", vector.name);
        passed += 1;
    }

    println!("\nResults: {} passed, {} failed", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}
