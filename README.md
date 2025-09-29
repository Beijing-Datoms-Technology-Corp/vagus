# Vagus — A Vagal-Nerve Layer for Safer Agents

Vagus 在 LLM/Planner 与执行器之间引入链上"迷走神经层"，用 **传入证据(Afferent)**、**张力(VTI)与三态(ANS)**、**制动(Vagal Brake)**、**反射弧(Reflex)**、**短时效能力令牌(Capability Token)** 来防止不对齐与危险动作。

## Components

- **contracts/**: Solidity/Foundry contracts (ANSState, AfferentInbox, VagalBrake, CapabilityIssuer, ReflexArc, …)
- **gateway/**: Rust device-side gateway (events, local VTI, AEP, CBF stub)
- **oracle/**: Rust Tone Oracle (VTI computation + ANS update)
- **planner/**: Python tools (schema → intent → EIP‑712)
- **schemas/**: YAML schemas & policies for mechanical arm (MVP)

## Quickstart

```bash
# 1) 启动本地链
./infra/devnet/anvil.sh

# 2) 部署合约
forge script script/DeployCore.s.sol --rpc-url http://127.0.0.1:8545 --broadcast

# 3) 运行 Tone Oracle
cargo run -p tone-oracle

# 4) 运行设备网关（模拟模式）
cargo run -p vagus-gateway -- --executor-id 12 --sim

# 5) 生成并提交一个 Intent（Python）
python -m planner.examples.send_move_to
```

## Development Status

- [x] M1 — 脚手架 + CI 绿灯
- [x] M2 — 核心合约可用（最小集）
- [x] M3 — 设备侧网关（Rust）骨架
- [x] M4 — Tone Oracle 服务（Rust）
- [x] M5 — Schema/Policy（机械臂最小集）
- [x] M6 — Portable Spec（链无关规格层）
- [x] M7 — CosmWasm 合约实现（WASM L1 最小闭环）
- [x] M8 — 网关/Oracle 多链客户端与 Relayer（本地双链）
- [x] M9 — 接口一致性测试（黄金规范套件）
- [x] M10 — 文档/脚本与演示
- [x] M11-M20 — Master Plan Complete! 🎉
  - [x] P0 — ANS 滞后修复（三态机 + 连续计数 + 最少驻留时间）
  - [x] T-1 — 规格补丁（I19-I26 不变量 + 新错误码 + 代码生成）
  - [x] T-2 — ER1/ER6 实现（VagalBrake + preStateRoot 一致性）
  - [x] T-3 — ER2 实现（Reflex 显式触发 + 分页撤销）
  - [x] T-4 — ER3/ER5 实现（WASM 授权 + TTL 统一秒级）
  - [x] T-5 — ER4 实现（CBOR 规范化 + 双哈希一致性）
  - [x] T-6 — ER7/ER8 实现（RateLimiter/CircuitBreaker + Safe+Timelock/cw3-dao）
  - [x] T-7 — 发布工程 + SRE（版本冻结 + SBOM + Runbook + 应急演练）

## Multichain Quick Start

Vagus now supports both EVM and CosmWasm chains! Here's how to get started:

### 1. Launch Dual-Chain Environment

```bash
# Start EVM + Cosmos chains, gateways, oracle, and relayers
./infra/devnet/up.sh
```

This starts:
- **Anvil (EVM)**: http://localhost:8545
- **wasmd (Cosmos)**: http://localhost:26657
- **Tone Oracle**: http://localhost:3000
- **Cross-chain Relayers**: Auto-sync events between chains

### 2. Deploy Contracts

```bash
# EVM contracts
cd contracts
forge script script/DeployCore.s.sol --rpc-url http://localhost:8545 --broadcast

# CosmWasm contracts (coming soon)
cd ../wasm-contracts
# Deploy scripts to be added
```

### 3. Run Cross-Chain Demo

```bash
# Execute full cross-chain capability lifecycle
./demo/scripts/cross-chain-demo.sh
```

This demonstrates:
1. Issue capability on EVM
2. Relay event to Cosmos
3. Detect danger on Cosmos
4. Cross-chain revocation

### 4. Run Golden Tests

```bash
# Test cross-chain invariants
cd tests/golden
cargo run -- run-all --evm-rpc http://localhost:8545 --cosmos-rpc http://localhost:26657
```

## Architecture

Vagus implements a safety layer inspired by the autonomic nervous system's vagal nerve, providing:

1. **Afferent Evidence Processing**: Device-side sensors feed real-time telemetry to the blockchain
2. **Autonomic Nervous System (ANS) State**: Three-state system (SAFE/DANGER/SHUTDOWN) with hysteresis
3. **Vagal Brake**: Dynamic scaling of action parameters based on current ANS state
4. **Reflex Arc**: Automated revocation of capabilities when dangerous conditions are detected
5. **Capability Tokens**: Short-lived, revocable permissions for specific actions

## License

Apache License 2.0