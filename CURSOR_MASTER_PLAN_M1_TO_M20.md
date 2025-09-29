# CURSOR_MASTER_PLAN_M1_TO_M20.md

> **角色设定（给编程助理）**
> 你是 Vagus 的**总集成与生产加固负责人**。在已完成的 M1–M10 基础上，继续推进 M11–M20。
> 所有工作须同时满足 **EVM** 与 **WASM L1（vagus‑chain / CosmWasm）** 两栈的一致语义与不变式，且**严格遵守黄金编码原则**。

---

## 0) 黄金编码原则（务必严格执行）

1. **Correctness first**：以不可变式与规范为最高优先级；模棱两可一律拒绝执行。
2. **Fail‑closed**：任何校验失败/预言机掉线/时间漂移/中继异常 → 阻断或降级至 DANGER/SHUTDOWN。
3. **Single source of truth**：`/spec/*.yml` 是**唯一事实来源**（类型、事件、错误码、不可变式）。所有代码从 spec 生成骨架并做 drift 检查。
4. **Determinism**：参数**CBOR 规范化**后**双哈希**（`sha256`+`keccak`）；链上状态转移可复现。
5. **Explicit units**：所有物理量显式单位；跨栈（EVM/WASM）边界做单位一致性与边界测试。
6. **Least privilege**：白名单/角色控制；禁止无界循环与可重入；升级/参数变更需多签+延时。
7. **No silent fallback**：不支持或异常必须给出明确错误码+事件；记录足迹便于审计回放。
8. **Small, testable slices**：小步提交；每条路径含单测、e2e、黄金一致性测试。
9. **Observability**：关键动作发机读事件（键名来自 `/spec/events.yml`）；导出运行指标。
10. **Docs‑as‑code**：接口/消息体/安全边界写在 `docs/` 并跟随代码演进；文档更新是验收一部分。

---

## 1) **跨栈等价性要求（ER）** —— 把 L1 设计修正点固化为强约束

> 这些 ER 要求已经被整合进 M11–M20 的任务与验收标准，并写入 `/spec/invariants.yml` 与黄金测试套件。

* **ER1（VagalBrake 独立）**：`CapabilityIssuer` 不能在缺少 `VagalBrake` 成功校验的情况下签发能力（仅“逃逸/急停”类例外，且需治理显式白名单）。
* **ER2（Reflex 显式触发）**：`ReflexArc` 只接受显式调用：`on_state_change`、`on_aep`、`pulse`；**禁止**依赖“链内事件监听自动执行”。
* **ER3（授权模型）**：WASM L1 上**仅**以 `tx.sender + Registry 白名单` 为授权依据；EIP‑712 仅用于链下审计。
* **ER4（参数规范化与双哈希）**：所有 `params/envelope/preStateRoot/metrics` 在两栈均使用**相同的 CBOR 规范化**，并同时存储 `*_sha256` 与 `*_keccak`。
* **ER5（时间统一）**：统一**秒级**时间；WASM 侧将 `env.block.time`（纳秒）**向下取整**再参与 TTL 判定。
* **ER6（前置状态一致性）**：`issue` 时必须满足 `preStateRoot == AfferentInbox.latest(executorId)`；不匹配即拒绝。
* **ER7（限流与熔断）**：`RateLimiter` 与 `CircuitBreaker` 在两栈行为一致，外部依赖异常时**默认降级**。
* **ER8（治理与可升级）**：关键参数/升级同样走**多签+延时**，变更发事件，迁移保留状态兼容。

> 将 ER1–ER8 同步登记为不可变式 **I19–I26**（见 §2），并纳入黄金测试。

---

## 2) 规格与不可变式（Spec）更新

在现有 `I1–I18` 基础上，补充：

* **I19**：无 `VagalBrake` 通过则 `CapabilityIssuer.issue` 必须失败（逃逸例外需事件标记）。
* **I20**：`ReflexArc` 只能由 `ANSStateManager` / `AfferentInbox` / keeper 显式调用，且具去重节流。
* **I21**：同一 CBOR 输入在两栈产生**一致**的 `sha256/keccak`（黄金测试比对子串）。
* **I22**：WASM 授权仅基于 `tx.sender` 的白名单；任何链上验签路径被禁用。
* **I23**：TTL 判定使用秒；WASM 必须 floor 时间；跨栈同一意图 TTL 结果一致。
* **I24**：签发事件必须包含 `scaled_limits_hash` 与双哈希 `pre_state_root_*` 与 `params_hash_*`。
* **I25**：Reflex 批量撤销分页上限 K；永不超 gas/执行时限；长队列可被 `pulse` 完成。
* **I26**：两栈在 `RateLimiter/CircuitBreaker` 的超限/熔断语义一致（错误码与事件一致）。

