# CURSOR_MULTICHAIN_TASKS.md

> **角色设定（给编程助理）**
> 你是 Vagus 多链化负责人。目标：在不牺牲安全性与可审计性的前提下，把核心合约与外围组件迁移为**可在 EVM 与 WASM L1 上运行**。
> 参考对象：EVM（Solidity/Foundry）与 **CosmWasm（Rust）**。Polkadot/ink! 作为后续变体预留接口。

---

## 0) 背景与范围

* 现状：M1–M5 已完成（EVM 合约 + Rust 网关 + Rust Tone Oracle + Python Planner + Schema/Policy）。
* 目标：

  1. 将核心合约组在 WASM L1 上（优先 CosmWasm）提供**等价语义的实现**；
  2. 提供一套**链无关的规格层（Portable Spec）**，用于代码生成与一致性测试；
  3. 网关与 Oracle 统一“链客户端接口”，可同时或分别连接 EVM/WASM；
  4. 提供**跨链事件/状态桥接**的可插拔适配层（本期实现 Mock Relayer + 本地双链 e2e）；
  5. CI 扩展，确保两套实现一致通过**同一组规范测试**与不可变式（I1–I5）。

---

## 1) 重要差异与设计原则（实施前请牢记）

| 主题     | EVM (Solidity)          | WASM (CosmWasm)       | 适配方案                                                                |
| ------ | ----------------------- | --------------------- | ------------------------------------------------------------------- |
| 地址类型   | 20 bytes, `address`     | bech32/字符串            | 统一本地 `Identity` 结构：`ChainId + Variant(EVM{20B}, Cosmos{String})`    |
| 事件     | Logs/Topics             | Attributes/Events     | 统一事件模型：`EventKind + kv[]`；在 EVM 解析 logs，在 CosmWasm 解析 attributes    |
| 时间     | `block.timestamp` (sec) | `env.block.time` (ns) | 统一 `Timepoint`（秒），CosmWasm 向下取整                                     |
| 哈希     | keccak256               | sha256（常用）            | **Commit 双哈希**：`keccak` 与 `sha256` 都生成并存储；或统一 CBOR→sha256，EVM 侧再存映射 |
| 签名     | EIP‑712/`ecrecover`     | 合约内验签成本高              | **链级身份优先**：以 `tx.sender==planner` 为准；EIP‑712 仅作记录/审计（链下验）           |
| NFT    | ERC‑721                 | cw721                 | `CapabilityToken`：EVM 实现 ERC‑721；CosmWasm 以 `cw721` 子集实现            |
| Gas/存储 | 便宜日志，较贵存储               | 事件属性便宜，存储按字节计费        | 尽量**链下存证 + 上链根**；参数/限制做紧凑编码                                         |
| 访问控制   | `onlyOwner`/roles       | `info.sender`/`cw4`   | 抽象 `Role/Authority` 接口，EVM 与 CosmWasm 各自实现                          |

---

## 2) 里程碑（M6–M10）

### **M6 — Portable Spec（链无关规格层）**

**目标**：把核心“类型/事件/不可变式/错误码”抽象到一个语言/链中立的规范中，用以**代码生成**与**一致性测试**。

**任务**

1. 新建目录 `spec/`：

   * `spec/types.yml`：`Intent`, `TokenMeta`, `Guard`, `AEP`, `VTI`, `State{SAFE,DANGER,SHUTDOWN}` 等字段、单位、取值范围；
   * `spec/events.yml`：`CapabilityIssued`, `CapabilityRevoked`, `AEPPosted`, `VagalToneUpdated`, `ReflexTriggered` 的标准键值；
   * `spec/invariants.yml`：I1–I5 不可变式与前置/后置条件文本化；
   * `spec/errors.yml`：标准错误码，如 `ANS_BLOCKED`, `ANS_LIMIT`, `TTL`, `STATE_MISMATCH`；
2. 在 `planner/` 增加 `vagus_planner/codegen.py`：

   * 从 `spec/*.yml` 生成：

     * `contracts/src/core/Types.sol` 的 `struct`/`error`，
     * `wasm-contracts/cosmwasm/packages/vagus-spec/src/lib.rs` 的 `struct`/`enum`，
     * `gateway` 与 `oracle` 共享的 Rust 类型（`vagus-spec` crate）；
   * 生成**事件键名常量**，避免拼写漂移；
3. 在 CI 中新增一个 job：**Spec Drift Check**（若手写代码的字段/事件与 spec 不一致则失败）。

**DoD**

