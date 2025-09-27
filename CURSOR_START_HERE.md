
# CURSOR_START_HERE.md

> **角色设定（给编程助理）**
> 你是“Vagus”项目的首席代码生成与集成助理。请严格遵循本文件，按里程碑逐步创建文件、目录、代码与测试。所有生成的代码必须可编译、可测试，并通过本文件定义的验收标准。

---

## 0) 项目愿景与核心抽象（速览）

* **Vagus = “迷走神经层”**：在 LLM/Planner 与执行器（机械臂/无人机/服务器）之间，提供**传入（Afferent）证据收集、张力评估与三态管理（SAFE/DANGER/SHUTDOWN）、制动（Vagal Brake）、反射弧（Reflex）、能力令牌授权（Capability Token）**。
* **MVP 技术栈（必须）**：

  * **智能合约**：Solidity + Foundry（EVM 兼容 L2）；
  * **设备侧网关**：Rust（ethers-rs / alloy / tokio）；
  * **Tone Oracle 服务**：Rust（axum）或 Go（二选一，默认 Rust）；
  * **Planner/工具**：Python（pydantic + web3.py，用于生成意图与本地静态校验）；
  * **Schema/Policy**：YAML/JSON + 代码生成器（Python）；
  * **索引/看板（可选）**：subgraph 或自研轻量索引器（后续里程碑）。

---

## 1) 目录结构（初始脚手架）

创建如下结构与关键文件（不存在则新建）：

```
vagus/
├─ README.md
├─ LICENSE
├─ .cursorrules
├─ .editorconfig
├─ .gitignore
├─ .github/
│  └─ workflows/
│     ├─ contracts-ci.yml
│     ├─ rust-ci.yml
│     └─ python-ci.yml
├─ contracts/                # Foundry
│  ├─ foundry.toml
│  ├─ lib/
│  ├─ src/
│  │  ├─ core/               # 最小可用合约组
│  │  │  ├─ Types.sol
│  │  │  ├─ ExecutorRegistry.sol
│  │  │  ├─ SchemaRegistryIface.sol
│  │  │  ├─ PolicyHubIface.sol
│  │  │  ├─ AfferentInbox.sol
│  │  │  ├─ ANSStateManager.sol
│  │  │  ├─ CapabilityIssuer.sol
│  │  │  ├─ VagalBrake.sol
│  │  │  ├─ ReflexArc.sol
│  │  │  └─ Events.sol
│  │  └─ extras/             # 后续里程碑
│  │     ├─ Coordinator.sol
│  │     ├─ HomeostasisBudget.sol
│  │     ├─ RateLimiter.sol
│  │     ├─ CircuitBreaker.sol
│  │     └─ Governance.sol
│  ├─ script/
│  │  ├─ DeployCore.s.sol
│  │  └─ DevnetConfig.json
│  └─ test/
│     ├─ ANSStateManager.t.sol
│     ├─ CapabilityIssuer.t.sol
│     ├─ VagalBrake.t.sol
│     └─ ReflexArc.t.sol
├─ gateway/                  # 设备侧网关（Rust）
│  ├─ Cargo.toml
│  └─ crates/
│     ├─ vagus-gateway/
│     ├─ vagus-telemetry/
│     └─ vagus-crypto/
├─ oracle/                   # Tone Oracle 服务（Rust）
│  ├─ Cargo.toml
│  └─ tone-oracle/
├─ planner/                  # Planner/工具（Python）
│  ├─ pyproject.toml
│  ├─ vagus_planner/
│  │  ├─ __init__.py
│  │  ├─ intents.py
│  │  ├─ schemas.py
│  │  ├─ eip712.py
│  │  └─ validate.py
│  └─ tests/
├─ schemas/                  # YAML/JSON Schema & Policy
│  ├─ mechanical_arm/
│  │  ├─ actions.yaml       # MOVE_TO/GRASP等
│  │  └─ policy.yaml
│  └─ common/
│     └─ units.yaml
├─ docs/
│  ├─ VagusSpec.md
│  ├─ Architecture.md
│  └─ ThreatModel.md
└─ infra/
   ├─ docker/
   │  ├─ Dockerfile.contracts
   │  ├─ Dockerfile.gateway
   │  └─ Dockerfile.oracle
   └─ devnet/
      └─ anvil.sh
```

---

## 2) 里程碑（Milestones）与“定义完成”（DoD）

> **要求**：逐个完成；每个里程碑结束时，CI 需绿灯，通过构建与测试，并更新 `README.md` 的状态表。

### M1 — 脚手架 + CI 绿灯

* 目标：

  1. 完成目录&文件初始化（见上）。
  2. Foundry/ Rust / Python 三线 CI 工作流可运行（编译 + 基础测试 + lints）。
  3. `README.md` 有清晰的“项目简介 + 快速开始 + 合约部署脚本”。