> **任务**：把 I19–I26 写入 `/spec/invariants.yml`，并更新 `spec/errors.yml`（`ANS_BLOCKED`、`ANS_LIMIT`、`TTL`、`STATE_MISMATCH`、`NOT_AUTHORIZED`、`RATE_LIMITED`、`CBOR_HASH_MISMATCH`）。
> **CI**：Spec Drift + Invariants 测试必过。

---

## 3) 里程碑一览（**合并版 M1–M20**）

> **状态**：M1–M10 已完成（EVM + WASM 多链就绪、黄金套件可跑）。以下为**合并后的 M11–M20**，包含 L1 等价性约束（ER1–ER8）与生产级加固。

### **M11 — 版本冻结与发布工程**

* 产物版本化（SemVer、带生成指纹）；WASM code id+checksums，EVM ABIs/bytecode/storage layout；SBOM+LICENSE 报告；签名。
* **验收**：`v1.0.0-rc.1` tag；release 工件可复现；Docs《版本与发布流程》。

### **M12 — 治理与升级安全（EVM+WASM）**

* **EVM**：Safe+Timelock+UUPS，升级/回滚测试；
* **WASM**：cw3‑dao+cw4‑group，`migrate()` 带状态版本；
* **验收**：I10/I11 通过；治理提案变更 `danger_enter` 示例可生效（延时后）。

### **M13 — 限流与熔断（最终版，含 ER7）**

* 令牌桶/滑窗（维度：tenantId/executorId/actionId/planner）；`CircuitBreaker` 仅允许逃逸类动作；失联自动降级；
* **验收**：I16/I26 通过；黄金测试覆盖两栈一致。

### **M14 — 可观测性与索引**

* EVM subgraph + WASM 轻量索引器；统一 REST/GraphQL；Prom 指标（撤销延迟直方图、AEP 速率等）。
* **验收**：看板展示 AEP→Revocation 延迟；审计导出含 `scaled_limits_hash` 与双哈希。

### **M15 — Relayer 加固（持久化/幂等/可恢复）**

* Durable queue（RocksDB/SQLite）、去重键、at‑least‑once 发送、断点续传、混沌测试；
* **验收**：I13/I14 通过；长跑 1h 无副作用；`--from-height` 恢复成功。

### **M16 — 性能与费用基准**

* EVM gas‑snapshot；WASM 运行时基准（criterion）；场景：授权批量、反射分页、AEP 峰值；
* **验收**：生成 `docs/Benchmarks.md`，阈值回归告警。

### **M17 — 形式化与模糊验证**

* EVM：Echidna/Foundry invariants（含 I19–I26）；
* WASM：`proptest` + `cw-multi-test`；
* **验收**：CI 新增 invariants job，失败即阻断。

### **M18 — 隐私增强与 ZK（可选）**

* 区间证明（如 `min_distance≥d_min`）；EVM Verifier + WASM 验证或链下签名根；
* **验收**：演示用例在不泄露原始数据的情况下完成授权。

### **M19 — 多租户 AAA（认证/授权/审计）**

* `tenantId` 贯穿 Intent/Token/事件；限流与审计按租户隔离；
* **验收**：I15 通过；跨租户越权被拒并记录。

### **M20 — 运行手册与 SRE**

* Runbook、Security‑Checklist、应急演练与报警规则；
* **验收**：按 Runbook 完成一次演练（脚本自动化），全链路通过。

---

## 4) 需要 Cursor 逐条执行的**命令式任务**

> **注意**：以下任务默认你已处于 `main` 分支，且 M1–M10 代码与 CI 均为绿。每步完成**先本地跑全套测试**再提交。

### T‑1 规范补丁（并生成骨架）

* **操作**：

  1. 在 `/spec/invariants.yml` 增加 I19–I26；
  2. 在 `/spec/errors.yml` 增加 `CBOR_HASH_MISMATCH` 等；
  3. 运行 `planner/vagus_planner/codegen.py`，更新：

     * `contracts/src/core/Types.sol`、`Events.sol`；
     * `l1/wasm/packages/vagus-spec/src/lib.rs`；
     * `gateway/crates/vagus-spec/`；
  4. 确保 **Spec Drift** job 绿色。
* **验收**：生成产物带版本指纹与常量，CI drift 通过。

### T‑2 ER1/ER6 落地（Issuer+Brake+Inbox 一致）

