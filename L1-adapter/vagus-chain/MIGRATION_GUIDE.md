# Vagus Solidity to Native Migration Guide

## 概述

本文档提供了将 Vagus 核心 Solidity 智能合约迁移到 vagus-chain L1 原生协议的完整指南。这个迁移将显著提升系统性能、安全性和可维护性。

## 迁移战略

### 为什么选择原生协议？

1. **极致性能**: 核心逻辑以原生 Rust 代码运行，不受 Wasm 虚拟机限制
2. **零/极低 Gas 成本**: 协议级操作可以设定极低费用
3. **更高安全性**: 攻击面大幅缩小，核心逻辑受 L1 保护
4. **更简单架构**: L1 和核心协议成为内聚整体

### 迁移原则

1. **行为等价**: 原生实现必须与 Solidity 版本完全等价
2. **接口优先**: 先定义接口，再实现功能
3. **测试驱动**: 以测试用例验证等价性
4. **渐进迁移**: 分阶段迁移，确保系统稳定

## 迁移工作流总览

### 迁移目标
将现有的 Solidity 智能合约实现迁移到 vagus-chain L1 的原生协议实现，同时保持与现有客户端代码的完全兼容性。

### 关键原则
1. **零停机迁移**: 确保现有系统在迁移期间保持正常运行
2. **渐进式切换**: 支持 A/B 测试和流量逐步切换
3. **回滚能力**: 在出现问题时能够快速回滚到 Solidity 版本
4. **数据一致性**: 确保迁移前后数据状态的一致性

### 迁移范围
- **核心合约**: ANSStateManager, CapabilityIssuer, VagalBrake, AfferentInbox, ReflexArc
- **客户端组件**: Tone Oracle, Vagus Gateway
- **部署脚本**: 多链部署支持
- **测试套件**: 完整的回归测试

## 第一阶段：准备阶段 (Week 1-2)

### 1.1 环境准备

#### 任务 1.1.1: 部署 vagus-chain L1 测试网络
```bash
# 1. 克隆 vagus-chain 仓库
git clone https://github.com/vagus-chain/vagus-chain.git
cd vagus-chain

# 2. 启动开发网络
cargo run --bin vagus-chain -- --dev --tmp

# 3. 验证网络状态
curl -X POST http://localhost:26657 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"status","params":[],"id":1}'
```

#### 任务 1.1.2: 部署现有 Solidity 合约作为基准
```bash
# 1. 部署到测试网络
cd /home/ubuntu/vagus
./infra/devnet/anvil.sh &
forge script script/DeployCore.s.sol --rpc-url http://127.0.0.1:8545 --broadcast

# 2. 记录部署地址
cat contracts/script/DevnetConfig.json
```

### 1.2 接口规范验证

#### 任务 1.2.1: 验证预编译地址可用性
```bash
# 检查预编译地址是否可调用
cast call 0x0000000000000000000000000000000000000001 \
  "getExecutorState(uint256)" 1 \
  --rpc-url http://localhost:26657
```

#### 任务 1.2.2: 对比 ABI 兼容性
```bash
# 生成 Solidity 合约的 ABI
forge inspect ANSStateManager abi > solidity_ans_abi.json

# 对比原生实现 ABI
diff solidity_ans_abi.json native_ans_abi.json
```

## 第二阶段：原生实现开发 (Week 3-6)

### 2.1 vagus-chain 团队任务

#### 任务 2.1.1: 实现 ANSStateManager 原生模块
基于 `ANS_STATE_MACHINE_SPECIFICATION.md` 实现状态机逻辑：

```rust
// 在 vagus-chain/src/native/ans_state_manager.rs
pub struct ANSStateManager {
    config: HysteresisConfig,
    executor_states: HashMap<u256, ExecutorState>,
}

impl ANSStateManager {
    pub fn update_tone(&mut self, executor_id: u256, tone: u32) -> Result<(), VagusError> {
        // 实现完整的状态转换逻辑
        // 参考 ANS_STATE_MACHINE_SPECIFICATION.md
    }
}
```

#### 任务 2.1.2: 实现其他核心模块
按照相同模式实现 CapabilityIssuer, VagalBrake, AfferentInbox, ReflexArc。

