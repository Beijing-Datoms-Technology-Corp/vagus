# Vagus 多链架构指南

## 概述

Vagus 是一个多链去中心化自主神经系统（ANS），在 EVM 兼容链和 CosmWasm 链上提供统一的 safety layer。核心目标是在不牺牲安全性与可审计性的前提下，实现跨链兼容性。

## 架构概览

```
┌─────────────────┐    ┌─────────────────┐
│   EVM Chain     │    │  CosmWasm Chain │
│   (Ethereum,    │    │   (Cosmos,      │
│    Polygon,     │    │    Osmosis)     │
│    Arbitrum)    │    │                 │
│                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │ Afferent    │ │    │ │ Afferent    │ │
│ │ Inbox       │◄┼────┼►│ Inbox       │ │
│ └─────────────┘ │    │ └─────────────┘ │
│                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │ ANS State   │◄┼────┼►│ ANS State   │ │
│ │ Manager     │ │    │ │ Manager     │ │
│ └─────────────┘ │    │ └─────────────┘ │
│                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │ Capability  │◄┼────┼►│ Capability  │ │
│ │ Issuer      │ │    │ │ Issuer      │ │
│ └─────────────┘ │    │ └─────────────┘ │
│                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │ Vagal Brake │◄┼────┼►│ Vagal Brake │ │
│ └─────────────┘ │    │ └─────────────┘ │
│                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │ Reflex Arc  │◄┼────┼►│ Reflex Arc  │ │
│ └─────────────┘ │    │ └─────────────┘ │
└─────────────────┘    └─────────────────┘
        ▲                        ▲
        │                        │
        └─────────┬──────────────┘
                  │
           ┌─────────────┐
           │   Relayer   │
           │             │
           │ • Event     │
           │   Sync      │
           │ • State     │
           │   Mirror    │
           │ • Invariant │
           │   Check     │
           └─────────────┘
                  ▲
                  │
           ┌─────────────┐
           │  Clients    │
           │             │
           │ • Gateway   │
           │ • Oracle    │
           │ • Planners  │
           └─────────────┘
```

## 核心组件

### 1. 链无关规格层 (Portable Spec)

位置: `spec/`

所有跨链类型和事件都在 YAML 规格文件中定义，然后通过代码生成器生成对应的 Solidity 和 Rust 代码。

**关键文件:**
- `spec/types.yml` - 结构体、枚举、常量定义
- `spec/events.yml` - 标准化事件格式
- `spec/invariants.yml` - 系统不变式
- `spec/errors.yml` - 错误码定义

**代码生成:**
```bash
cd planner
python -m vagus_planner.codegen
```

### 2. 统一链客户端 (Chain Client)

位置: `gateway/crates/vagus-chain/`

提供统一的异步 trait 接口，支持 EVM 和 CosmWasm 链：

```rust
#[async_trait]
pub trait ChainClient: Send + Sync {
    async fn submit_aep(&self, aep: &AfferentEvidencePacket) -> Result<String>;
    async fn issue_with_brake(&self, intent: &Intent, scaled_limits_hash: &[u8; 32], expires_at: u64) -> Result<String>;
    async fn revoke_capability(&self, token_id: &str, reason: u8) -> Result<()>;
    async fn subscribe_events<F>(&self, callback: F) -> Result<()> where F: Fn(Event) + Send + Sync + 'static;
    async fn get_guard(&self, action_id: &[u8; 32]) -> Result<Guard>;
    async fn get_ans_state(&self) -> Result<ANSState>;
    async fn update_tone(&self, vti: u64, suggested_state: ANSState) -> Result<()>;
}
```

### 3. 跨链中继器 (Relayer)

位置: `relayer/`

监听源链事件，转发到目标链，保障状态一致性：

```bash
# 启动双向中继
cargo run -- --source-chain evm --source-rpc http://localhost:8545 \
             --target-chain cosmos --target-rpc http://localhost:26657 \
             --private-key $PRIVATE_KEY
```

### 4. 多链客户端应用

#### 网关 (Gateway)
```bash
# EVM 模式
vagus-gateway start --chain evm --rpc-url http://localhost:8545 \
                   --afferent-inbox 0x...

# Cosmos 模式
vagus-gateway start --chain cosmos --rpc-url http://localhost:26657 \
                   --afferent-inbox vagus1...
```

#### Oracle
```bash
# 双链模式
tone-oracle serve --port 3000 \
                 --evm-rpc http://localhost:8545 \
                 --cosmos-rpc http://localhost:26657 \
                 --private-key $PRIVATE_KEY
```

## 开发环境设置

### 1. 本地双链环境

使用 Docker Compose 启动完整开发环境：

```bash
# 启动所有服务
./infra/devnet/up.sh

# 服务概览:
# - anvil (EVM): http://localhost:8545
# - wasmd (Cosmos): http://localhost:26657
# - tone-oracle: http://localhost:3000
# - gateway-evm: 运行中
# - gateway-cosmos: 运行中
# - relayer-evm-to-cosmos: 运行中
# - relayer-cosmos-to-evm: 运行中
```

### 2. 合约部署

#### EVM 合约
```bash
cd contracts
forge script script/DeployCore.s.sol --rpc-url http://localhost:8545 --broadcast
```

