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
- [ ] M4 — Tone Oracle 服务（Rust）
- [ ] M5 — Schema/Policy（机械臂最小集）

## Architecture

Vagus implements a safety layer inspired by the autonomic nervous system's vagal nerve, providing:

1. **Afferent Evidence Processing**: Device-side sensors feed real-time telemetry to the blockchain
2. **Autonomic Nervous System (ANS) State**: Three-state system (SAFE/DANGER/SHUTDOWN) with hysteresis
3. **Vagal Brake**: Dynamic scaling of action parameters based on current ANS state
4. **Reflex Arc**: Automated revocation of capabilities when dangerous conditions are detected
5. **Capability Tokens**: Short-lived, revocable permissions for specific actions

## License

Apache License 2.0