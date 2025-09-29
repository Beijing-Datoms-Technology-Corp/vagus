> 保存为仓库根目录：`/CURSOR_L1_ON_VAGUS_CHAIN.md`（并追加 `.cursorrules`）。按里程碑顺序逐条执行，**每步完成即提交**，CI 必须绿灯。

### 0) 前置原则（黄金编码原则 —— 必须遵守）

1. **Correctness first**：以不可变式与规范为最高优先级；模棱两可时选择“拒绝执行”。
2. **Fail‑closed**：任何校验失败/预言机掉线/时间漂移/桥接异常 → 阻断或降级至 DANGER/SHUTDOWN。
3. **Single source of truth**：`/spec/*.yml` 为类型/事件/错误码**唯一来源**；代码由 codegen 生成骨架；CI 做 drift check。
4. **Determinism**：链上状态转移与哈希承诺必须确定性；参数 CBOR 规范化；双哈希（sha256+keccak）。
5. **Explicit units**：所有物理量带单位；跨 VM 边界做一致性与边界测试。
6. **Observability**：每个关键路径发事件/attributes，键名必须匹配 `/spec/events.yml`。
7. **Least privilege**：白名单/角色控制；无界循环与可重入路径一律禁止。
8. **No silent fallback**：不支持/异常必须显式错误码与事件。
9. **Small, testable slices**：小步快跑、单元 + e2e 测试覆盖。
10. **Docs-as-code**：接口与消息体在 `docs/` 有示例，更新文档是验收的一部分。

---

### 1) 目录与工程初始化（L1/WASM 目标）

**任务 M11：添加 WASM 合约工作区（默认 CosmWasm），与 Spec 驱动的代码生成**

1. 新增目录：

```
l1/
└─ wasm/
   ├─ Cargo.toml                 # workspace
   ├─ packages/vagus-spec/       # 由 codegen 生成（Rust 类型/事件/错误码）
   └─ contracts/
      ├─ executor_registry/
      ├─ ans_state_manager/
      ├─ afferent_inbox/
      ├─ vagal_brake/
      ├─ capability_issuer/
      └─ reflex_arc/
```

2. 在 `spec/` 下新增/补全（若已存在则补齐字段）：

   * `types.yml`：`Intent`, `TokenMeta`, `Guard`, `AEP`, `ANSState{SAFE,DANGER,SHUTDOWN}`, `ScaledLimits`, `Identity`。
   * `events.yml`：统一键名：`CapabilityIssued/CapabilityRevoked/AEPPosted/VagalToneUpdated/ReflexTriggered`。
   * `errors.yml`：`ANS_BLOCKED`, `ANS_LIMIT`, `TTL`, `STATE_MISMATCH`, `NOT_AUTHORIZED`, `RATE_LIMITED` 等。
   * `invariants.yml`：I1–I5（原有）+ I6–I9（跨链与 fail‑closed 补丁）。

3. 在 `planner/vagus_planner/codegen.py` 增强：

   * 生成 `l1/wasm/packages/vagus-spec/src/lib.rs`（Rust struct/enum/const），
   * 同步 EVM 侧 `contracts/src/core/Types.sol / Events.sol`（若已有则校验），
   * 生成 `gateway/crates/vagus-spec/`（共享 Rust 类型）。

4. 增加 CI Job：**Spec Drift Check**（比较生成文件与手写引用处的字段/事件键名），不一致则失败。

**DoD**

* `cargo build -p vagus-spec`（WASM 包）通过；
* 重新运行 codegen 幂等；
* CI 新增 drift check 并绿灯。

---

### 2) 合约实现（WASM，CosmWasm 语义；若 `vagus-chain` SDK 不同，请由适配层替换）

**任务 M12：六合约最小闭环（与 EVM 语义等价）**

> 所有合约：
>
> * `InstantiateMsg`：管理员/角色白名单；
> * `ExecuteMsg`：动作；
> * `QueryMsg`：只读；
> * **事件**：以 attributes 发出；键名来自 `spec/events.yml`；
> * **时间**：秒级；`env.block.time`（ns）→ 向下取整。

#### 2.1 `executor_registry`

* **职责**：注册 `executor/oracle/gateway/governance`，维护映射与元数据。
* **核心存储**：

  * `executors: Map<ExecutorId, {controller: Addr, profile_hash: {sha256, keccak}, active: bool}>`
  * `oracles: Set<Addr>`、`gateways: Map<Addr, ExecutorId>`