* **操作**：

  * 在 EVM 与 WASM 的 `CapabilityIssuer` 中：

    * 缺少 `VagalBrake.apply` 返回的 `scaled_limits_hash` → 直接 `revert(ANS_BLOCKED)`；
    * 校验 `preStateRoot == AfferentInbox.latest(executorId)` 不通过 → `revert(STATE_MISMATCH)`；
  * 单测：没有 Brake、或前置状态不一致 → 必失败；事件不落地。
* **验收**：I19、I24 通过；黄金测试比对两栈行为一致。

### T‑3 ER2 落地（Reflex 显式触发 + 分页撤销）

* **操作**：

  * 在两栈 `ReflexArc` 中实现 `on_state_change/on_aep/pulse`；
  * `CapabilityIssuer.active_tokens_of` 支持分页；`ReflexArc` 单次处理上限 K（配置化）。
* **验收**：I20、I25 通过；AEP 风暴长队列可被 `pulse` 清空。

### T‑4 ER3/ER5 落地（授权与时间）

* **操作**：

  * WASM：所有 `ExecuteMsg` 权限基于 `info.sender` + Registry 白名单；移除链上验签逻辑；
  * TTL 判定使用 `floor(env.block.time.seconds())`；EVM 对齐。
* **验收**：I22、I23 通过；跨栈同一 TTL 结果一致。

### T‑5 ER4 落地（CBOR 规范化与双哈希）

* **操作**：

  * 在 planner 与网关共用的 **CBOR 规范化**模块固化；
  * 两栈合约检查 `params_hash_*` 与 `pre_state_root_*` 双哈希存在并一致；
  * 新增错误码 `CBOR_HASH_MISMATCH`。
* **验收**：I21、I24 通过；黄金测试对比字节级一致。

### T‑6 ER7/ER8 落地（限流/熔断 + 治理/升级）

* **操作**：

  * 两栈统一 `RateLimiter`（滑窗/令牌桶）与 `CircuitBreaker` 行为；
  * EVM：Safe+Timelock+UUPS；WASM：cw3‑dao 迁移与参数提案；
  * 文档与测试覆盖。
* **验收**：I10/I11/I16/I26 通过。

### T‑7 发布与 SRE（M11/M20）

* **操作**：

  * 发布流水线产物与 SBOM；
  * 观测/索引/报警看板；
  * Runbook 与应急演练脚本。
* **验收**：版本发布可复现；一键演练通过。

---

## 5) `.cursorrules` 合并追加

```
# === Multichain & L1 Equivalence Rules ===
- Enforce ER1–ER8 at code + tests level; add invariants I19–I26 to /spec and golden tests.
- VagalBrake must be an independent contract/service: Issuer cannot mint without a valid scaled_limits_hash.
- ReflexArc: allow only explicit calls (on_state_change, on_aep, pulse) with dedupe + rate limit; never rely on event listeners.
- WASM Auth: use tx.sender + Registry roles only; disable on-chain signature verification paths.
- Time: operate in seconds; on WASM floor env.block.time(ns) to seconds before comparisons.
- Hashing: normalize with CBOR (stable map ordering, IEEE754 for numbers), then store both sha256 and keccak; names suffixed with *_sha256/*_keccak.
- Pre-state gate: preStateRoot must equal AfferentInbox.latest(executorId) at issue time; otherwise revert.
- Pagination: any O(n) op must be chunked with upper bound K; test worst cases.
- Governance/Upgrade: multisig + timelock (EVM) or cw3-dao (WASM); all parameter changes emit canonical events and pass delay.
- Observability: events/attributes keys must exactly match /spec/events.yml; add Prom metrics for revocation latency and rate-limit hits.
```

---

## 6) 文档锚点（Cursor 需要创建/补全）

* `docs/L1-On-Vagus-Chain.md`（已存在则合并等价性要求与 ER1–ER8）
* `docs/Governance-and-Upgrades.md`
* `docs/Observability.md`
* `docs/Benchmarks.md`
* `docs/Formal-Verification.md`
* `docs/Runbook.md`
* `docs/Security-Checklist.md`
* `docs/Privacy-and-ZK.md`（若启用 M18）

---

### 结语

这份合并后的主计划**把“vagus‑chain（WASM L1）上与 EVM 等价的迷走神经层”约束（ER1–ER8）**牢固嵌入 M11–M20 的每一个任务、验收与测试中。
交给 Cursor 按 **T‑1 → T‑7** 执行即可，所有更改需在**两栈同步通过黄金测试**，并以 `/spec` 为唯一事实来源持续校验一致性。