#### 任务 2.1.3: 集成到 L1 运行时
```rust
// 在 vagus-chain/src/runtime.rs
pub fn create_precompiles() -> Vec<Box<dyn Precompile>> {
    vec![
        Box::new(ANSStateManagerPrecompile::new()),
        Box::new(CapabilityIssuerPrecompile::new()),
        Box::new(VagalBrakePrecompile::new()),
        Box::new(AfferentInboxPrecompile::new()),
        Box::new(ReflexArcPrecompile::new()),
    ]
}
```

### 2.2 Vagus 团队并行任务

#### 任务 2.2.1: 实现客户端适配器
基于 `NATIVE_CLIENT_ADAPTERS.md` 实现 Rust 客户端：

```rust
// 在 vagus/gateway/crates/vagus-chain/src/native_clients.rs
pub struct NativeANSStateManager {
    rpc_client: RpcClient,
}

impl NativeANSStateManager {
    pub async fn update_tone(&self, executor_id: u256, tone: u32) -> Result<(), VagusError> {
        // 调用预编译合约
        self.rpc_client.call_precompile(ANS_STATE_MANAGER_ADDRESS, "updateTone", (executor_id, tone)).await
    }
}
```

#### 任务 2.2.2: 运行集成测试
```bash
# 运行测试套件
cd /home/ubuntu/vagus
cargo test --test integration --features native-contracts

# 验证与 Solidity 版本的一致性
cargo test --test regression --features native-contracts
```

## 第三阶段：集成测试阶段 (Week 7-8)

### 3.1 功能验证

#### 任务 3.1.1: 运行完整测试套件
```bash
# 运行所有集成测试
cd /home/ubuntu/vagus
cargo test --workspace --features native-contracts

# 生成测试覆盖率报告
cargo tarpaulin --workspace --features native-contracts --out Html
open tarpaulin-report.html
```

#### 任务 3.1.2: 性能基准测试
```bash
# 运行性能基准测试
cargo bench --features native-contracts

# 对比原生实现与 Solidity 实现的性能
./scripts/benchmark-comparison.sh
```

#### 任务 3.1.3: 端到端集成测试
```bash
# 启动完整系统测试
./scripts/e2e-test.sh --native-mode

# 验证事件流和状态一致性
./scripts/verify-event-consistency.sh
```

### 3.2 兼容性验证

#### 任务 3.2.1: ABI 兼容性检查
```bash
# 验证所有函数选择器匹配
./scripts/verify-abi-compatibility.sh

# 检查事件签名一致性
./scripts/verify-event-signatures.sh
```

#### 任务 3.2.2: 行为一致性测试
```bash
# 运行回归测试套件
cargo test --test regression

# 验证状态转换逻辑
./scripts/verify-state-machine.sh
```

## 第四阶段：生产迁移阶段 (Week 9-12)

### 4.1 生产环境准备

#### 任务 4.1.1: 部署 vagus-chain L1 主网
```bash
# 1. 部署主网节点
kubectl apply -f k8s/vagus-chain-mainnet.yaml

# 2. 验证网络同步
curl -X POST https://vagus-chain-mainnet.example.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"status","params":[],"id":1}'

# 3. 验证预编译合约可用性
cast call 0x0000000000000000000000000000000000000001 \
  "getExecutorState(uint256)" 1 \
  --rpc-url https://vagus-chain-mainnet.example.com
```

#### 任务 4.1.2: 配置客户端切换机制
```bash
# 1. 更新配置管理系统
kubectl apply -f k8s/config-management.yaml

# 2. 设置功能开关
export VAGUS_NATIVE_MODE_ENABLED=false  # 初始关闭
export VAGUS_TRAFFIC_SPLIT=0            # 初始 0% 流量

# 3. 配置健康检查
kubectl apply -f k8s/health-checks.yaml
```

### 4.2 渐进式流量切换

#### 任务 4.2.1: 第 1 天 - 10% 流量切换
```bash
# 启用原生模式
export VAGUS_NATIVE_MODE_ENABLED=true

# 设置 10% 流量切换
export VAGUS_TRAFFIC_SPLIT=10

# 重新部署客户端
kubectl rollout restart deployment/tone-oracle
kubectl rollout restart deployment/vagus-gateway

# 监控关键指标
./scripts/monitor-migration.sh --phase=1
```