* `forge build`、`cargo build -p vagus-spec` 通过；
* `codegen.py` 可重复运行且幂等；
* `.github/workflows/*` 新增 spec-drift 阶段并绿灯。

---

### **M7 — CosmWasm 合约实现（WASM L1 最小闭环）**

**目标**：在 `wasm-contracts/` 下实现 CosmWasm 版本的核心合约：`AfferentInbox`、`ANSStateManager`、`CapabilityIssuer(cw721)`, `VagalBrake`, `ReflexArc`。功能与 EVM 对齐。

**任务**

1. 新建目录结构：

```
wasm-contracts/
├─ Cargo.toml           # workspace
├─ cosmwasm/
│  ├─ packages/vagus-spec/         # 由 codegen 生成/维护
│  ├─ contracts/afferent_inbox/
│  ├─ contracts/ans_state_manager/
│  ├─ contracts/capability_issuer/ # 依赖 cw721-base 精简版
│  ├─ contracts/vagal_brake/
│  └─ contracts/reflex_arc/
└─ tests/
   └─ integration.rs
```

2. **AfferentInbox**

   * `ExecuteMsg::PostAEP { executor_id, state_root_sha256, state_root_keccak, metrics_hash_sha256, metrics_hash_keccak, attestation }`；
   * 只允许白名单 `oracle`/`gateway` 地址调用；
   * 保存**最近 N 条**摘要（默认1条），查询 `latest_state_root(executor_id)`；
3. **ANSStateManager**

   * `ExecuteMsg::UpdateTone { vti, suggested }` 带回滞（可配置阈值与最小驻留时间）；
   * `QueryMsg::GuardFor { action_id } -> Guard`；
4. **CapabilityIssuer**（cw721 子集）

   * `ExecuteMsg::Issue { intent, scaled_limits_hash } -> token_id`；
   * `ExecuteMsg::Revoke { token_id, reason }`；
   * `QueryMsg::IsValid { token_id } -> bool`；
   * `QueryMsg::ActiveTokensOf { executor_id } -> Vec<token_id>`；
   * **注意**：`planner` 权限 = `info.sender` 白名单（避免链上验签）；
5. **VagalBrake**

   * 只接受对 `CapabilityIssuer` 的代理调用：`IssueWithBrake { intent }`；
   * 读取 `ANS.guard_for(action)`，检查/缩放**可制动字段**（来自 `spec/types.yml` 的标注）；
6. **ReflexArc**

   * `ExecuteMsg::OnAEP { executor_id, metrics }`：若触发规则阈值 → 批量 `Revoke`；带节流；
7. **共性**：所有事件以**attributes** 形式发出，键名与 `spec/events.yml` 一致。
8. 在 `wasm-contracts/tests/integration.rs` 用 `cw-multi-test` 写最小 e2e：

   * 部署五个合约 → 发 `IssueWithBrake` → `AEP` 触发 → `ReflexArc` 撤销。

**DoD**

* `cargo build --target wasm32-unknown-unknown` 通过；
* `cargo test -p wasm-contracts` 通过；
* 事件属性键名与 `spec/events.yml` 一致（集成测试断言）。

---

### **M8 — 网关/Oracle 多链客户端与 Relayer（本地双链）**

**目标**：让 `gateway` 与 `tone-oracle` 既能对接 EVM，也能对接 CosmWasm；提供一个最小**跨链中继（Relayer）**，将关键事件在两条链之间同步（本期使用 Mock）。

**任务**

1. 在 `gateway/` 新建 crate `vagus-chain/`：

   * 提供 `trait ChainClient { fn submit_aep(...); fn issue_with_brake(...); fn revoke(...); fn subscribe_events(...); }`；
   * `features = ["evm", "cosmos"]` 两个实现：

     * **EVM**：沿用 `ethers-rs`；
     * **Cosmos**：使用 `cosmrs`（Tx 构造、查询、事件订阅使用 RPC/WebSocket 或轮询）；
   * 统一事件解码为 `vagus_spec::Event`；
2. 修改 `vagus-gateway`：

   * 增加 CLI：`--chain evm|cosmos` 与多实例支持；
   * AEP/执行路径不变；
3. 修改 `tone-oracle`：

   * 增加 `--evm-rpc` 与 `--cosmos-rpc` 两个输出目标；
   * `updateTone` 可选择只上到某条链或双发；
4. 新建 `relayer/`（Rust）：

   * 监听一条链的核心事件（`VagalToneUpdated`, `CapabilityIssued/Revoked`, `AEPPosted`），用**幂等**策略转发到另一条链；
   * 采用**事件去重**（事件哈希 + 时间窗）；
