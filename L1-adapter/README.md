# L1 Adapter Layer

## 概述

L1 适配层是 Vagus 协议的核心组件，负责将 Vagus 的核心业务逻辑适配到不同的 Layer 1 区块链上。这个目录包含了针对不同 L1 的适配实现、文档和工具。

## 目录结构

```
L1-adapter/
├── README.md                           # 本文件
├── vagus-chain/                        # vagus-chain L1 适配
│   ├── README.md                       # vagus-chain 适配说明
│   ├── NATIVE_INTERFACE_SPECIFICATION.md
│   ├── ANS_STATE_MACHINE_SPECIFICATION.md
│   ├── INTEGRATION_TEST_SUITE.md
│   ├── NATIVE_CLIENT_ADAPTERS.md
│   └── MIGRATION_GUIDE.md
├── ethereum/                          # Ethereum 适配 (未来)
├── cosmos/                            # Cosmos 适配 (未来)
└── polkadot/                          # Polkadot 适配 (未来)
```

## 设计原则

### 1. 统一接口
所有 L1 适配器都实现相同的核心接口，确保 Vagus 协议在不同链上的一致性。

### 2. 原生优化
针对每个 L1 的特性进行优化，充分利用各链的原生能力。

### 3. 模块化设计
每个适配器都是独立的模块，可以单独开发、测试和部署。

### 4. 向后兼容
保持与现有 Vagus 组件的兼容性，支持渐进式迁移。

## 核心组件

### Vagus 核心合约
- **ANSStateManager**: 自主神经系统状态管理
- **CapabilityIssuer**: 能力令牌发行和管理
- **VagalBrake**: 动态参数缩放机制
- **AfferentInbox**: 传入证据包处理
- **ReflexArc**: 自动撤销机制

### 适配器接口
每个 L1 适配器都必须实现以下核心接口：

```rust
pub trait L1Adapter {
    // 状态管理
    async fn update_ans_state(&self, executor_id: u256, tone: u32) -> Result<()>;
    async fn get_ans_state(&self, executor_id: u256) -> Result<ANSState>;
    
    // 能力管理
    async fn issue_capability(&self, intent: &Intent) -> Result<u256>;
    async fn revoke_capability(&self, token_id: u256, reason: u8) -> Result<()>;
    async fn is_capability_valid(&self, token_id: u256) -> Result<bool>;
    
    // 证据处理
    async fn submit_evidence(&self, aep: &AfferentEvidencePacket) -> Result<()>;
    async fn get_latest_state_root(&self, executor_id: u256) -> Result<[u8; 32]>;
    
    // 事件监听
    async fn subscribe_events<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(Event) + Send + Sync + 'static;
}
```

## 当前支持的 L1

### vagus-chain
Vagus 生态的专用 L1 区块链，提供原生协议支持。

**特点**:
- 原生合约实现，极致性能
- 零/极低 Gas 成本
- 专为 Vagus 协议优化
- 完整的迟滞性状态机支持

**文档**: [vagus-chain/README.md](./vagus-chain/README.md)

## 未来计划

### Ethereum 适配
- 基于 Solidity 智能合约
- 支持 EVM 兼容链
- 标准 Gas 费用模型

### Cosmos 适配
- 基于 CosmWasm 智能合约
- 支持 IBC 跨链通信
- 自定义费用模型

### Polkadot 适配
- 基于 Substrate 运行时
- 支持平行链架构
- 共享安全模型

## 开发指南

### 添加新的 L1 适配器

1. **创建目录结构**
   ```bash
   mkdir L1-adapter/new-l1
   cd L1-adapter/new-l1
   ```

2. **实现核心接口**
   - 创建 `adapter.rs` 实现 `L1Adapter` trait
   - 创建 `client.rs` 处理 L1 特定逻辑
   - 创建 `types.rs` 定义 L1 特定类型

3. **编写文档**
   - 创建 `README.md` 说明适配器功能
   - 创建接口规范文档
   - 创建集成测试文档

4. **添加测试**
   - 单元测试
   - 集成测试
   - 端到端测试

### 测试新适配器

```bash
# 运行所有适配器测试
cargo test --workspace

# 运行特定适配器测试
cargo test -p vagus-l1-adapter-new-l1

# 运行集成测试
cd tests/integration
cargo test -- --test new-l1
```

## 贡献指南

### 代码规范
- 使用 Rust 标准格式化工具
- 遵循 clippy 建议
- 编写完整的文档注释
- 保持 90% 以上的测试覆盖率

### 文档规范
- 使用 Markdown 格式
- 包含完整的 API 文档
- 提供使用示例
- 保持文档与代码同步

### 提交流程
1. Fork 仓库
2. 创建功能分支
3. 实现功能并添加测试
4. 更新文档
5. 提交 Pull Request

## 许可证

本项目采用 Apache 2.0 许可证。详见 [LICENSE](../../LICENSE) 文件。

## 联系方式

- 项目仓库: https://github.com/Beijing-Datoms-Technology-Corp/vagus
- 问题报告: https://github.com/Beijing-Datoms-Technology-Corp/vagus/issues
- 讨论区: https://github.com/Beijing-Datoms-Technology-Corp/vagus/discussions