#### 任务 4.2.2: 第 2-3 天 - 50% 流量切换
```bash
# 增加到 50% 流量
export VAGUS_TRAFFIC_SPLIT=50

# 重新部署
kubectl rollout restart deployment/tone-oracle
kubectl rollout restart deployment/vagus-gateway

# 详细监控
./scripts/monitor-migration.sh --phase=2
```

#### 任务 4.2.3: 第 4-7 天 - 100% 流量切换
```bash
# 切换到 100% 流量
export VAGUS_TRAFFIC_SPLIT=100

# 最终部署
kubectl rollout restart deployment/tone-oracle
kubectl rollout restart deployment/vagus-gateway

# 最终验证
./scripts/verify-full-migration.sh
```

### 4.3 迁移后优化

#### 任务 4.3.1: 清理遗留代码
```bash
# 停止 Solidity 合约部署
kubectl delete deployment/solidity-contracts

# 清理旧的配置文件
./scripts/cleanup-legacy-configs.sh

# 更新文档
./scripts/update-documentation.sh
```

#### 任务 4.3.2: 性能优化
```bash
# 启用原生特定的优化
export VAGUS_OPTIMIZATION_LEVEL=native

# 调整超时设置
export VAGUS_RPC_TIMEOUT=50ms  # 原生实现更快

# 重新部署
kubectl rollout restart deployment/tone-oracle
kubectl rollout restart deployment/vagus-gateway
```

## 第五阶段：监控和维护 (Week 13+)

### 5.1 监控设置

#### 任务 5.1.1: 部署监控栈
```bash
# 部署 Prometheus 和 Grafana
kubectl apply -f k8s/monitoring-stack.yaml

# 配置原生特定的指标
kubectl apply -f k8s/native-metrics.yaml

# 设置告警规则
kubectl apply -f k8s/alerting-rules.yaml
```

#### 任务 5.1.2: 设置性能监控
```yaml
# native-performance-dashboard.json
{
  "dashboard": {
    "title": "Vagus Native Performance",
    "panels": [
      {
        "title": "ANS State Update Latency",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(ans_state_update_duration_bucket[5m]))",
            "legendFormat": "95th percentile"
          }
        ]
      },
      {
        "title": "Capability Issuance Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(capability_issued_total[5m])",
            "legendFormat": "Capabilities per second"
          }
        ]
      }
    ]
  }
}
```

### 5.2 定期维护

#### 任务 5.2.1: 建立维护流程
```bash
# 每周性能检查
0 2 * * 1 ./scripts/weekly-performance-check.sh

# 每月回归测试
0 3 1 * * ./scripts/monthly-regression-test.sh

# 每季度安全审计
0 4 1 */3 * ./scripts/quarterly-security-audit.sh
```

#### 任务 5.2.2: 备份和恢复
```bash
# 每日状态备份
0 1 * * * ./scripts/backup-chain-state.sh

# 灾难恢复测试
0 5 15 * * ./scripts/disaster-recovery-test.sh
```

## 故障排除和回滚计划

### 紧急回滚程序

#### 场景 1: 性能问题
```bash
# 立即回滚到 Solidity 版本
export VAGUS_TRAFFIC_SPLIT=0
kubectl rollout restart deployment/tone-oracle
kubectl rollout restart deployment/vagus-gateway

# 分析性能问题
./scripts/analyze-performance-issue.sh
```

#### 场景 2: 功能错误
```bash
# 停用原生模式
export VAGUS_NATIVE_MODE_ENABLED=false
kubectl rollout restart deployment/tone-oracle
kubectl rollout restart deployment/vagus-gateway

# 启动详细日志
export RUST_LOG=vagus_chain=debug
kubectl rollout restart deployment/tone-oracle
```

#### 场景 3: 数据不一致
```bash
# 暂停所有写入操作
kubectl scale deployment tone-oracle --replicas=0
kubectl scale deployment vagus-gateway --replicas=0

# 手动同步状态
./scripts/manual-state-sync.sh

# 逐步恢复服务
kubectl scale deployment tone-oracle --replicas=1
kubectl scale deployment vagus-gateway --replicas=1
```

### 调试工具

#### 实时状态监控
```bash
# 监控执行器状态
watch -n 1 './scripts/monitor-executor-states.sh'

# 查看事件流
tail -f logs/vagus-events.log | jq .

# 检查预编译合约调用
./scripts/debug-precompile-calls.sh
```