5. `infra/devnet` 增加：

   * `wasmd.sh`：拉起本地 `wasmd`/`osmosisd`（或 `simd`） + 账户初始化；
   * `compose.yaml`：同时启动 anvil + wasmd + relayer + oracle（可选）；
6. CI 新增 `wasm-ci.yml`：

   * 构建 CosmWasm 合约并跑单元测试；
   * 跳过重型集成测试（devcontainer 里再跑）。

**DoD**

* 本地脚本：`anvil.sh && wasmd.sh` → `relayer` 将 `CapabilityIssued` 从 EVM 同步到 CosmWasm（打印日志验证）；
* `gateway` 在 `--chain=cosmos` 模式下成功 `postAEP`；
* `tone-oracle` 可向两条链发布 `updateTone`（可配置单/双发）。

---

### **M9 — 接口一致性测试（黄金规范套件）**

**目标**：对 EVM 与 CosmWasm 的实现套用**同一规格测试**，验证功能/不可变式一致。

**任务**

1. 在 `tests/` 新建 `golden/`：

   * 以 Rust 写跨链测试库，分别调用 EVM 与 CosmWasm 客户端；
   * 断言 I1–I5：

     * `SHUTDOWN ⇒ 无有效非逃逸 Token`；
     * `DANGER ⇒ token.limits ≤ SAFE基线 × VTI`；
     * 反射撤销延迟 ≤ 配置上限；
     * Envelope ⊆ NoGo 补集（模拟校验 + 哈希比对）；
     * CBF 投影安全（用模拟器桩）；
2. 为**事件等价性**写测试：同一操作在两条链产生的事件/属性键值需匹配 `spec/events.yml`；
3. **Fuzz/Property**：Foundry + Rust `proptest`，对阈值边界/回滞抖动进行随机测试。

**DoD**

* `make golden-test`（或 `cargo test -p golden`）一次跑完两条链的等价性测试（CosmWasm 用 `cw-multi-test`，EVM 用 anvil）。

---

### **M10 — 文档/脚本与演示**

**任务**

1. `docs/Multichain.md`：

   * 架构图（EVM/WASM 并行）、事件对照表、不可变式说明；
   * Devnet 启动流程与典型交互脚本；
2. `README.md`：新增**多链快速开始**；
3. `infra/devnet/`：一键脚本 `up.sh` 启动 anvil + wasmd + oracle + gateway + relayer；
4. Demo：

   * 发一个 `MOVE_TO`（EVM）→ Issued；
   * Relayer 同步到 CosmWasm；
   * Gateway 在 Cosmos 上报 AEP → Reflex 在 Cosmos 撤销；
   * Relayer 再把撤销同步回 EVM。

**DoD**

* 一键脚本在干净环境可跑通 Demo；
* 文档覆盖“常见陷阱”（时间单位、哈希、事件键名、签名差异）。

---

## 3) 对现有代码库的结构变更

在仓库根新增/修改：

```
spec/                              # 链无关规格（代码生成源）
wasm-contracts/                    # CosmWasm 实现
relayer/                           # 跨链中继
gateway/crates/vagus-chain/        # 多链客户端
docs/Multichain.md                 # 文档
infra/devnet/wasmd.sh              # 本地Cosmos链
infra/devnet/up.sh                 # 一键双链
.github/workflows/wasm-ci.yml      # CI
```

---

## 4) 统一编码/哈希/时间规范

* **时间**：所有意图与 token TTL 以**秒**为单位；CosmWasm 读 `env.block.time`（纳秒）后向下取整。
* **哈希**：对入参与度量使用**CBOR 严格规范化**后做 `sha256`，同时在 EVM 侧再存一个 `keccak(cbor)`；事件内带两个哈希字段：`*_sha256` 与 `*_keccak`。
* **身份**：链上**以 tx.sender 为权威**（`planner` 白名单）；EIP‑712 用于链下审计与回放，不作为 CosmWasm 合约验签前提。
* **事件键**：严格使用 `spec/events.yml` 的键名；EVM 事件 `topics[0]` 映射 `kind`，CosmWasm attributes 的 `event` 映射同名。

---

## 5) 对网关与 Oracle 的升级要点

* 网关新增 `--chain`、`--rpc`、`--key` 参数；把 AEP 上链与事件订阅改为走 `vagus-chain` 抽象。
* 本地 VTI 计算逻辑不变；**在 Cosmos 下也执行就地限速/限力**。
* Oracle 支持**双通道上链**，并记录两侧 txhash 对照以便审计。