* **Execute**：

  * `register_executor{controller, profile_hash_*}`（only gov）
  * `register_oracle{addr}`（only gov），`register_gateway{addr, executor_id}`（only gov）
  * `set_active{executor_id, on}`（only gov）
* **Query**：`get_executor`, `is_oracle`, `gateway_of`。

#### 2.2 `ans_state_manager`

* **职责**：三态 + 回滞/驻留时间 + Guard（缩放系数）。
* **存储**：

  * `state[executor_id] -> {state, tone, updated_at}`
  * `guard_baseline[action_id] -> Guard`（SAFE 态基线）
  * 阈值与驻留：`cfg: {danger_enter, danger_exit, shutdown_enter, dwell_min_seconds}`
* **Execute**：

  * `update_tone{executor_id, tone (0..1e6), suggested}`（only oracle）
  * 内部根据阈值与驻留更新时间状态；发 `VagalToneUpdated` 事件。
* **Query**：`get_state{executor_id}`, `guard_for{action_id}`。

#### 2.3 `afferent_inbox`

* **职责**：AEP/状态根锚定，限制调用者为对应 gateway。
* **Execute**：

  * `post_aep{executor_id, state_root_sha256, state_root_keccak, metrics_hash_sha256, metrics_hash_keccak, attestation}`（only gateway for executor）
  * 记录滚动窗口的最新一条（MVP）。发 `AEPPosted`。
* **Query**：`latest_state_root{executor_id}`。

#### 2.4 `vagal_brake`

* **职责**：读取 `ans_state_manager.guard_for(action)`，对意图的**可制动字段**进行缩放与校验；**不存状态**。
* **Execute（内部供 issuer 调用或外部代理调用）**：

  * `apply{executor_id, action_id, params_cbor, brakeable_mask}` → 返回 `scaled_limits_hash`（以 `SubMsg::ReplyOn::Success` 或回执 attributes 传递）。
* **错误**：`ANS_BLOCKED`（不允许）、`ANS_LIMIT`（超过缩放上限）。

#### 2.5 `capability_issuer`（可直接内含 cw721/cw1155 子集，或使用依赖）

* **职责**：签发与撤销 Token；TTL；有效性查询；活跃 Token 枚举。
* **依赖**：`executor_registry`、`ans_state_manager`、`afferent_inbox`、`vagal_brake`。
* **Execute**：

  * `issue{intent}`：

    1. 校验 `tx.sender ∈ planner_whitelist`（或走 registry 的角色），
    2. 校验 TTL（`now ∈ [notBefore, notAfter]`），
    3. 校验 `preStateRoot == afferent_inbox.latest(executor_id)`，
    4. 调 `vagal_brake.apply` 获取 `scaled_limits_hash`，
    5. **mint** Token：保存 `executor_id, action_id, params_hash_{sha256,keccak}, envelope_hash_{*}, pre_state_root_{*}, not_[before|after], maxDuration, maxEnergy, scaled_limits_hash`，
    6. 发 `CapabilityIssued`。
  * `revoke{token_id, reason}`：更新状态并发 `CapabilityRevoked`。
* **Query**：`is_valid{token_id}`, `active_tokens_of{executor_id}`。

#### 2.6 `reflex_arc`

* **职责**：接收来自 `ans_state_manager` 或 `afferent_inbox` 的**显式调用**，决定是否批量撤销。
* **Execute**：

  * `on_state_change{executor_id, new_state}`（only ans） → 若 `DANGER/SHUTDOWN` 则批量 `revoke`；
  * `on_aep{executor_id, metrics}`（only inbox） → 触发阈值则批量 revoke；
  * `pulse{executor_id}`：keeper 可周期调用，处理遗漏情况（幂等 + 节流）。
* **注意**：为避免 O(n) 风险，**分页撤销**，每次最多处理 K 个活跃 Token；剩余由 keeper 继续调 `pulse`。

**DoD（M12 总验收）**

* 六个合约 `cargo wasm` 构建与单测（`cw-multi-test`）通过；
* e2e：部署 → 注册 → AEP → update_tone → issue（通过 VagalBrake）→ reflex revoke；
* 事件键名与 `/spec/events.yml` 一致（测试断言）。

---

### 3) 网关/Oracle 对接 L1（vagus-chain）