#### 性能分析
```bash
# 生成火焰图
cargo flamegraph --bin vagus-gateway --features native-contracts

# 内存使用分析
valgrind --tool=massif target/release/vagus-gateway

# 网络延迟分析
./scripts/network-latency-analysis.sh
```

## 成功指标和验收标准

### 功能验收标准
- [ ] 所有 API 调用 100% 兼容
- [ ] 状态转换逻辑完全一致
- [ ] 事件发出准确无误
- [ ] 权限控制正常工作

### 性能验收标准
- [ ] ANS 更新延迟 < 10ms (目标 < 5ms)
- [ ] 能力发行延迟 < 50ms (目标 < 20ms)
- [ ] 支持 1000+ 并发执行器
- [ ] 内存使用 < 1GB

### 可靠性验收标准
- [ ] 可用性 > 99.9%
- [ ] 错误率 < 0.01%
- [ ] 回滚时间 < 5 分钟
- [ ] 数据一致性 100%

### 业务验收标准
- [ ] 与现有客户端完全兼容
- [ ] Gas 成本降低 > 90%
- [ ] 用户体验无明显变化
- [ ] 安全属性保持不变

## 总结

这个迁移指南提供了完整的从 Solidity 到原生协议的迁移路径，确保：

1. **零停机迁移**: 通过渐进式流量切换确保业务连续性
2. **可回滚性**: 在任何阶段都可以快速回滚到 Solidity 版本
3. **数据一致性**: 通过详细的测试确保状态一致性
4. **性能提升**: 充分利用原生实现的性能优势

通过遵循这个指南，Vagus 协议可以安全、高效地迁移到 vagus-chain L1 的原生实现，实现极致性能和零 Gas 成本的安全协议。

```rust
// 与 Solidity 结构体完全对应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HysteresisConfig {
    pub danger_enter_tone: u32,
    pub safe_exit_tone: u32,
    pub shutdown_enter_tone: u32,
    pub n_danger_enter: u8,
    pub n_safe_exit: u8,
    pub n_shutdown_enter: u8,
    pub dwell_min_sec: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorState {
    pub state: u8,
    pub tone: u32,
    pub updated_at: u64,
    pub last_transition_at: u64,
    pub ctr_danger: u8,
    pub ctr_safe: u8,
    pub ctr_shutdown: u8,
}
```

## 第二阶段：L1 团队实现

### 2.1 实现优先级

1. **ANSStateManager** (最高优先级)
   - 核心状态机逻辑
   - 迟滞性机制
   - 事件发出

2. **CapabilityIssuer** (高优先级)
   - 令牌发行和撤销
   - 权限验证
   - 状态根验证

3. **VagalBrake** (中优先级)
   - 动态参数缩放
   - ANS 状态查询

4. **AfferentInbox** (中优先级)
   - 证据存储
   - 状态根管理

5. **ReflexArc** (低优先级)
   - 自动撤销机制
   - 事件监听

### 2.2 实现检查清单

#### ANSStateManager 实现
- [ ] 状态枚举定义 (SAFE, DANGER, SHUTDOWN)
- [ ] 迟滞性配置结构
- [ ] 执行器状态结构
- [ ] updateTone 函数实现
- [ ] 状态转换逻辑
- [ ] 计数器更新逻辑
- [ ] 最小停留时间检查
- [ ] guardFor 函数实现
- [ ] getExecutorState 函数实现
- [ ] VagalToneUpdated 事件发出
- [ ] ReflexArc 通知机制
- [ ] 权限验证
- [ ] 输入验证
- [ ] 错误处理

#### CapabilityIssuer 实现
- [ ] TokenMeta 结构定义
- [ ] Intent 结构定义
- [ ] issueCapability 函数实现
- [ ] VagalBrake 验证
- [ ] AfferentInbox 验证
- [ ] 令牌元数据存储
- [ ] 活跃令牌列表管理
- [ ] revoke 函数实现
- [ ] isValid 函数实现
- [ ] activeTokensOf 函数实现
- [ ] CapabilityIssued 事件发出
- [ ] CapabilityRevoked 事件发出
- [ ] 权限验证
- [ ] 输入验证
- [ ] 错误处理

### 2.3 存储模型实现

