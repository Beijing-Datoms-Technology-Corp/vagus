# Vagus-Chain L1 适配器

## 概述

本目录包含针对 vagus-chain L1 的适配实现和文档。vagus-chain 是 Vagus 生态的专用 Layer 1 区块链，提供原生协议支持，实现极致性能和零/极低 Gas 成本。

## 重要说明

**vagus-chain 实现在单独的项目中**，本目录仅包含：
- 接口规范和文档
- 客户端适配代码
- 测试用例和指南
- 迁移和部署文档

## 目录结构

```
vagus-chain/
├── README.md                           # 本文件
├── NATIVE_INTERFACE_SPECIFICATION.md  # 原生接口规范
├── ANS_STATE_MACHINE_SPECIFICATION.md # ANS 状态机规范
├── INTEGRATION_TEST_SUITE.md          # 集成测试套件
├── NATIVE_CLIENT_ADAPTERS.md          # 原生客户端适配器
└── MIGRATION_GUIDE.md                 # 迁移指南
```

## 核心特性

### 原生协议支持
- 核心业务逻辑作为 L1 原生模块实现
- 不受 Wasm 虚拟机性能限制
- 以原生 Rust 代码速度运行

### 极致性能
- 函数调用 < 100ms
- 状态查询 < 10ms
- 支持 1000+ 并发用户

### 零/极低 Gas 成本
- 协议级操作可设定极低费用
- 核心安全功能零费用
- 优化的存储和计算模型

### 更高安全性
- 攻击面大幅缩小
- 核心逻辑受 L1 保护
- 原生权限和访问控制

## 核心合约

### 预定义地址
```rust
pub const ANS_STATE_MANAGER: Address = Address([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]);
pub const CAPABILITY_ISSUER: Address = Address([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02]);
pub const VAGAL_BRAKE: Address = Address([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03]);
pub const AFFERENT_INBOX: Address = Address([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04]);
pub const REFLEX_ARC: Address = Address([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05]);
```

### 1. ANSStateManager
**功能**: 自主神经系统状态管理
- 管理 SAFE/DANGER/SHUTDOWN 状态
- 实现迟滞性机制防止状态震荡
- 提供动态缩放因子

**核心函数**:
- `updateTone(uint256 executorId, uint32 tone)`: 更新执行器 tone 值
- `getExecutorState(uint256 executorId)`: 获取执行器状态
- `guardFor(uint256 executorId, bytes32 actionId)`: 获取保护信息

### 2. CapabilityIssuer
**功能**: 能力令牌发行和管理
- 发行时间限制的能力令牌
- 支持令牌撤销和验证
- 集成 VagalBrake 和 AfferentInbox 验证

**核心函数**:
- `issueCapability(Intent intent, bytes32 scaledLimitsHash)`: 发行能力令牌
- `revoke(uint256 tokenId, uint8 reason)`: 撤销令牌
- `isValid(uint256 tokenId)`: 检查令牌有效性

### 3. VagalBrake
**功能**: 动态参数缩放机制
- 基于 ANS 状态动态调整参数
- 实现安全限制和验证
- 提供预览功能

**核心函数**:
- `previewBrake(Intent intent)`: 预览制动效果
- `issueWithBrake(Intent intent)`: 带制动发行能力

### 4. AfferentInbox
**功能**: 传入证据包处理
- 接收和存储传感器证据
- 管理状态根和指标哈希
- 支持授权 attestor

**核心函数**:
- `postAEP(uint256 executorId, bytes32 stateRoot, bytes32 metricsHash, bytes signature)`: 提交证据
- `latestStateRoot(uint256 executorId)`: 获取最新状态根

### 5. ReflexArc
**功能**: 自动撤销机制
- 监听 ANS 状态变化
- 自动撤销危险情况下的能力
- 支持分页处理大量令牌

**核心函数**:
- `on_state_change(uint256 executorId, uint8 newState)`: 状态变化回调
- `on_aep(uint256 executorId)`: 证据包回调

## 文档说明

### 1. NATIVE_INTERFACE_SPECIFICATION.md
**功能**: 原生接口规范文档
- 详细的 ABI 规范
- 状态模型定义
- 存储模型设计
- 错误处理规范

**使用说明**: L1 团队实现原生合约的精确规范

### 2. ANS_STATE_MACHINE_SPECIFICATION.md
**功能**: ANS 状态机详细规范
- 状态转换逻辑
- 迟滞性机制实现
- 完整的伪代码和 Rust 实现示例
- 安全机制和性能优化