* DoD：

  * `forge build`、`forge test` 通过；
  * `cargo test`（gateway/oracle 子项目空测试）通过；
  * `pytest`（planner 空测试）通过；
  * GitHub Actions 全绿。

### M2 — 核心合约可用（最小集）

* 目标：实现并测试下列合约的**可编译 + 基本行为**：

  * `Types.sol`：`Intent`/`TokenMeta`/常量；
  * `Events.sol`：标准化事件；
  * `AfferentInbox.sol`：记录可信状态根（AEP 摘要），带基本访问控制；
  * `ANSStateManager.sol`：`SAFE/DANGER/SHUTDOWN` 三态 + 张力（VTI）更新 + 回滞（hysteresis）；
  * `CapabilityIssuer.sol`：签发/撤销 Capability（ERC‑721 语义即可）；
  * `VagalBrake.sol`：调用 Issuer 前的**缩放与拦截**；
  * `ReflexArc.sol`：基于 AEP 指标触发**批量撤销**；
  * `SchemaRegistryIface.sol`、`PolicyHubIface.sol`：接口占位（MVP 使用 mock）。

* DoD：

  * 单元测试覆盖：

    * `ANSStateManager`：阈值/回滞/驻留时间；
    * `CapabilityIssuer`：签发/TTL/撤销；
    * `VagalBrake`：根据 VTI 缩放限制并拒绝越界参数；
    * `ReflexArc`：模拟危险 AEP → 撤销生效 token；
  * `script/DeployCore.s.sol` 一键部署本地 devnet；
  * `anvil` 本地链脚本运行成功。

### M3 — 设备侧网关（Rust）骨架

* 目标：

  * `vagus-gateway`：

    * 订阅 `CapabilityIssued/Revoked` 事件；
    * 校验 token TTL/缩放限制哈希；
    * 打包并提交 AEP（含最小指标：min human distance / temp / energy / jerk 的摘要），上报到 `AfferentInbox`；
    * 本地 **VTI 计算**（简化公式）与**CBF 接口占位**；
  * `vagus-crypto`：EIP‑712 验签/签名工具；
  * `vagus-telemetry`：遥测数据结构 + 哈希承诺。

* DoD：

  * 本地 e2e：部署合约 → 脚本签发 token → 网关监听并“执行”（打印模拟）→ 上报 AEP → Reflex 可触发撤销。

### M4 — Tone Oracle 服务（Rust）

* 目标：

  * `tone-oracle` 服务：接收网关送来的窗口指标（HTTP/WS），计算 **VTI**，按策略推送到 `ANSStateManager.updateTone`；
  * 支持 `SAFE/DANGER/SHUTDOWN` 的建议转换，并记录回滞参数。

* DoD：

  * e2e：改变输入指标 → VTI 变化 → 合约状态切换 → `VagalBrake` 缩放系数随之生效（测试验证）。

### M5 — Schema/Policy（机械臂最小集）

* 目标：

  * `schemas/mechanical_arm/actions.yaml`：定义 `MOVE_TO/GRASP` 字段、单位、上下界、可制动字段列表；
  * `schemas/mechanical_arm/policy.yaml`：DANGER/SAFE 下的差异策略（速度/力缩放、禁用锐器抓取）；
  * `planner`：实现意图构造、Schema 解析、静态校验、EIP‑712 打包。

* DoD：

  * 用 `planner` 生成 Intent → 通过 `VagalBrake` → `CapabilityIssuer` 签发；
  * 反例：越界参数/缺失字段被拒绝。

> 后续 M6+：RateLimiter、Coordinator、HomeostasisBudget、治理与审计看板等，按 roadmap 推进。

---

## 3) 关键接口与最小规范（供实现时对照）

> **注意**：以下为**必须实现的最小接口/事件**，字段可根据需要扩展，但不得更名或更改语义。

### 3.1 `Types.sol`（最小）

* `struct Intent { uint256 executorId; bytes32 actionId; bytes params; bytes32 envelopeHash; bytes32 preStateRoot; uint64 notBefore; uint64 notAfter; uint32 maxDurationMs; uint32 maxEnergyJ; address planner; uint256 nonce; }`
* `struct TokenMeta { ...; bool revoked; }`

### 3.2 统一事件（`Events.sol`）

* `event CapabilityIssued(uint256 indexed tokenId, uint256 indexed executorId, bytes32 indexed actionId, bytes32 paramsHash, uint64 notAfter);`
* `event CapabilityRevoked(uint256 indexed tokenId, uint8 reason);`
* `event AEPPosted(uint256 indexed executorId, bytes32 stateRoot, bytes32 metricsHash);`
* `event VagalToneUpdated(uint256 tone, uint8 state);`
* `event ReflexTriggered(uint256 indexed executorId, bytes32 reason, uint256[] revoked);`