```rust
// Merkle Trie 键结构
pub const ANS_STATE_PREFIX: &str = "ans_state_manager:executor:";
pub const CAPABILITY_TOKEN_PREFIX: &str = "capability_issuer:token:";
pub const ACTIVE_TOKENS_PREFIX: &str = "capability_issuer:active:";
pub const EVIDENCE_PREFIX: &str = "afferent_inbox:evidence:";

// 存储操作示例
impl ANSStateManager {
    fn store_executor_state(&self, executor_id: u256, state: &ExecutorState) -> Result<()> {
        let key = format!("{}{}", ANS_STATE_PREFIX, executor_id);
        let value = bincode::serialize(state)?;
        self.storage.set(key.as_bytes(), &value)?;
        Ok(())
    }
    
    fn load_executor_state(&self, executor_id: u256) -> Result<Option<ExecutorState>> {
        let key = format!("{}{}", ANS_STATE_PREFIX, executor_id);
        if let Some(value) = self.storage.get(key.as_bytes())? {
            let state = bincode::deserialize(&value)?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }
}
```

## 第三阶段：测试和验证

### 3.1 单元测试

每个原生函数都必须有对应的单元测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ans_safe_to_danger_transition() {
        let mut ans = ANSStateManager::new();
        let owner = ans.get_owner();
        let executor_id = 1;
        
        // 第一次更新 - 应该保持 SAFE
        ans.update_tone(owner, executor_id, 350000).unwrap();
        let state = ans.get_executor_state(executor_id);
        assert_eq!(state.state, SAFE);
        assert_eq!(state.ctr_danger, 1);
        
        // 第二次更新 - 应该保持 SAFE
        ans.update_tone(owner, executor_id, 360000).unwrap();
        let state = ans.get_executor_state(executor_id);
        assert_eq!(state.state, SAFE);
        assert_eq!(state.ctr_danger, 2);
        
        // 第三次更新 - 应该转换为 DANGER
        ans.update_tone(owner, executor_id, 370000).unwrap();
        let state = ans.get_executor_state(executor_id);
        assert_eq!(state.state, DANGER);
        assert_eq!(state.ctr_danger, 0); // 计数器应该重置
    }
}
```

### 3.2 集成测试

验证合约间的交互：

```rust
#[test]
fn test_capability_issuance_flow() {
    let mut ans = ANSStateManager::new();
    let mut issuer = CapabilityIssuer::new();
    let mut vagal_brake = VagalBrake::new();
    
    // 设置依赖关系
    issuer.set_vagal_brake(vagal_brake.address());
    vagal_brake.set_ans_state_manager(ans.address());
    
    // 确保执行器在 SAFE 状态
    let owner = ans.get_owner();
    ans.update_tone(owner, 1, 100000).unwrap();
    
    // 创建意图
    let intent = create_test_intent();
    
    // 获取预期的 scaledLimitsHash
    let (expected_hash, allowed) = vagal_brake.preview_brake(&intent);
    assert!(allowed);
    
    // 发行能力令牌
    let token_id = issuer.issue_capability(intent, expected_hash).unwrap();
    
    // 验证令牌元数据
    let meta = issuer.get_token_meta(token_id);
    assert_eq!(meta.executor_id, intent.executor_id);
    assert!(!meta.revoked);
}
```

### 3.3 回归测试

确保原生实现与 Solidity 版本行为完全一致：

```rust
#[test]
fn test_solidity_compatibility() {
    // 部署 Solidity 版本
    let solidity_ans = deploy_solidity_ans();
    
    // 部署原生版本
    let native_ans = ANSStateManager::new();
    
    // 执行相同的操作序列
    let test_sequence = vec![
        (1, 100000),
        (1, 200000),
        (1, 350000),
        (1, 360000),
        (1, 370000),
        (1, 100000),
        (1, 120000),
        (1, 140000),
        (1, 160000),
        (1, 180000),
    ];
    
    for (executor_id, tone) in test_sequence {
        solidity_ans.update_tone(executor_id, tone);
        native_ans.update_tone(owner, executor_id, tone).unwrap();
        
        // 验证状态一致
        let solidity_state = solidity_ans.get_executor_state(executor_id);
        let native_state = native_ans.get_executor_state(executor_id);
        
        assert_eq!(solidity_state.state, native_state.state);
        assert_eq!(solidity_state.tone, native_state.tone);
        assert_eq!(solidity_state.ctr_danger, native_state.ctr_danger);
        assert_eq!(solidity_state.ctr_safe, native_state.ctr_safe);
        assert_eq!(solidity_state.ctr_shutdown, native_state.ctr_shutdown);
    }
}
```

## 第四阶段：客户端适配

### 4.1 更新客户端配置

```rust
// 检测原生模式
fn is_native_mode() -> bool {
    std::env::var("VAGUS_USE_NATIVE_CONTRACTS")
        .map(|v| v == "true")
        .unwrap_or(false)
}