**使用说明**: 实现 ANSStateManager 的核心参考文档

### 3. INTEGRATION_TEST_SUITE.md
**功能**: 全面的集成测试套件
- 100+ 个详细测试用例
- 单元测试、集成测试、端到端测试
- 性能测试和回归测试
- 完整的测试执行指南

**使用说明**: 验证原生实现与 Solidity 版本行为完全一致

### 4. NATIVE_CLIENT_ADAPTERS.md
**功能**: 客户端适配器实现指南
- 原生合约客户端实现
- 配置管理更新
- 部署和测试指南
- 迁移检查清单

**使用说明**: 更新 Tone Oracle 和 Vagus Gateway 客户端代码

### 5. MIGRATION_GUIDE.md
**功能**: 完整的迁移和协作指南
- 6 个阶段的详细迁移计划
- L1 团队和 Vagus 团队协作机制
- 风险缓解策略
- 成功标准和监控方案

**使用说明**: 指导从 Solidity 合约到原生协议的完整迁移过程

## 快速开始

### 1. 环境准备
```bash
# 克隆 vagus-chain 项目 (单独仓库)
git clone https://github.com/vagus-chain/vagus-chain.git
cd vagus-chain

# 构建 vagus-chain L1
cargo build --release
```

### 2. 启动 vagus-chain L1
```bash
# 启动开发网络
./target/release/vagus-chain --dev --tmp

# 或使用 Docker
docker run -p 26657:26657 vagus-chain:latest
```

### 3. 配置客户端
```bash
# 设置环境变量
export VAGUS_USE_NATIVE_CONTRACTS=true
export VAGUS_CHAIN_RPC_URL=http://localhost:26657

# 启动 Tone Oracle
cargo run --bin tone-oracle start

# 启动 Vagus Gateway
cargo run --bin vagus-gateway start --executor-id 1 --chain vagus-chain --rpc-url http://localhost:26657
```

### 4. 运行测试
```bash
# 运行集成测试
cargo test --workspace --features native-contracts

# 运行回归测试
cargo run --bin regression-test -- --compare-solidity
```

## 开发工作流

### L1 团队工作流
1. 阅读接口规范文档
2. 实现原生合约模块
3. 编写单元测试
4. 执行集成测试
5. 性能优化和审计

### Vagus 团队工作流
1. 更新客户端适配器
2. 编写回归测试
3. 执行端到端测试
4. 准备生产部署

## 性能指标

### 目标性能
- 函数调用延迟: < 100ms
- 状态查询延迟: < 10ms
- 并发用户支持: 1000+
- 内存使用: < 1GB

### 监控指标
- 函数调用延迟分布
- 状态转换频率
- 令牌发行/撤销速率
- 错误率和成功率

## 安全考虑

### 权限控制
- 严格的函数访问控制
- 基于角色的权限管理
- 多签名支持

### 输入验证
- 所有输入参数验证
- 范围检查和类型检查
- 防止整数溢出

### 状态一致性
- 原子性状态更新
- 事务回滚机制
- 状态同步验证

## 故障排除

### 常见问题
1. **连接失败**: 检查 RPC URL 和网络连接
2. **权限错误**: 验证私钥和地址配置
3. **状态不一致**: 检查状态同步和事件监听
4. **性能问题**: 查看监控指标和日志

### 调试工具
```bash
# 查看日志
tail -f logs/vagus-chain.log

# 监控指标
curl http://localhost:9090/metrics

# 检查状态
curl -X POST http://localhost:26657 -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"state_getStorage","params":["0x..."],"id":1}'
```

## 贡献指南

### 代码贡献
1. Fork 仓库
2. 创建功能分支
3. 实现功能并添加测试
4. 提交 Pull Request

### 文档贡献
1. 更新相关文档
2. 添加使用示例
3. 保持文档与代码同步

### 问题报告
1. 使用 GitHub Issues
2. 提供详细的复现步骤
3. 包含日志和配置信息

## 许可证

本项目采用 Apache 2.0 许可证。详见 [LICENSE](../../LICENSE) 文件。

## 相关链接

- vagus-chain 项目: https://github.com/vagus-chain/vagus-chain
- Vagus 主项目: https://github.com/Beijing-Datoms-Technology-Corp/vagus
- 文档网站: https://docs.vagus.ai
- 社区论坛: https://forum.vagus.ai