**任务 M13：双端客户端抽象 + 显式触发**

1. 在 `gateway/crates/vagus-chain/` 新增 `wasm` 实现（若已有 EVM 实现并存）：

   * `trait ChainClient { fn post_aep(...); fn issue_intent(...); fn revoke(...); fn update_tone(...); fn call_reflex_pulse(...); }`
   * `impl ChainClient for WasmClient`：基于 `vagus-chain` SDK（或 CosmWasm RPC）实现交易构造、查询与 attributes 解析；
   * AEP 上报时**携带最小窗口指标**，方便 ReflexArc 决策。

2. `vagus-gateway`：

   * 新增 `--chain=vagus-l1` 配置；
   * 在**每次 AEP 上报成功**后，尝试调用 `reflex_arc.pulse(executor_id)`（幂等、可失败重试）。

3. `tone-oracle`：

   * 新增 `--target=vagus-l1`；周期或事件驱动地 `ans_state_manager.update_tone`；
   * 配置**驻留/回滞阈值**，与链上一致。

**DoD**

* 本地 devnet：`anvil`（EVM）可并存（但此里程碑验证 vagus-chain 路径）→ AEP/更新 tone/issue/pulse 全链路跑通。

---

### 4) 限流与熔断（最小可用）

**任务 M14：RateLimiter + CircuitBreaker（可先集成到 Issuer，也可独立合约）**

* `rate_limiter`（可嵌入 issuer）：

  * 维度：`{executor_id, action_id, planner}`；
  * 令牌桶/滑窗计数；超限返回 `RATE_LIMITED`。
* `circuit_breaker`：

  * `trip{scope}`（gov/ans/reflex 可触发）→ 使 `issuer.issue` 直接拒绝或仅允许逃逸类动作；
  * `reset{scope}`（延时治理）。

**DoD**：超限与熔断路径单测 + e2e 通过。

---

### 5) 一致性与不可变式测试（黄金套件延伸）

**任务 M15：把 EVM 与 vagus-chain/WASM 放到同一规范测试框架**

* 在 `tests/golden/`：

  * 共用 `/spec/*.yml` 生成的类型；
  * 针对两条实现分别跑：I1–I5（原有）+ I6–I9（跨链一致性、fail‑closed、时间漂移保护）。
* 事件等价：同一用例在两侧产生的事件键/值对一致（考虑哈希双字段）。
* 反射撤销延迟：从 AEP/状态改变到 `CapabilityRevoked` 的块高差 ≤ 配置上限。

**DoD**：`make golden-test` 或 `cargo test -p golden` 一键通过。

---

### 6) 文档与一键脚本

**任务 M16：文档 + Devnet**

* `docs/L1-On-Vagus-Chain.md`：

  * 架构图（WASM 六合约与调用关系）、事件/错误码表、消息体示例；
  * 常见陷阱：时间单位、显式触发、哈希双存、授权白名单。
* `infra/devnet/vagus-chain.sh`：启动本地 `vagus-chain` 节点（或模拟器）、部署脚本、账户预置；
* `infra/devnet/up.sh`：一键启动 oracle/gateway 并连到 vagus-chain。

**DoD**：在全新环境按文档可跑通端到端 Demo。

---

## 三、提交给 Cursor 的“可执行提示词”（逐条投喂）

将以下命令式提示词逐条给 Cursor，让其在 repo 中完成任务：

1. **创建 L1/WASM 目录与 Spec 同步**

   * *指令*：

     > 新建 `l1/wasm` 工作区与 `packages/vagus-spec`，在 `spec/` 下补齐 `types.yml/events.yml/errors.yml/invariants.yml`，实现 `planner/vagus_planner/codegen.py` 的 Rust 代码生成（含常量、事件键、错误码、类型）。新增 CI 的 Spec Drift 检查。要求幂等。
   * *验收*：`cargo build -p vagus-spec` 通过；CI drift job 绿灯。

2. **实现 6 个 WASM 合约骨架与接口**

   * *指令*：

     > 在 `l1/wasm/contracts` 下实现 `executor_registry / ans_state_manager / afferent_inbox / vagal_brake / capability_issuer / reflex_arc` 的 `InstantiateMsg/ExecuteMsg/QueryMsg` 与状态存储。所有事件键名严格来自 `/spec/events.yml`。实现最小逻辑（无业务细节）与单元测试模板（`cw-multi-test`）。
   * *验收*：`cargo test -p ...` 通过；事件键名测试通过。