### 3.3 `ANSStateManager.sol`

* `enum State { SAFE, DANGER, SHUTDOWN }`
* `function updateTone(uint256 tone, State suggested) external;`

  * **含回滞/最短驻留**；对外 `view guardFor(bytes32 actionId)` 返回缩放系数与 allow。

### 3.4 `VagalBrake.sol`

* `issueWithBrake(Intent it) returns (uint256 tokenId)`：

  * 读取 `guardFor(it.actionId)`，检查/缩放可制动字段（如 `vMax/forceLimit/maxDuration/maxEnergy`）；
  * 不满足则 revert：`"ANS:blocked"`/`"ANS:limit"`。

### 3.5 `CapabilityIssuer.sol`

* `issueCapability(Intent it) returns (uint256 tid)`：

  * 校验 TTL、前置状态匹配（从 AfferentInbox/Oracle 获取）；
  * 记录 `scaledLimitsHash`（由 VagalBrake 计算传入）；
* `revoke(uint256 tid, uint8 reason)`；
* `isValid(uint256 tid) view returns (bool)`；
* `activeTokensOf(executorId) view returns (uint256[])`（供 Reflex 方便撤销）。

### 3.6 `AfferentInbox.sol`

* `postAEP(executorId, stateRoot, metricsHash, attestation)` 验签 & 存档；
* `latestStateRoot(executorId) view returns (bytes32)`。

### 3.7 `ReflexArc.sol`

* `onAEP(...)` 内部解析/阈值判断 → 调用 `CapabilityIssuer.revoke` 批量撤销；
* 需有基础**节流**以防 DoS。

---

## 4) 合约测试要点（Foundry）

* **ANS 回滞测试**：多次上下抖动的 VTI 输入，验证状态机不抖动；
* **缩放不可变式**：`state==DANGER ⇒ token.limits ≤ SAFE基线 × VTI`；
* **反射时效**：模拟危险 AEP，上链时间戳差值 ≤ 设定上限（例如 2 块内）；
* **撤销覆盖**：撤销后 `isValid(tid)==false` 且新发令需失败或受限；
* **事件与索引**：所有关键步骤必须发事件，字段一致。

---

## 5) Rust 网关要点

* 订阅合约事件（ws 或 polling backoff）；
* **AEP** 构造：窗口统计（min human distance、温度、能量剩余、jerk 归一化）；
* **VTI 本地计算**（与 oracle 一致的简化版），用于**就地限速/限力**（即使链上允许，本地仍可更严）；
* **CBF 接口**：trait 占位 `fn guard(setpoint: Pose, sensors: &Sensors) -> Pose`；
* **签名/验签**：EIP‑712 工具；
* 失败重试与幂等（nonce/去重）。

---

## 6) Tone Oracle 要点

* HTTP/WS 接口接收窗口指标，计算 `VTI in [0,1]`：

  * 建议：`VTI = Σ w_i * m_i`（人距余裕、温度余裕、能量余裕、jerk、环境扰动等），权重配置化；
* 回滞/驻留：低于阈值进入 DANGER，恢复必须超更高阈值；
* 周期性上链 `updateTone(VTI, suggestedState)`，失败重试 & metrics 记录。

---

## 7) Planner/Schema 工具

* 读取 `schemas/*/actions.yaml`、`policy.yaml`；
* 生成 `Intent`（pydantic）并做**静态校验**（单位正规化、上下界、必填、互斥）；
* EIP‑712 打包 & 签名（供链上核验）；
* 示例：机械臂 `MOVE_TO`、`GRASP`。

---

## 8) 代码质量与安全

* Solidity：启用 `forge fmt` / `forge coverage`，Slither 静态分析；
* Rust：`clippy`、`rustfmt`、单元/集成测试；
* Python：`ruff`、`mypy`、`pytest`；
* 安全策略：`SECURITY.md` + `ThreatModel.md`（最小不可变式 I1–I5 记录）。

---

## 9) 快速开始（开发者）

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

---

## 10) 任务分解（按顺序把下面交给 Cursor 执行）

> 将每一条作为“指令”逐条执行，完成后提交变更并更新 `README.md` 进度表。

1. **初始化脚手架与 CI**

   * 生成上述目录/文件骨架；
   * 填充 `.github/workflows/*`（Foundry/Rust/Python）最小 CI；
   * 写 `README.md`：项目摘要、组件、快速开始；
   * 许可证：Apache-2.0。

2. **实现合约最小集（M2）**

   * 完成 `Types.sol / Events.sol / AfferentInbox.sol / ANSStateManager.sol / CapabilityIssuer.sol / VagalBrake.sol / ReflexArc.sol`；
   * 写 `DeployCore.s.sol`；
   * 为每个合约写 2–3 个核心单测；
   * 本地 `forge test` 通过。