// 选择客户端实现
async fn create_chain_client(config: ChainConfig) -> Result<Arc<dyn ChainClient>> {
    if is_native_mode() {
        let client = NativeChainClient::new(&config).await?;
        Ok(Arc::new(client))
    } else {
        let client = EvmChainClient::new(&config).await?;
        Ok(Arc::new(client))
    }
}
```

### 4.2 更新部署脚本

```bash
#!/bin/bash
# deploy-native.sh

echo "Deploying Vagus with native contracts..."

# 检查原生模式
if [ "$VAGUS_USE_NATIVE_CONTRACTS" = "true" ]; then
    echo "Using native contracts on vagus-chain L1"
    
    # 启动 vagus-chain L1
    ./infra/devnet/vagus-chain.sh
    
    # 等待链启动
    sleep 10
    
    # 启动服务
    export VAGUS_CHAIN_RPC_URL=http://localhost:26657
    cargo run --bin tone-oracle start &
    cargo run --bin vagus-gateway start --executor-id 1 --chain evm --rpc-url http://localhost:26657 &
    
    echo "Native deployment complete"
else
    echo "Using traditional smart contracts"
    # 传统部署逻辑
fi
```

## 第五阶段：生产部署

### 5.1 预生产测试

```bash
# 1. 启动测试环境
./infra/devnet/vagus-chain.sh
./infra/devnet/anvil.sh

# 2. 运行完整测试套件
cargo test --workspace --features native-contracts

# 3. 运行集成测试
cd tests/golden
cargo run -- run-all --evm-rpc http://localhost:8545 --cosmos-rpc http://localhost:26657

# 4. 运行性能测试
cargo run --bin performance-test -- --native-mode

# 5. 运行回归测试
cargo run --bin regression-test -- --compare-solidity
```

### 5.2 生产部署步骤

```bash
# 1. 备份当前系统
./scripts/backup-system.sh

# 2. 部署原生合约到 vagus-chain L1
./scripts/deploy-native-contracts.sh

# 3. 更新客户端配置
./scripts/update-client-configs.sh --native-mode

# 4. 逐步切换服务
./scripts/rolling-update.sh --phase=1  # 切换 10% 流量
./scripts/rolling-update.sh --phase=2  # 切换 50% 流量
./scripts/rolling-update.sh --phase=3  # 切换 100% 流量

# 5. 验证系统健康
./scripts/health-check.sh --comprehensive

# 6. 监控和告警
./scripts/setup-monitoring.sh --native-mode
```

### 5.3 回滚计划

```bash
# 紧急回滚脚本
#!/bin/bash
# rollback.sh

echo "Rolling back to Solidity contracts..."

# 1. 停止原生服务
pkill -f "tone-oracle"
pkill -f "vagus-gateway"

# 2. 恢复传统配置
./scripts/update-client-configs.sh --solidity-mode

# 3. 启动传统服务
cargo run --bin tone-oracle start --solidity-mode &
cargo run --bin vagus-gateway start --solidity-mode &

# 4. 验证回滚成功
./scripts/health-check.sh --solidity-mode

echo "Rollback complete"
```

## 第六阶段：监控和维护

### 6.1 性能监控

```yaml
# monitoring/native-metrics.yml
native_contracts:
  ans_state_manager:
    - name: "update_tone_duration"
      type: "histogram"
      help: "Duration of updateTone calls"
      labels: ["executor_id"]
    
    - name: "state_transitions_total"
      type: "counter"
      help: "Total number of state transitions"
      labels: ["from_state", "to_state"]
  
  capability_issuer:
    - name: "issue_capability_duration"
      type: "histogram"
      help: "Duration of issueCapability calls"
      labels: ["executor_id"]
    
    - name: "tokens_issued_total"
      type: "counter"
      help: "Total number of tokens issued"
      labels: ["executor_id"]
    
    - name: "tokens_revoked_total"
      type: "counter"
      help: "Total number of tokens revoked"
      labels: ["reason"]