3. **补全业务逻辑（回滞/缩放/TTL/前置状态匹配/分页撤销）**

   * *指令*：

     > 分三个 PR 完成：
     > A）`ans_state_manager`：回滞、驻留、Guard 缩放；
     > B）`vagal_brake`：对 `brakeable` 字段进行缩放校验与 `scaled_limits_hash` 生成（CBOR→sha256/keccak）；
     > C）`capability_issuer`：TTL 检查、`preStateRoot` 一致性、mint、`active_tokens_of`、分页 revoke；`reflex_arc`：`on_state_change/on_aep/pulse` 实现与节流。
   * *验收*：六合约单测 + e2e 通过；越界/过期/状态不匹配用例触发正确错误码。

4. **网关/Oracle 对接 vagus-chain**

   * *指令*：

     > 在 `gateway/crates/vagus-chain` 添加 `WasmClient`，实现 `ChainClient` 所有方法。`vagus-gateway` 增加 `--chain=vagus-l1`，在成功 `post_aep` 后调用 `reflex_arc.pulse(executor_id)`（忽略幂等冲突）。`tone-oracle` 支持 `--target=vagus-l1` 并发起 `update_tone`。
   * *验收*：本地 devnet 上跑通 end-to-end。

5. **RateLimiter 与 CircuitBreaker（最小版本）**

   * *指令*：

     > 在 Issuer 中实现基于滑动窗口的限流；新增 `circuit_breaker` 开关（可内嵌或独立合约），在打开时仅允许逃逸类动作。补充单测。
   * *验收*：超限拒绝/熔断拒绝的路径有事件、有错误码，测试通过。

6. **黄金套件一致性测试**

   * *指令*：

     > 在 `tests/golden/` 写跨实现测试，比较 WASM 与 EVM 的行为一致性与事件键值一致性，验证 I1–I9。
   * *验收*：一键命令跑两栈测试全绿。

7. **文档与一键脚本**

   * *指令*：

     > 编写 `docs/L1-On-Vagus-Chain.md` 与 `infra/devnet/vagus-chain.sh`、`up.sh`。在 README 增加 L1 快速开始。
   * *验收*：按文档在干净环境可完成部署与 Demo。

---

## 四、`.cursorrules` 追加（务必添加）

```
# === Vagus L1 (WASM) Rules ===
- CosmWasm 默认目标：时间统一为秒；将 env.block.time(ns) 向下取整。
- Events: 必须使用 spec/events.yml 中的键；WASM 侧以 attributes 发出；EVM 侧以 topics/logs 发出。
- Hashing: 所有 params/metrics/envelope/preStateRoot 先做 CBOR 规范化，再同时计算 sha256 与 keccak；字段名以 *_sha256/*_keccak 结尾。
- Auth: 以 tx.sender + Registry 白名单为唯一链上授权依据；EIP-712 仅用于链下审计，不在 WASM 合约内验签。
- Reflex: 只允许显式触发（on_state_change/on_aep/pulse）；禁止依赖“链内事件监听”。
- Revocation: 必须分页撤销；每次处理上限 K（配置化），防止 gas/执行时间爆炸。
- Rate Limit + Circuit Breaker: Fail-closed；当外部依赖异常（oracle/gateway/relayer）时默认进入 DANGER 或拒绝新授权。
- Upgrade/Governance: 关键参数（阈值/回滞/缩放系数）变更必须有事件与延时；测试覆盖。
- Tests: cw-multi-test 覆盖正常/越界/异常/抖动边界；golden 测试比较 EVM 与 WASM 两侧事件一致性。
```

---

### 结语

按上述修正与任务推进，**Vagus 将在 vagus-chain（WASM L1）上形成与 EVM 等价的“迷走神经层”合约组**：

* `AfferentInbox`（传入证据锚定）
* `ANSStateManager`（张力/三态/回滞）
* `VagalBrake`（独立制动门，语义清晰）
* `CapabilityIssuer`（短时效能力令牌；前置状态校验；与 Brake 解耦）
* `ReflexArc`（显式触发的反射弧，分页撤销防爆）
* `ExecutorRegistry`（身份与角色白名单）
  配合网关与 Oracle 的显式触发与最小限流/熔断，**在不改动 L1 的前提下**实现企业级的安全与可审计性，并严格坚持我们的**黄金编码原则**。