3. **Rust 网关骨架（M3）**

   * `vagus-gateway`：事件监听、AEP 上报、VTI 本地计算、CBF 占位；
   * `cargo test` 通过；
   * 增加一个 e2e 集成测试（使用 anvil）。

4. **Tone Oracle（M4）**

   * `tone-oracle`：最小 HTTP 接口 + 上链 `updateTone`；
   * 添加阈值与回滞配置；
   * 集成测试：喂入高/低指标，观察合约状态切换与 `VagalBrake` 缩放变化。

5. **Schema/Policy & Planner（M5）**

   * 填充 `schemas/mechanical_arm/*`；
   * `planner` 读取 schema → 生成 Intent → 通过 `VagalBrake` → `CapabilityIssuer` 签发；
   * 增加一条失败用例（越界参数应被拒绝）。

---

# .cursorrules（放在仓库根目录）

> **目标**：规范代码风格、约束生成内容、提醒编程助理遵循设计与安全边界。

```
# === Global ===
- Always write compilable, runnable code with minimal external assumptions.
- Prefer smallest viable slices: implement skeletons first, then iterate.
- Every contract/service MUST emit the canonical events defined in /contracts/src/core/Events.sol.
- Never silently change event names or struct field names once introduced.
- When adding a new param limit, also add a test that fails when exceeding it.

# === Solidity ===
- Version: ^0.8.24; OZ only if necessary.
- Gas/readability tradeoff: prioritize clarity for MVP.
- Use custom errors over require strings where reasonable.
- All external functions must be annotated with NatSpec.
- Tests: Foundry; include invariant tests for ANS state machine hysteresis and capability revocation semantics.
- Security checks: reentrancy guards where state writes follow external calls; no unbounded loops over user-controlled arrays.

# === Rust (gateway/oracle) ===
- Edition 2021; use tokio, anyhow, thiserror, tracing.
- HTTP server (oracle): axum; Client: reqwest.
- EVM bindings: ethers-rs or alloy; prefer typed Abigen.
- Provide feature flags: `sim` for simulated sensors, `hw` placeholder for real hardware.
- Include integration test against an anvil devnet spun up in-test (use `anvil::spawn` if available or docker fallback).

# === Python (planner) ===
- Python 3.11; use pydantic for schemas.
- Strict type checking (mypy), lint via ruff; tests via pytest.
- Never send raw JSON to chain: sign EIP-712 typed data and match Solidity domain separator.

# === Schemas/Policies ===
- YAML files must specify units and bounds; mark "brakeable" fields explicitly.
- Provide a codegen step that produces a "scaledLimitsHash" given an Intent and Guard.

# === CI ===
- Workflows must run on push/PR; fail on lint/test failures.
- Cache builds for Foundry, cargo, and pip.

# === Documentation ===
- Update README.md when a milestone completes; include command snippets that actually work.
- Keep docs consistent with emitted events and function names.

# === Commit ===
- Conventional Commits; include scope (contracts/gateway/oracle/planner/schemas/docs/infra).
```

---

## 附：README.md 开头模板（可由 Cursor 直接写入）

````md
# Vagus — A Vagal-Nerve Layer for Safer Agents

Vagus 在 LLM/Planner 与执行器之间引入链上“迷走神经层”，用 **传入证据(Afferent)**、**张力(VTI)与三态(ANS)**、**制动(Vagal Brake)**、**反射弧(Reflex)**、**短时效能力令牌(Capability Token)** 来防止不对齐与危险动作。

## Components
- **contracts/**: Solidity/Foundry contracts (ANSState, AfferentInbox, VagalBrake, CapabilityIssuer, ReflexArc, …)
- **gateway/**: Rust device-side gateway (events, local VTI, AEP, CBF stub)
- **oracle/**: Rust Tone Oracle (VTI computation + ANS update)
- **planner/**: Python tools (schema → intent → EIP‑712)
- **schemas/**: YAML schemas & policies for mechanical arm (MVP)

## Quickstart
```bash
./infra/devnet/anvil.sh
forge script script/DeployCore.s.sol --rpc-url http://127.0.0.1:8545 --broadcast
cargo run -p tone-oracle
cargo run -p vagus-gateway -- --executor-id 12 --sim
python -m planner.examples.send_move_to
````

```

---

**交付方式**：  
1）将本文件保存为 `CURSOR_START_HERE.md`；  
2）将 `.cursorrules` 内容保存为仓库根目录文件；  
3）逐条把“任务分解”指令提交给 Cursor 执行；  
4）每个里程碑结束后，确保 CI 绿灯并更新 `README.md` 状态。

若需要，我也可以在下一步直接给出 **M2 合约最小实现（代码骨架 + Foundry 测试）** 与 **M3 网关事件循环样例** 的具体内容。
```