```

### 6.2 告警规则

```yaml
# monitoring/alert-rules-native.yml
groups:
  - name: vagus_native
    rules:
      - alert: NativeContractHighLatency
        expr: histogram_quantile(0.95, update_tone_duration) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Native contract calls are slow"
          description: "95th percentile latency is {{ $value }}s"
      
      - alert: NativeContractErrors
        expr: rate(contract_errors_total[5m]) > 0.01
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "High error rate in native contracts"
          description: "Error rate is {{ $value }} errors/sec"
      
      - alert: StateTransitionAnomaly
        expr: rate(state_transitions_total[1h]) > 100
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Unusual state transition frequency"
          description: "{{ $value }} transitions per hour"
```

### 6.3 维护任务

```bash
# 定期维护脚本
#!/bin/bash
# maintenance.sh

# 1. 每日健康检查
./scripts/health-check.sh --comprehensive

# 2. 性能基准测试
./scripts/performance-benchmark.sh --native-mode

# 3. 数据一致性检查
./scripts/consistency-check.sh --native-vs-solidity

# 4. 日志分析
./scripts/analyze-logs.sh --native-mode --last-24h

# 5. 容量规划
./scripts/capacity-analysis.sh --native-mode
```

## 协作指南

### L1 团队职责

1. **实现原生合约**
   - 按照接口规范实现所有函数
   - 确保状态模型完全一致
   - 实现所有必需的事件

2. **性能优化**
   - 确保函数调用在 100ms 内完成
   - 优化存储访问模式
   - 实现批处理支持

3. **安全审计**
   - 代码审查
   - 安全测试
   - 漏洞修复

### Vagus 团队职责

1. **接口定义**
   - 提供精确的 ABI 规范
   - 定义状态模型
   - 编写测试用例

2. **客户端适配**
   - 更新客户端代码
   - 实现原生模式检测
   - 维护向后兼容性

3. **测试验证**
   - 编写回归测试
   - 执行集成测试
   - 验证行为等价性

### 沟通机制

1. **每日同步会议**
   - 进度更新
   - 问题讨论
   - 决策记录

2. **技术审查**
   - 代码审查
   - 设计审查
   - 测试审查

3. **文档维护**
   - 接口文档
   - 实现文档
   - 测试文档

## 成功标准

### 功能标准
- [ ] 所有原生函数正确实现
- [ ] 状态模型完全一致
- [ ] 事件正确发出
- [ ] 权限验证正确

### 性能标准
- [ ] 函数调用 < 100ms
- [ ] 状态查询 < 10ms
- [ ] 支持 1000+ 并发用户
- [ ] 内存使用 < 1GB

### 质量标准
- [ ] 单元测试覆盖率 > 90%
- [ ] 集成测试全部通过
- [ ] 回归测试全部通过
- [ ] 安全审计通过

### 运维标准
- [ ] 监控指标完整
- [ ] 告警规则有效
- [ ] 日志记录完整
- [ ] 文档齐全

## 风险缓解

### 技术风险
- **性能不达标**: 提前进行性能测试和优化
- **兼容性问题**: 严格的回归测试
- **安全漏洞**: 全面的安全审计

### 项目风险
- **进度延迟**: 每日进度跟踪和风险预警
- **资源不足**: 提前规划资源分配
- **沟通问题**: 建立有效的沟通机制

### 运维风险
- **部署失败**: 完善的回滚计划
- **数据丢失**: 多重备份策略
- **服务中断**: 渐进式部署

## 总结

这个迁移项目将显著提升 Vagus 系统的性能、安全性和可维护性。通过严格的接口定义、全面的测试验证和渐进式部署，我们可以确保迁移过程的安全和成功。

关键成功因素：
1. **接口优先**: 先定义接口，再实现功能
2. **测试驱动**: 以测试用例验证等价性
3. **渐进迁移**: 分阶段部署，降低风险
4. **密切协作**: L1 团队和 Vagus 团队紧密配合

通过遵循本指南，我们可以成功完成从 Solidity 智能合约到 vagus-chain L1 原生协议的迁移。
