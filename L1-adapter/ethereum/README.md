# Ethereum L1 适配器

## 概述

本目录将包含针对 Ethereum 及其兼容链的适配实现。Ethereum 适配器基于 Solidity 智能合约实现，支持标准的 EVM 环境。

## 状态

🚧 **开发中** - 此适配器尚未实现

## 计划特性

### 智能合约实现
- 基于 Solidity 的 Vagus 核心合约
- 支持 EVM 兼容链 (Ethereum, Polygon, BSC 等)
- 标准 Gas 费用模型

### 核心合约
- `ANSStateManager.sol` - 状态管理合约
- `CapabilityIssuer.sol` - 能力令牌合约
- `VagalBrake.sol` - 动态缩放合约
- `AfferentInbox.sol` - 证据处理合约
- `ReflexArc.sol` - 自动撤销合约

### 部署地址
合约地址将在部署后更新：
- ANSStateManager: `TBD`
- CapabilityIssuer: `TBD`
- VagalBrake: `TBD`
- AfferentInbox: `TBD`
- ReflexArc: `TBD`

## 开发计划

### 阶段 1: 合约开发
- [ ] 实现核心 Solidity 合约
- [ ] 编写单元测试
- [ ] Gas 优化

### 阶段 2: 客户端适配
- [ ] 实现 Ethereum 客户端
- [ ] 集成 Web3 库
- [ ] 事件监听机制

### 阶段 3: 测试和部署
- [ ] 集成测试
- [ ] 主网部署
- [ ] 监控和告警

## 贡献

欢迎贡献代码和文档！请查看 [L1-adapter/README.md](../README.md) 了解贡献指南。