#### CosmWasm 合约
```bash
cd wasm-contracts
# 编译合约
cargo build --target wasm32-unknown-unknown --release

# 部署到本地 wasmd
# (需要编写部署脚本)
```

## 测试

### 黄金规范测试

运行跨链不变式测试：

```bash
cd tests/golden
cargo run -- run-all --evm-rpc http://localhost:8545 --cosmos-rpc http://localhost:26657
```

### 不变式说明

Vagus 系统维护以下关键不变式：

1. **I1: SHUTDOWN ⇒ 无有效 Token**
   - 当系统处于 SHUTDOWN 状态时，所有 capability token 必须失效

2. **I2: DANGER ⇒ Token 限制按比例缩放**
   - DANGER 状态下，token 参数限制应按 VTI 值进行缩放

3. **I3: 反射撤销延迟 ≤ 配置上限**
   - 危险条件下的自动撤销必须在配置的时间内完成

4. **I4: Intent 包络 ⊆ No-Go 补集**
   - 所有执行意图必须在安全边界内

5. **I5: CBF 投影安全**
   - 控制障碍函数确保系统保持在安全状态

6. **I6: 跨链一致性**
   - 相同事件在两条链上的状态必须在 ΔT 时间窗内一致

## 事件对照表

| 事件名称 | EVM (Topic 0) | CosmWasm (Attribute) | 说明 |
|---------|---------------|---------------------|-----|
| CapabilityIssued | `keccak256("CapabilityIssued")` | `event: "capability_issued"` | Token 发行 |
| CapabilityRevoked | `keccak256("CapabilityRevoked")` | `event: "capability_revoked"` | Token 撤销 |
| AEPPosted | `keccak256("AEPPosted")` | `event: "aep_posted"` | 证据提交 |
| VagalToneUpdated | `keccak256("VagalToneUpdated")` | `event: "vagal_tone_updated"` | 音调更新 |
| ReflexTriggered | `keccak256("ReflexTriggered")` | `event: "reflex_triggered"` | 反射触发 |

## 常见陷阱

### 1. 时间单位差异
- **EVM**: `block.timestamp` (秒)
- **CosmWasm**: `env.block.time` (纳秒) → 需要 `floor(time / 1_000_000_000)`

### 2. 哈希算法
- **双哈希策略**: 同时计算 `keccak256` (EVM 兼容) 和 `sha256` (CosmWasm 偏好)
- 事件中包含两个哈希值字段

### 3. 地址格式
- **EVM**: 20 字节十六进制 (0x...)
- **Cosmos**: bech32 字符串 (cosmos1...)

### 4. 权限模型
- **EVM**: 合约内部检查 + `msg.sender`
- **Cosmos**: 合约外部检查 + `info.sender`

### 5. Gas/存储成本
- **EVM**: 存储相对便宜，日志廉价
- **Cosmos**: 存储按字节计费，事件属性廉价

## 演示场景

### 场景 1: 跨链 Capability 生命周期

1. **EVM 上发行 Token**
   ```bash
   # Planner 请求 capability
   curl -X POST http://localhost:3000/vti -d '{"executor_id": 1, "human_distance_mm": 1500}'
   ```

2. **Relayer 同步到 Cosmos**
   ```
   EVM: CapabilityIssued(tokenId=1, executorId=1)
   ↓
   Cosmos: capability_issued(token_id=1, executor_id=1)
   ```

3. **Cosmos 上报告危险情况**
   ```bash
   # Gateway 在 Cosmos 上提交 AEP
   # Reflex Arc 检测到危险 → 撤销 token
   ```

4. **Relayer 同步撤销回 EVM**
   ```
   Cosmos: reflex_triggered(executor_id=1, revoked_count=1)
   ↓
   EVM: ReflexTriggered(executorId=1, revokedCount=1)
   ```

## 故障排除

### 常见问题

1. **Relayer 连接失败**
   ```bash
   # 检查链端点
   curl http://localhost:8545
   curl http://localhost:26657/status
   ```

2. **合约地址错误**
   ```bash
   # 更新配置
   export EVM_ANS_MANAGER=0x...
   export COSMOS_ANS_MANAGER=vagus1...
   ```

3. **权限不足**
   ```bash
   # 检查私钥
   export PRIVATE_KEY=0x...
   ```

### 调试命令

```bash
# 查看服务日志
docker-compose logs -f [service-name]

# 测试链连接
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","id":1}'

# 测试 Oracle
curl http://localhost:3000/health

# 运行黄金测试
cargo run --manifest-path tests/golden/Cargo.toml -- run-all
```

## 扩展到新链

要添加对新区块链的支持：

1. 在 `ChainType` 枚举中添加新类型
2. 实现对应的 `ChainClient`
3. 更新 `ChainClientFactory`
4. 添加相应的依赖和配置
5. 编写针对新链的黄金测试

## 结论

Vagus 的多链架构通过统一的规格层、抽象的客户端接口和可靠的跨链同步，实现了在保持安全性的同时支持多种区块链平台。这种设计确保了系统的可扩展性和一致性，同时为不同的区块链生态系统提供了无缝集成体验。
