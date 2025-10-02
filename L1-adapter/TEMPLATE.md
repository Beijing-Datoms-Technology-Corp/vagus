# L1 适配器模板

## 概述

本模板用于创建新的 L1 适配器。请复制此模板并根据目标 L1 的特性进行定制。

## 创建新适配器

### 1. 创建目录结构
```bash
mkdir L1-adapter/new-l1
cd L1-adapter/new-l1
```

### 2. 复制模板文件
```bash
cp ../TEMPLATE.md README.md
```

### 3. 实现核心接口
创建以下文件：
- `adapter.rs` - 实现 `L1Adapter` trait
- `client.rs` - L1 特定客户端逻辑
- `types.rs` - L1 特定类型定义
- `config.rs` - 配置管理

### 4. 编写文档
- 更新 `README.md`
- 创建接口规范文档
- 创建集成测试文档

## 模板 README

```markdown
# [L1 Name] L1 适配器

## 概述

本目录包含针对 [L1 Name] 的适配实现。[简要描述 L1 特性]

## 状态

🚧 **开发中** - 此适配器尚未实现

## 计划特性

### [L1 特定实现方式]
- 基于 [技术栈] 的 [实现方式]
- 支持 [L1 特定特性]
- [费用模型/其他特性]

### 核心合约
- `[合约1]` - [功能描述]
- `[合约2]` - [功能描述]
- `[合约3]` - [功能描述]
- `[合约4]` - [功能描述]
- `[合约5]` - [功能描述]

### 部署地址
合约地址将在部署后更新：
- ANSStateManager: `TBD`
- CapabilityIssuer: `TBD`
- VagalBrake: `TBD`
- AfferentInbox: `TBD`
- ReflexArc: `TBD`

## 开发计划

### 阶段 1: 合约开发
- [ ] 实现 [L1 特定合约]
- [ ] 编写单元测试
- [ ] [L1 特定优化]

### 阶段 2: 客户端适配
- [ ] 实现 [L1 Name] 客户端
- [ ] 集成 [L1 特定库]
- [ ] [L1 特定功能]

### 阶段 3: 测试和部署
- [ ] 集成测试
- [ ] [L1 特定部署]
- [ ] 监控和告警

## 贡献

欢迎贡献代码和文档！请查看 [L1-adapter/README.md](../README.md) 了解贡献指南。
```

## 实现指南

### 核心接口实现
```rust
use async_trait::async_trait;
use vagus_chain::{L1Adapter, ANSState, Intent, AfferentEvidencePacket, Event};

pub struct NewL1Adapter {
    // L1 特定字段
}

#[async_trait]
impl L1Adapter for NewL1Adapter {
    async fn update_ans_state(&self, executor_id: u256, tone: u32) -> Result<()> {
        // 实现 ANS 状态更新
    }
    
    async fn get_ans_state(&self, executor_id: u256) -> Result<ANSState> {
        // 实现 ANS 状态查询
    }
    
    async fn issue_capability(&self, intent: &Intent) -> Result<u256> {
        // 实现能力令牌发行
    }
    
    async fn revoke_capability(&self, token_id: u256, reason: u8) -> Result<()> {
        // 实现能力令牌撤销
    }
    
    async fn is_capability_valid(&self, token_id: u256) -> Result<bool> {
        // 实现能力令牌验证
    }
    
    async fn submit_evidence(&self, aep: &AfferentEvidencePacket) -> Result<()> {
        // 实现证据提交
    }
    
    async fn get_latest_state_root(&self, executor_id: u256) -> Result<[u8; 32]> {
        // 实现状态根查询
    }
    
    async fn subscribe_events<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        // 实现事件监听
    }
}
```

### 配置管理
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewL1Config {
    pub rpc_url: String,
    pub chain_id: u64,
    pub private_key: Option<String>,
    pub contract_addresses: HashMap<String, String>,
    // L1 特定配置
}

impl NewL1Config {
    pub fn from_env() -> Result<Self> {
        // 从环境变量加载配置
    }
}
```

### 测试实现
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_ans_state_update() {
        let adapter = NewL1Adapter::new(test_config()).await?;
        
        // 测试 ANS 状态更新
        adapter.update_ans_state(1, 100000).await?;
        let state = adapter.get_ans_state(1).await?;
        assert_eq!(state, ANSState::SAFE);
    }
    
    #[tokio::test]
    async fn test_capability_issuance() {
        let adapter = NewL1Adapter::new(test_config()).await?;
        
        // 测试能力令牌发行
        let intent = create_test_intent();
        let token_id = adapter.issue_capability(&intent).await?;
        assert!(token_id > 0);
    }
}
```

## 检查清单

### 开发前
- [ ] 研究目标 L1 的特性和限制
- [ ] 选择合适的开发库和工具
- [ ] 设计适配器架构
- [ ] 创建项目结构

### 开发中
- [ ] 实现核心接口
- [ ] 编写单元测试
- [ ] 实现配置管理
- [ ] 添加错误处理
- [ ] 编写文档

### 开发后
- [ ] 集成测试
- [ ] 性能测试
- [ ] 安全审计
- [ ] 部署测试
- [ ] 用户文档

## 相关资源

- [L1-adapter/README.md](../README.md) - L1 适配器总览
- [vagus-chain/README.md](./vagus-chain/README.md) - vagus-chain 适配器示例
- [Vagus 主项目](https://github.com/Beijing-Datoms-Technology-Corp/vagus) - 主项目仓库
