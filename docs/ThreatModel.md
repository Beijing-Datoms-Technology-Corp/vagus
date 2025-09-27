# Vagus Threat Model

## Security Assumptions

1. Blockchain provides immutable state and consensus
2. EIP-712 signatures prevent intent tampering
3. Time-based token expiration limits damage windows

## Attack Vectors

### 1. Intent Manipulation
- **Mitigation**: EIP-712 structured signing
- **Mitigation**: Schema validation at multiple layers

### 2. Evidence Spoofing
- **Mitigation**: Cryptographic attestation requirements
- **Mitigation**: Multi-source evidence correlation

### 3. State Machine Exploitation
- **Mitigation**: Hysteresis prevents oscillation
- **Mitigation**: Rate limiting on state transitions

### 4. Capability Exhaustion
- **Mitigation**: Gas limits and economic costs
- **Mitigation**: Token expiration and revocation

## Safety Invariants

I1: DANGER state always reduces capability vs SAFE
I2: SHUTDOWN state blocks all dangerous actions
I3: Revoked tokens cannot be used for new actions
I4: Evidence must be fresher than action timestamps
I5: Scaling never increases brakeable parameters beyond safe limits