---

## 6) 安全与不可变式（多链补丁）

* **I6（一致性）**：同一 `CapabilityToken` 在两条链上的撤销结果必须在 ΔT 窗口内一致（由 relayer 保障）。
* **I7（最小权限）**：CosmWasm 合约对外 `ExecuteMsg` 必须检查 `info.sender` 是否在 `oracle/gateway` 白名单。
* **I8（重放防护）**：跨链消息携带源链 id 与事件序号，relayer 落地**去重表**（只保留滚动窗口）。
* **I9（时间漂移）**：若链间时间差 > 阈值，`VagalBrake` 在从属链进入 `DANGER` 保守模式。

---

## 7) 交付验收（一次性清单）

* ✅ `wasm-contracts` 构建与单测通过；
* ✅ `relayer` 能把 EVM 的 `CapabilityIssued` 同步为 CosmWasm 的“镜像记录”（或直接触发对应动作）；
* ✅ `gateway --chain=cosmos` 能 `postAEP` 并驱动 `ReflexArc`；
* ✅ `golden` 等价性测试通过；
* ✅ 文档与一键脚本可跑通 Demo。

---

## 8) 任务执行顺序（交给 Cursor 逐条执行）

1. **创建分支**：`feat/multichain-wasm`；初始化 `spec/`，实现 codegen 与 drift check（M6）。
2. **实现 CosmWasm 五合约**与 `cw-multi-test` 集成测试（M7）。
3. **抽象链客户端**并改造网关/Oracle；实现 `relayer/`（M8）。
4. **编写 golden 一致性测试**：不可变式与事件对照（M9）。
5. **文档 + 一键脚本 + 演示**（M10）。
6. 发起 PR，确保 CI 四套（contracts-ci / rust-ci / python-ci / wasm-ci）全绿；更新 `README.md` 进度表。

---

# 附：更新 `.cursorrules`（新增多链与黄金编码原则）

请把以下内容**追加**到根目录 `.cursorrules`：

```
# === Multichain Rules ===
- Never fork semantics between EVM and WASM: use /spec/*.yml as single source of truth and run drift checks.
- Always emit both sha256 and keccak commitments for params/metrics (names end with _sha256/_keccak).
- Time is seconds. CosmWasm must floor env.block.time (ns) to seconds before comparisons.
- Identity: prefer tx.sender (whitelist) for authorization on both chains; EIP-712 signatures are for off-chain audit.
- Event keys MUST match spec/events.yml exactly. Add tests that read chain logs/attributes and compare keys/values.

# === Relayer ===
- Ensure idempotency: de-duplicate by (src_chain_id, block_height, tx_hash, log_index_or_attr_index).
- Backoff with jitter; never hammer endpoints.
- Implement "fail closed": when relayer is down, WASM side defaults to DANGER guard; never auto-escalate to SAFE.

# === Golden Coding Principles ===
1) **Correctness first**：以不可变式和规范为最高优先级；模棱两可时选择“拒绝执行”。  
2) **Fail closed**：所有校验失败、桥接异常、时间漂移、预言机掉线，一律阻断或降级到 DANGER/SHUTDOWN。  
3) **Small, testable slices**：小步提交，每个模块都有独立单元测试与最小 e2e。  
4) **Single source of truth**：规格→代码生成→实现，禁止手写重复类型/事件。  
5) **Determinism**：所有链上状态转移与哈希承诺具有确定性；禁止使用非确定性随机。  
6) **Explicit units**：所有物理量携带单位，跨链时做相同单位换算与边界测试。  
7) **No silent fallback**：遇到不支持的特性/链差异，必须记录事件并返回明确错误码。  
8) **Observability**：每个关键路径发事件/日志，便于审计与回放；绝不省略。  
9) **Security-by-default**：最小权限、白名单、重放与节流；避免可重入与无界循环。  
10) **Docs-as-code**：每个公共接口/消息体都在 `docs/` 中有示例与边界说明；更新文档是验收的一部分。
```

---

## 结语

以上提示词将把 **Vagus** 升级为**双栈（EVM + WASM）**的“迷走神经层”。请按 **M6 → M10** 顺序执行，并在每个里程碑结束时更新文档与 CI。
如需，我也可以在后续补充 **CosmWasm 合约样例骨架（ExecuteMsg/QueryMsg/State）** 与 **relayer 事件映射表**，用于直接落地实现。
