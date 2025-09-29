# Vagus Protocol SRE Runbook

## Overview

This runbook provides operational procedures for maintaining the Vagus Protocol in production environments. It covers monitoring, incident response, deployment procedures, and emergency protocols.

## Table of Contents

1. [Monitoring and Observability](#monitoring-and-observability)
2. [Incident Response](#incident-response)
3. [Deployment Procedures](#deployment-procedures)
4. [Emergency Protocols](#emergency-protocols)
5. [Maintenance Procedures](#maintenance-procedures)

## Monitoring and Observability

### Key Metrics to Monitor

#### EVM Chain Metrics
- **Rate Limiter Status**: Track requests per window, rejections
- **Circuit Breaker State**: Monitor state transitions, failure counts
- **ANS State Transitions**: Log state changes with timestamps
- **Capability Issuance**: Track successful/failed issuances
- **Gas Usage**: Monitor contract gas consumption patterns

#### WASM Chain Metrics
- **Contract Execution Time**: Monitor CosmWasm execution duration
- **Rate Limiting**: Track sliding window usage
- **Circuit Breaker Events**: Log state changes
- **Capability Token Lifecycle**: Monitor issuance, revocation patterns

#### Cross-Chain Metrics
- **Equivalence Checks**: Verify CBOR hash consistency
- **State Synchronization**: Monitor inbox state updates
- **Time Drift**: Ensure consistent timestamp handling

### Alert Conditions

#### Critical Alerts
- Circuit breaker enters OPEN state
- Rate limiter blocks >50% of requests for 5+ minutes
- ANS state stuck in DANGER/SHUTDOWN for >1 hour
- Cross-stack hash mismatch detected
- Contract emergency pause triggered

#### Warning Alerts
- Rate limiter blocks >10% of requests
- ANS state transitions >5 times per hour
- Gas usage exceeds 90% of block limit
- Time drift >5 seconds between chains

#### Info Alerts
- Successful capability issuance spikes
- New reflex arc triggers
- Governance proposal submissions

### Monitoring Dashboards

#### Prometheus Metrics
```prometheus
# Rate limiter metrics
vagus_rate_limiter_requests_total{contract="capability_issuer", chain="evm|wasm"}
vagus_rate_limiter_rejections_total{contract="capability_issuer", chain="evm|wasm"}
vagus_rate_limiter_window_usage_ratio{contract="capability_issuer", chain="evm|wasm"}

# Circuit breaker metrics
vagus_circuit_breaker_state{contract="capability_issuer", chain="evm|wasm"}
vagus_circuit_breaker_failures_total{contract="capability_issuer", chain="evm|wasm"}
vagus_circuit_breaker_successes_total{contract="capability_issuer", chain="evm|wasm"}

# ANS metrics
vagus_ans_state{executor_id="...", chain="evm|wasm"}
vagus_ans_transitions_total{from_state="...", to_state="...", chain="evm|wasm"}
vagus_ans_dwell_time_seconds{state="...", chain="evm|wasm"}

# Capability metrics
vagus_capability_issued_total{executor_id="...", action_id="...", chain="evm|wasm"}
vagus_capability_revoked_total{reason="...", chain="evm|wasm"}
vagus_capability_active_count{executor_id="...", chain="evm|wasm"}

# Cross-chain metrics
vagus_cross_chain_hash_match{component="params|prestate|metrics"}
vagus_cross_chain_time_drift_seconds{chain="evm_vs_wasm"}
```

#### Grafana Dashboard Layout

1. **Overview Panel**
   - System health status (green/yellow/red)
   - Active alerts summary
   - Key metric trends (last 24h)

2. **Rate Limiting Panel**
   - Request rates per executor
   - Rejection rates with breakdown
   - Window utilization heatmaps

3. **Circuit Breaker Panel**
   - Current states across all contracts
   - Failure/success ratios
   - Recovery time tracking

4. **ANS State Panel**
   - State distribution pie chart
   - Transition timeline
   - Dwell time histograms

5. **Capability Management Panel**
   - Issuance success rates
   - Active token counts
   - Revocation reasons breakdown

6. **Cross-Chain Equivalence Panel**
   - Hash consistency checks
   - Time synchronization status
   - Equivalence violation alerts

## Incident Response

### Incident Classification

#### P0 (Critical) - Immediate Response Required
- System completely unavailable
- Safety-critical functionality compromised
- Cross-stack equivalence broken
- Unauthorized access to governance

#### P1 (High) - Response within 1 hour
- Partial system degradation
- Rate limiting blocking legitimate traffic
- Circuit breakers stuck open
- ANS state management failures

#### P2 (Medium) - Response within 4 hours
- Monitoring alerts not working
- Performance degradation
- Non-critical feature failures

#### P3 (Low) - Response within 24 hours
- Cosmetic issues
- Minor performance issues
- Documentation inaccuracies

### Response Procedures

#### For Circuit Breaker OPEN Incidents

1. **Assessment**
   - Identify affected executor-action pairs
   - Check failure patterns and root causes
   - Verify if issue is isolated or systemic

2. **Containment**
   - Enable emergency pause if needed
   - Manually reset circuit breaker if appropriate
   - Notify affected users/partners

3. **Recovery**
   - Gradually move circuits to HALF_OPEN
   - Monitor success rates during recovery
   - Fully restore when success threshold met

4. **Post-Mortem**
   - Analyze failure patterns
   - Update circuit breaker thresholds if needed
   - Improve monitoring/alerting

#### For Rate Limiting Issues

1. **Assessment**
   - Check if rate limits are too restrictive
   - Identify if attack or legitimate traffic spike
   - Review rate limit configurations

2. **Containment**
   - Temporarily increase limits for affected pairs
   - Implement temporary IP-based filtering if needed
   - Enable emergency pause for specific executors

3. **Recovery**
   - Gradually restore normal limits
   - Monitor for sustained high traffic
   - Update baseline expectations

#### For ANS State Issues

1. **Assessment**
   - Check current state and transition history
   - Verify tone indicator data quality
   - Review hysteresis configuration

2. **Containment**
   - Manually adjust state if stuck
   - Pause ANS-driven actions if unsafe
   - Implement manual override procedures

3. **Recovery**
   - Restore automatic state management
   - Validate tone processing pipeline
   - Update hysteresis parameters

## Deployment Procedures

### Pre-Deployment Checklist

- [ ] All tests passing (unit, integration, invariant)
- [ ] Code review completed and approved
- [ ] Security audit completed for new features
- [ ] Cross-stack equivalence verified
- [ ] SBOM updated and validated
- [ ] Version number incremented appropriately
- [ ] Release notes prepared
- [ ] Rollback plan documented

### EVM Deployment

```bash
# 1. Prepare deployment
cd contracts
npm run prepare-deployment

# 2. Run pre-deployment tests
npm run test:predeploy

# 3. Deploy to testnet
npx hardhat run scripts/deploy.ts --network sepolia

# 4. Run post-deployment verification
npm run verify:deployment

# 5. Deploy to mainnet (with timelock)
npx hardhat run scripts/deploy.ts --network mainnet --timelock
```

### WASM Deployment

```bash
# 1. Build optimized contracts
cd wasm-contracts
cargo build --release --target wasm32-unknown-unknown

# 2. Optimize WASM binaries
wasm-opt target/wasm32-unknown-unknown/release/*.wasm -o optimized/

# 3. Upload to testnet
wasmcli tx wasm store optimized/capability_issuer.wasm --from validator --chain-id testnet

# 4. Instantiate contracts
wasmcli tx wasm instantiate <code-id> <init-msg> --from validator --chain-id testnet

# 5. Deploy to mainnet
wasmcli tx wasm store optimized/*.wasm --from multisig --chain-id mainnet
wasmcli tx wasm instantiate <code-id> <init-msg> --from multisig --chain-id mainnet
```

### Cross-Chain Synchronization

1. Deploy to EVM first, get contract addresses
2. Update WASM initialization parameters with EVM addresses
3. Deploy WASM contracts
4. Verify cross-chain communication channels
5. Initialize shared state (if any)
6. Run cross-chain equivalence tests

### Rollback Procedures

#### EVM Rollback
```bash
# Via governance timelock
npx hardhat run scripts/rollback.ts --network mainnet

# Emergency rollback (if governance compromised)
npx hardhat run scripts/emergency-rollback.ts --network mainnet
```

#### WASM Rollback
```bash
# Migrate to previous code version
wasmcli tx wasm migrate <contract-addr> <new-code-id> <migrate-msg> --from multisig

# Emergency pause all contracts
wasmcli tx wasm execute <contract-addr> '{"emergency_pause":{}}' --from multisig
```

## Emergency Protocols

### Emergency Pause Activation

#### When to Activate
- Suspected security vulnerability
- Critical system instability
- Governance compromise
- Severe performance degradation
- Cross-stack equivalence violations

#### Activation Procedure
```bash
# EVM Emergency Pause
npx hardhat run scripts/emergency-pause.ts --network mainnet

# WASM Emergency Pause
wasmcli tx wasm execute <capability-issuer> '{"emergency_pause":{}}' --from emergency-key
wasmcli tx wasm execute <reflex-arc> '{"emergency_pause":{}}' --from emergency-key
```

### Emergency Recovery

1. **Assess Situation**
   - Confirm emergency pause effectiveness
   - Identify root cause
   - Evaluate recovery options

2. **Execute Recovery Plan**
   - Apply security patches if needed
   - Update contract parameters
   - Restore from clean state

3. **Gradual Restoration**
   - Lift emergency pause in stages
   - Monitor system behavior
   - Full restoration when confident

### Disaster Recovery

#### Complete System Failure
1. Activate all emergency pauses
2. Notify all stakeholders
3. Assess damage extent
4. Execute recovery from backups
5. Verify system integrity
6. Gradual service restoration

#### Data Loss Scenarios
1. Identify affected components
2. Restore from distributed backups
3. Validate data consistency
4. Rebuild derived state
5. Verify cross-chain equivalence

## Maintenance Procedures

### Regular Maintenance Tasks

#### Daily
- Monitor alert queues
- Review system metrics trends
- Check cross-chain synchronization
- Validate backup integrity

#### Weekly
- Run full test suite
- Review and rotate access keys
- Update monitoring thresholds
- Check certificate expirations

#### Monthly
- Security patch assessment
- Performance optimization review
- Documentation updates
- Stakeholder reporting

### Security Maintenance

#### Access Control Review
- Audit governance multisig composition
- Rotate emergency access keys
- Review and update authorization lists
- Validate contract ownership

#### Vulnerability Assessment
- Run automated security scans
- Review dependency updates
- Assess new threat vectors
- Update security policies

### Performance Optimization

#### Gas Optimization (EVM)
- Monitor contract gas usage
- Optimize hot paths
- Update compiler settings
- Consider contract upgrades

#### Execution Optimization (WASM)
- Profile contract execution times
- Optimize WASM binary sizes
- Update CosmWasm versions
- Tune runtime parameters

## Contact Information

### Emergency Contacts
- **Security Team**: security@vagusprotocol.com
- **DevOps/SRE**: sre@vagusprotocol.com
- **Development Team**: dev@vagusprotocol.com

### Escalation Paths
1. First responder acknowledges alert within 5 minutes
2. Escalate to senior SRE within 15 minutes for P0 incidents
3. Escalate to CTO within 30 minutes for critical security issues
4. Escalate to full incident response team within 1 hour

### Communication Channels
- **Alerts**: PagerDuty, OpsGenie
- **Chat**: Slack (#incidents, #sre)
- **Documentation**: Internal wiki, GitHub
- **External**: Status page, Twitter

## Appendices

### A. Metric Definitions
### B. Alert Configuration
### C. Deployment Scripts
### D. Recovery Playbooks
### E. Contact Lists
