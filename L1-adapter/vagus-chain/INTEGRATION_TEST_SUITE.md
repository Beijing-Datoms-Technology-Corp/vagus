# Vagus Native Integration Test Suite

## 概述

本文档定义了将 Vagus Solidity 合约迁移到 vagus-chain L1 原生协议所需的全面集成测试用例。这些测试用例将确保原生实现与 Solidity 版本的行为完全一致。

## 测试环境设置

### 测试网络配置
```yaml
test_network:
  name: "vagus-testnet"
  chain_id: "vagus-test-1"
  rpc_url: "http://localhost:26657"
  native_contracts:
    ans_state_manager: "0x0000000000000000000000000000000000000001"
    capability_issuer: "0x0000000000000000000000000000000000000002"
    vagal_brake: "0x0000000000000000000000000000000000000003"
    afferent_inbox: "0x0000000000000000000000000000000000000004"
    reflex_arc: "0x0000000000000000000000000000000000000005"
```

### 测试数据准备
```rust
pub struct TestData {
    pub owner: [u8; 20],
    pub oracle: [u8; 20],
    pub planner: [u8; 20],
    pub executor_ids: Vec<u256>,
    pub test_intents: Vec<Intent>,
    pub test_aep: AfferentEvidencePacket,
}

pub fn create_test_intent(executor_id: u256) -> Intent {
    Intent {
        executor_id,
        action_id: [0x01; 32],
        params: vec![],
        envelope_hash: [0x02; 32],
        pre_state_root: [0x03; 32],
        not_before: 1000000,
        not_after: 2000000,
        max_duration_ms: 30000,
        max_energy_j: 1000,
        planner: [0x04; 20],
        nonce: 1,
    }
}

pub fn create_test_aep(executor_id: u256) -> AfferentEvidencePacket {
    AfferentEvidencePacket {
        executor_id,
        state_root: [0x01; 32],
        metrics_hash: [0x02; 32],
        signature: vec![0x03; 64],
        timestamp: 1000000,
    }
}
```

## 1. ANSStateManager 集成测试

### 1.1 基础功能测试

#### 测试用例 1.1.1: 初始化状态
**描述**: 验证 ANSStateManager 的初始状态
**输入**: 无
**预期结果**:
- 默认配置正确设置
- 所有执行器初始状态为 SAFE
- 所有计数器为 0

```rust
#[test]
fn test_ans_initialization() {
    let ans = ANSStateManager::new();
    
    // 验证默认配置
    let config = ans.get_config();
    assert_eq!(config.danger_enter_tone, 300000);
    assert_eq!(config.safe_exit_tone, 150000);
    assert_eq!(config.shutdown_enter_tone, 700000);
    assert_eq!(config.n_danger_enter, 3);
    assert_eq!(config.n_safe_exit, 5);
    assert_eq!(config.n_shutdown_enter, 2);
    assert_eq!(config.dwell_min_sec, 60);
    
    // 验证执行器初始状态
    let state = ans.get_executor_state(1);
    assert_eq!(state.state, SAFE);
    assert_eq!(state.tone, 0);
    assert_eq!(state.ctr_danger, 0);
    assert_eq!(state.ctr_safe, 0);
    assert_eq!(state.ctr_shutdown, 0);
}
```

#### 测试用例 1.1.2: 权限验证
**描述**: 验证只有 owner 可以更新 tone
**输入**: 非 owner 地址尝试更新 tone
**预期结果**: 交易失败，返回 Unauthorized 错误

```rust
#[test]
fn test_ans_unauthorized_update() {
    let mut ans = ANSStateManager::new();
    let non_owner = [0x01; 20];
    
    let result = ans.update_tone_from(non_owner, 1, 100000);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), VagusError::Unauthorized);
}
```

#### 测试用例 1.1.3: 输入验证
**描述**: 验证 tone 值范围检查
**输入**: 超出范围的 tone 值 (> 1,000,000)
**预期结果**: 交易失败，返回 InvalidInput 错误

```rust
#[test]
fn test_ans_invalid_tone_range() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();
    
    let result = ans.update_tone_from(owner, 1, 1_000_001);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), VagusError::InvalidInput);
}
```

#### 测试用例 1.1.4: 边界值测试 - 最小值
**描述**: 验证 tone 值为 0 的处理
**输入**: tone = 0
**预期结果**: 成功更新状态

```rust
#[test]
fn test_ans_minimum_tone() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();

    ans.update_tone_from(owner, 1, 0).unwrap();
    let state = ans.get_executor_state(1);
    assert_eq!(state.tone, 0);
    assert_eq!(state.state, SAFE);
}
```

#### 测试用例 1.1.5: 边界值测试 - 最大值
**描述**: 验证 tone 值为 1,000,000 的处理
**输入**: tone = 1,000,000
**预期结果**: 成功更新状态

```rust
#[test]
fn test_ans_maximum_tone() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();

    ans.update_tone_from(owner, 1, 1_000_000).unwrap();
    let state = ans.get_executor_state(1);
    assert_eq!(state.tone, 1_000_000);
}
```

#### 测试用例 1.1.6: 无效执行器 ID
**描述**: 验证执行器 ID 为 0 的处理
**输入**: executor_id = 0
**预期结果**: 交易失败，返回 InvalidInput 错误

```rust
#[test]
fn test_ans_invalid_executor_id() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();

    let result = ans.update_tone_from(owner, 0, 100000);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), VagusError::InvalidInput);
}
```

### 1.2 状态转换测试

#### 测试用例 1.2.1: SAFE → DANGER 转换
**描述**: 验证从 SAFE 状态转换到 DANGER 状态
**输入**: 连续 3 次 tone 值 >= 300,000
**预期结果**: 状态转换为 DANGER，发出 VagalToneUpdated 事件

```rust
#[test]
fn test_ans_safe_to_danger_transition() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();
    let executor_id = 1;
    
    // 第一次更新 - 应该保持 SAFE
    ans.update_tone_from(owner, executor_id, 350000).unwrap();
    let state = ans.get_executor_state(executor_id);
    assert_eq!(state.state, SAFE);
    assert_eq!(state.ctr_danger, 1);
    
    // 第二次更新 - 应该保持 SAFE
    ans.update_tone_from(owner, executor_id, 360000).unwrap();
    let state = ans.get_executor_state(executor_id);
    assert_eq!(state.state, SAFE);
    assert_eq!(state.ctr_danger, 2);
    
    // 第三次更新 - 应该转换为 DANGER
    ans.update_tone_from(owner, executor_id, 370000).unwrap();
    let state = ans.get_executor_state(executor_id);
    assert_eq!(state.state, DANGER);
    assert_eq!(state.ctr_danger, 0); // 计数器应该重置
    
    // 验证事件发出
    let events = ans.get_events();
    assert!(events.iter().any(|e| matches!(e, Event::VagalToneUpdated(370000, DANGER, _))));
}
```

#### 测试用例 1.2.2: DANGER → SAFE 转换
**描述**: 验证从 DANGER 状态转换到 SAFE 状态
**输入**: 连续 5 次 tone 值 < 150,000
**预期结果**: 状态转换为 SAFE，发出 VagalToneUpdated 事件

```rust
#[test]
fn test_ans_danger_to_safe_transition() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();
    let executor_id = 1;
    
    // 先转换到 DANGER 状态
    for _ in 0..3 {
        ans.update_tone_from(owner, executor_id, 350000).unwrap();
    }
    let state = ans.get_executor_state(executor_id);
    assert_eq!(state.state, DANGER);
    
    // 连续 5 次低 tone 值
    for i in 0..5 {
        ans.update_tone_from(owner, executor_id, 100000).unwrap();
        let state = ans.get_executor_state(executor_id);
        if i < 4 {
            assert_eq!(state.state, DANGER);
            assert_eq!(state.ctr_safe, i + 1);
        } else {
            assert_eq!(state.state, SAFE);
            assert_eq!(state.ctr_safe, 0);
        }
    }
}
```

#### 测试用例 1.2.3: DANGER → SHUTDOWN 转换
**描述**: 验证从 DANGER 状态转换到 SHUTDOWN 状态
**输入**: 连续 2 次 tone 值 >= 700,000
**预期结果**: 状态转换为 SHUTDOWN，发出 VagalToneUpdated 事件

```rust
#[test]
fn test_ans_danger_to_shutdown_transition() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();
    let executor_id = 1;
    
    // 先转换到 DANGER 状态
    for _ in 0..3 {
        ans.update_tone_from(owner, executor_id, 350000).unwrap();
    }
    let state = ans.get_executor_state(executor_id);
    assert_eq!(state.state, DANGER);
    
    // 连续 2 次超高 tone 值
    ans.update_tone_from(owner, executor_id, 750000).unwrap();
    let state = ans.get_executor_state(executor_id);
    assert_eq!(state.state, DANGER);
    assert_eq!(state.ctr_shutdown, 1);
    
    ans.update_tone_from(owner, executor_id, 800000).unwrap();
    let state = ans.get_executor_state(executor_id);
    assert_eq!(state.state, SHUTDOWN);
    assert_eq!(state.ctr_shutdown, 0);
}
```

### 1.3 迟滞性测试

#### 测试用例 1.3.1: 迟滞性机制验证
**描述**: 验证迟滞性防止状态震荡
**输入**: tone 值在阈值附近波动
**预期结果**: 状态不会频繁切换

```rust
#[test]
fn test_ans_hysteresis_mechanism() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();
    let executor_id = 1;
    
    // 在危险阈值附近波动
    let tone_values = vec![
        200000, // SAFE
        250000, // SAFE
        300000, // SAFE (第一次危险读数)
        310000, // SAFE (第二次危险读数)
        320000, // DANGER (第三次危险读数，转换)
        180000, // DANGER (第一次安全读数)
        160000, // DANGER (第二次安全读数)
        140000, // DANGER (第三次安全读数)
        120000, // DANGER (第四次安全读数)
        100000, // SAFE (第五次安全读数，转换)
    ];
    
    let expected_states = vec![
        SAFE, SAFE, SAFE, SAFE, DANGER,
        DANGER, DANGER, DANGER, DANGER, SAFE
    ];
    
    for (i, tone) in tone_values.iter().enumerate() {
        ans.update_tone_from(owner, executor_id, *tone).unwrap();
        let state = ans.get_executor_state(executor_id);
        assert_eq!(state.state, expected_states[i], "Failed at step {}", i);
    }
}
```

#### 测试用例 1.3.2: 最小停留时间测试
**描述**: 验证最小停留时间机制
**输入**: 在最小停留时间内尝试状态转换
**预期结果**: 状态转换被阻止

```rust
#[test]
fn test_ans_dwell_time_mechanism() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();
    let executor_id = 1;
    
    // 转换到 DANGER
    for _ in 0..3 {
        ans.update_tone_from(owner, executor_id, 350000).unwrap();
    }
    let state = ans.get_executor_state(executor_id);
    assert_eq!(state.state, DANGER);
    
    // 立即尝试转换回 SAFE (应该失败)
    for _ in 0..5 {
        ans.update_tone_from(owner, executor_id, 100000).unwrap();
    }
    let state = ans.get_executor_state(executor_id);
    assert_eq!(state.state, DANGER); // 应该仍然是 DANGER
    
    // 等待最小停留时间后再次尝试
    advance_time(61); // 61 秒
    for _ in 0..5 {
        ans.update_tone_from(owner, executor_id, 100000).unwrap();
    }
    let state = ans.get_executor_state(executor_id);
    assert_eq!(state.state, SAFE); // 现在应该转换成功
}
```

### 1.4 多执行器测试

#### 测试用例 1.4.1: 独立状态管理
**描述**: 验证多个执行器的状态独立管理
**输入**: 不同执行器的不同 tone 值
**预期结果**: 每个执行器的状态独立变化

```rust
#[test]
fn test_ans_multiple_executors() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();
    
    let executor_ids = vec![1, 2, 3];
    
    // 为每个执行器设置不同的状态
    for (i, &executor_id) in executor_ids.iter().enumerate() {
        let tone = 200000 + (i as u32 * 100000);
        ans.update_tone_from(owner, executor_id, tone).unwrap();
    }
    
    // 验证每个执行器的状态
    for (i, &executor_id) in executor_ids.iter().enumerate() {
        let state = ans.get_executor_state(executor_id);
        assert_eq!(state.state, SAFE);
        assert_eq!(state.tone, 200000 + (i as u32 * 100000));
    }
    
    // 让执行器 1 进入 DANGER 状态
    for _ in 0..3 {
        ans.update_tone_from(owner, 1, 350000).unwrap();
    }
    
    // 验证其他执行器状态不受影响
    let state2 = ans.get_executor_state(2);
    let state3 = ans.get_executor_state(3);
    assert_eq!(state2.state, SAFE);
    assert_eq!(state3.state, SAFE);
    
    let state1 = ans.get_executor_state(1);
    assert_eq!(state1.state, DANGER);
}
```

## 2. CapabilityIssuer 集成测试

### 2.1 基础功能测试

#### 测试用例 2.1.1: 能力令牌发行
**描述**: 验证能力令牌的正确发行
**输入**: 有效的 Intent 和 scaledLimitsHash
**预期结果**: 成功发行令牌，发出 CapabilityIssued 事件

```rust
#[test]
fn test_capability_issuance() {
    let mut issuer = CapabilityIssuer::new();
    let mut vagal_brake = VagalBrake::new();
    let mut afferent_inbox = AfferentInbox::new();
    
    // 设置依赖关系
    issuer.set_vagal_brake(vagal_brake.address());
    issuer.set_afferent_inbox(afferent_inbox.address());
    
    // 准备测试数据
    let intent = create_test_intent();
    let state_root = [0x01; 32];
    afferent_inbox.post_aep(1, state_root, [0x02; 32], vec![]);
    
    // 获取预期的 scaledLimitsHash
    let (expected_hash, allowed) = vagal_brake.preview_brake(&intent);
    assert!(allowed);
    
    // 发行能力令牌
    let token_id = issuer.issue_capability(intent.clone(), expected_hash).unwrap();
    
    // 验证令牌元数据
    let meta = issuer.get_token_meta(token_id);
    assert_eq!(meta.executor_id, intent.executor_id);
    assert_eq!(meta.action_id, intent.action_id);
    assert_eq!(meta.scaled_limits_hash, expected_hash);
    assert!(!meta.revoked);
    
    // 验证事件发出
    let events = issuer.get_events();
    assert!(events.iter().any(|e| matches!(e, Event::CapabilityIssued(token_id, _, _, _, _, _, _, _, _))));
}
```

#### 测试用例 2.1.2: 令牌验证
**描述**: 验证令牌有效性检查
**输入**: 有效和无效的令牌 ID
**预期结果**: 正确返回令牌有效性

```rust
#[test]
fn test_capability_validation() {
    let mut issuer = CapabilityIssuer::new();
    // ... 设置依赖关系
    
    // 发行有效令牌
    let intent = create_test_intent();
    let token_id = issuer.issue_capability(intent, expected_hash).unwrap();
    
    // 验证有效令牌
    assert!(issuer.is_valid(token_id));
    
    // 验证不存在的令牌
    assert!(!issuer.is_valid(999));
    
    // 撤销令牌
    issuer.revoke(token_id, 1).unwrap();
    
    // 验证已撤销的令牌
    assert!(!issuer.is_valid(token_id));
}
```

### 2.2 权限和验证测试

#### 测试用例 2.2.1: VagalBrake 验证
**描述**: 验证 VagalBrake 的 scaledLimitsHash 验证
**输入**: 不匹配的 scaledLimitsHash
**预期结果**: 发行失败，返回验证错误

```rust
#[test]
fn test_capability_vagal_brake_validation() {
    let mut issuer = CapabilityIssuer::new();
    // ... 设置依赖关系
    
    let intent = create_test_intent();
    let wrong_hash = [0xFF; 32];
    
    let result = issuer.issue_capability(intent, wrong_hash);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), VagusError::InvalidInput);
}
```

#### 测试用例 2.2.2: 状态根验证
**描述**: 验证 AfferentInbox 的状态根验证
**输入**: 不匹配的 preStateRoot
**预期结果**: 发行失败，返回状态不匹配错误

```rust
#[test]
fn test_capability_state_root_validation() {
    let mut issuer = CapabilityIssuer::new();
    // ... 设置依赖关系
    
    let mut intent = create_test_intent();
    intent.pre_state_root = [0xFF; 32]; // 错误的状态根
    
    let result = issuer.issue_capability(intent, expected_hash);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), VagusError::StateMismatch);
}
```

### 2.3 撤销机制测试

#### 测试用例 2.3.1: 手动撤销
**描述**: 验证手动撤销能力令牌
**输入**: 有效的令牌 ID 和撤销原因
**预期结果**: 令牌被撤销，发出 CapabilityRevoked 事件

```rust
#[test]
fn test_capability_manual_revocation() {
    let mut issuer = CapabilityIssuer::new();
    // ... 设置依赖关系
    
    let intent = create_test_intent();
    let token_id = issuer.issue_capability(intent, expected_hash).unwrap();
    
    // 验证令牌有效
    assert!(issuer.is_valid(token_id));
    
    // 撤销令牌
    issuer.revoke(token_id, 1).unwrap();
    
    // 验证令牌已撤销
    assert!(!issuer.is_valid(token_id));
    
    // 验证事件发出
    let events = issuer.get_events();
    assert!(events.iter().any(|e| matches!(e, Event::CapabilityRevoked(token_id, 1))));
}
```

#### 测试用例 2.3.2: 权限验证
**描述**: 验证只有授权地址可以撤销令牌
**输入**: 非授权地址尝试撤销
**预期结果**: 撤销失败，返回权限错误

```rust
#[test]
fn test_capability_revocation_unauthorized() {
    let mut issuer = CapabilityIssuer::new();
    // ... 设置依赖关系
    
    let intent = create_test_intent();
    let token_id = issuer.issue_capability(intent, expected_hash).unwrap();
    
    let unauthorized = [0x01; 20];
    let result = issuer.revoke_from(unauthorized, token_id, 1);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), VagusError::Unauthorized);
}
```

## 3. VagalBrake 集成测试

### 3.1 动态缩放测试

#### 测试用例 3.1.1: SAFE 状态缩放
**描述**: 验证 SAFE 状态下的 100% 缩放
**输入**: SAFE 状态下的 Intent
**预期结果**: 参数保持 100% 缩放

```rust
#[test]
fn test_vagal_brake_safe_scaling() {
    let mut ans = ANSStateManager::new();
    let mut vagal_brake = VagalBrake::new();
    
    vagal_brake.set_ans_state_manager(ans.address());
    
    // 确保执行器在 SAFE 状态
    let owner = ans.get_owner();
    ans.update_tone_from(owner, 1, 100000).unwrap();
    
    let intent = create_test_intent();
    let (scaled_hash, allowed) = vagal_brake.preview_brake(&intent);
    
    assert!(allowed);
    // 验证缩放因子为 100%
    assert_eq!(extract_scaling_factor(scaled_hash), 10000);
}
```

#### 测试用例 3.1.2: DANGER 状态缩放
**描述**: 验证 DANGER 状态下的 60% 缩放
**输入**: DANGER 状态下的 Intent
**预期结果**: 参数缩放到 60%

```rust
#[test]
fn test_vagal_brake_danger_scaling() {
    let mut ans = ANSStateManager::new();
    let mut vagal_brake = VagalBrake::new();
    
    vagal_brake.set_ans_state_manager(ans.address());
    
    // 转换到 DANGER 状态
    let owner = ans.get_owner();
    for _ in 0..3 {
        ans.update_tone_from(owner, 1, 350000).unwrap();
    }
    
    let intent = create_test_intent();
    let (scaled_hash, allowed) = vagal_brake.preview_brake(&intent);
    
    assert!(allowed);
    // 验证缩放因子为 60%
    assert_eq!(extract_scaling_factor(scaled_hash), 6000);
}
```

#### 测试用例 3.1.3: SHUTDOWN 状态阻止
**描述**: 验证 SHUTDOWN 状态下阻止操作
**输入**: SHUTDOWN 状态下的 Intent
**预期结果**: 操作被阻止，返回 ANSBlocked 错误

```rust
#[test]
fn test_vagal_brake_shutdown_blocking() {
    let mut ans = ANSStateManager::new();
    let mut vagal_brake = VagalBrake::new();
    
    vagal_brake.set_ans_state_manager(ans.address());
    
    // 转换到 SHUTDOWN 状态
    let owner = ans.get_owner();
    for _ in 0..3 {
        ans.update_tone_from(owner, 1, 350000).unwrap();
    }
    for _ in 0..2 {
        ans.update_tone_from(owner, 1, 750000).unwrap();
    }
    
    let intent = create_test_intent();
    let (scaled_hash, allowed) = vagal_brake.preview_brake(&intent);
    
    assert!(!allowed);
    assert_eq!(scaled_hash, [0; 32]);
}
```

## 4. AfferentInbox 集成测试

### 4.1 证据发布测试

#### 测试用例 4.1.1: 有效证据发布
**描述**: 验证有效证据的正确发布
**输入**: 来自授权 attestor 的证据
**预期结果**: 证据被存储，发出 AEPPosted 事件

```rust
#[test]
fn test_afferent_inbox_valid_evidence() {
    let mut inbox = AfferentInbox::new();
    let attestor = [0x01; 20];
    
    // 授权 attestor
    inbox.authorize_attestor(attestor);
    
    let executor_id = 1;
    let state_root = [0x01; 32];
    let metrics_hash = [0x02; 32];
    let signature = vec![0x03; 64];
    
    inbox.post_aep_from(attestor, executor_id, state_root, metrics_hash, signature).unwrap();
    
    // 验证证据存储
    let evidence = inbox.get_latest_evidence(executor_id);
    assert_eq!(evidence.state_root, state_root);
    assert_eq!(evidence.metrics_hash, metrics_hash);
    assert_eq!(evidence.attestor, attestor);
    
    // 验证事件发出
    let events = inbox.get_events();
    assert!(events.iter().any(|e| matches!(e, Event::AEPPosted(executor_id, state_root, metrics_hash))));
}
```

#### 测试用例 4.1.2: 未授权 attestor
**描述**: 验证未授权 attestor 无法发布证据
**输入**: 来自未授权 attestor 的证据
**预期结果**: 发布失败，返回 UnauthorizedAttestor 错误

```rust
#[test]
fn test_afferent_inbox_unauthorized_attestor() {
    let mut inbox = AfferentInbox::new();
    let unauthorized_attestor = [0x01; 20];
    
    let executor_id = 1;
    let state_root = [0x01; 32];
    let metrics_hash = [0x02; 32];
    let signature = vec![0x03; 64];
    
    let result = inbox.post_aep_from(unauthorized_attestor, executor_id, state_root, metrics_hash, signature);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), VagusError::UnauthorizedAttestor);
}
```

## 5. ReflexArc 集成测试

### 5.1 状态变化触发测试

#### 测试用例 5.1.1: DANGER 状态触发
**描述**: 验证 DANGER 状态变化触发反射弧
**输入**: 状态从 SAFE 转换到 DANGER
**预期结果**: 反射弧被触发，相关令牌被撤销

```rust
#[test]
fn test_reflex_arc_danger_trigger() {
    let mut ans = ANSStateManager::new();
    let mut issuer = CapabilityIssuer::new();
    let mut reflex_arc = ReflexArc::new();
    
    // 设置依赖关系
    ans.set_reflex_arc(reflex_arc.address());
    reflex_arc.set_capability_issuer(issuer.address());
    
    // 发行一些能力令牌
    let intent = create_test_intent();
    let token_id = issuer.issue_capability(intent, expected_hash).unwrap();
    
    // 转换到 DANGER 状态
    let owner = ans.get_owner();
    for _ in 0..3 {
        ans.update_tone_from(owner, 1, 350000).unwrap();
    }
    
    // 验证令牌被撤销
    assert!(!issuer.is_valid(token_id));
    
    // 验证事件发出
    let events = reflex_arc.get_events();
    assert!(events.iter().any(|e| matches!(e, Event::ReflexTriggered(1, "state_change", _, _))));
}
```

#### 测试用例 5.1.2: 冷却时间机制
**描述**: 验证反射弧的冷却时间机制
**输入**: 在冷却时间内多次触发
**预期结果**: 只有第一次触发生效

```rust
#[test]
fn test_reflex_arc_cooldown_mechanism() {
    let mut ans = ANSStateManager::new();
    let mut issuer = CapabilityIssuer::new();
    let mut reflex_arc = ReflexArc::new();
    
    // 设置依赖关系
    ans.set_reflex_arc(reflex_arc.address());
    reflex_arc.set_capability_issuer(issuer.address());
    
    // 发行能力令牌
    let intent = create_test_intent();
    let token_id = issuer.issue_capability(intent, expected_hash).unwrap();
    
    // 第一次触发
    for _ in 0..3 {
        ans.update_tone_from(owner, 1, 350000).unwrap();
    }
    
    let initial_events = reflex_arc.get_events().len();
    
    // 立即再次触发 (应该被冷却时间阻止)
    ans.update_tone_from(owner, 1, 400000).unwrap();
    
    let final_events = reflex_arc.get_events().len();
    assert_eq!(initial_events, final_events); // 事件数量不应增加
}
```

## 6. 端到端集成测试

### 6.1 完整工作流测试

#### 测试用例 6.1.1: 完整能力生命周期
**描述**: 验证从意图创建到能力撤销的完整流程
**输入**: 完整的用户操作序列
**预期结果**: 所有步骤正确执行，状态一致

```rust
#[test]
fn test_complete_capability_lifecycle() {
    // 1. 初始化所有组件
    let mut ans = ANSStateManager::new();
    let mut issuer = CapabilityIssuer::new();
    let mut vagal_brake = VagalBrake::new();
    let mut afferent_inbox = AfferentInbox::new();
    let mut reflex_arc = ReflexArc::new();
    
    // 2. 设置依赖关系
    setup_dependencies(&mut ans, &mut issuer, &mut vagal_brake, &mut afferent_inbox, &mut reflex_arc);
    
    // 3. 发布传感器证据
    let attestor = [0x01; 20];
    afferent_inbox.authorize_attestor(attestor);
    afferent_inbox.post_aep_from(attestor, 1, [0x01; 32], [0x02; 32], vec![]).unwrap();
    
    // 4. 创建意图
    let intent = create_test_intent();
    
    // 5. 通过 VagalBrake 发行能力令牌
    let token_id = vagal_brake.issue_with_brake(intent.clone()).unwrap();
    
    // 6. 验证令牌有效
    assert!(issuer.is_valid(token_id));
    
    // 7. 模拟危险情况，触发状态转换
    let owner = ans.get_owner();
    for _ in 0..3 {
        ans.update_tone_from(owner, 1, 350000).unwrap();
    }
    
    // 8. 验证反射弧触发，令牌被撤销
    assert!(!issuer.is_valid(token_id));
    
    // 9. 验证所有相关事件都被发出
    verify_all_events_emitted();
}
```

### 6.2 性能测试

#### 测试用例 6.2.1: 高频更新性能
**描述**: 验证高频 tone 更新的性能
**输入**: 1000 次连续的 tone 更新
**预期结果**: 所有更新在合理时间内完成

```rust
#[test]
fn test_high_frequency_performance() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();
    let executor_id = 1;
    
    let start_time = current_time();
    
    // 执行 1000 次更新
    for i in 0..1000 {
        let tone = 100000 + (i % 500000); // 在安全范围内波动
        ans.update_tone_from(owner, executor_id, tone).unwrap();
    }
    
    let end_time = current_time();
    let duration = end_time - start_time;
    
    // 验证性能要求 (1000 次更新应在 10 秒内完成)
    assert!(duration < 10_000_000_000); // 10 秒 (纳秒)
}
```

#### 测试用例 6.2.2: 多执行器并发性能
**描述**: 验证多执行器并发操作的性能
**输入**: 100 个执行器同时更新
**预期结果**: 所有更新正确完成，状态一致

```rust
#[test]
fn test_multiple_executors_concurrent() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();
    let num_executors = 100;
    
    let start_time = current_time();
    
    // 并发更新多个执行器
    let handles: Vec<_> = (0..num_executors)
        .map(|i| {
            let mut ans_clone = ans.clone();
            std::thread::spawn(move || {
                for j in 0..10 {
                    let tone = 100000 + (i * 1000 + j * 100);
                    ans_clone.update_tone_from(owner, i as u256, tone).unwrap();
                }
            })
        })
        .collect();
    
    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
    
    let end_time = current_time();
    let duration = end_time - start_time;
    
    // 验证性能要求
    assert!(duration < 5_000_000_000); // 5 秒
    
    // 验证所有执行器状态正确
    for i in 0..num_executors {
        let state = ans.get_executor_state(i as u256);
        assert_eq!(state.state, SAFE);
    }
}
```

## 7. 错误处理和边界条件测试

### 7.1 边界值测试

#### 测试用例 7.1.1: 计数器溢出
**描述**: 验证计数器溢出处理
**输入**: 超过 255 次的连续读数
**预期结果**: 计数器正确饱和，不溢出

```rust
#[test]
fn test_counter_overflow() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();
    let executor_id = 1;
    
    // 执行 300 次危险读数 (超过 u8 最大值)
    for _ in 0..300 {
        ans.update_tone_from(owner, executor_id, 350000).unwrap();
    }
    
    let state = ans.get_executor_state(executor_id);
    assert_eq!(state.ctr_danger, 255); // 应该饱和在 255
    assert_eq!(state.state, SAFE); // 仍然需要 3 次才能转换
}
```

#### 测试用例 7.1.2: 时间戳边界
**描述**: 验证时间戳边界条件
**输入**: 时间戳为 0 和最大值
**预期结果**: 正确处理边界值

```rust
#[test]
fn test_timestamp_boundaries() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();
    let executor_id = 1;
    
    // 测试时间戳为 0 的情况
    set_block_timestamp(0);
    ans.update_tone_from(owner, executor_id, 100000).unwrap();
    
    let state = ans.get_executor_state(executor_id);
    assert_eq!(state.updated_at, 0);
    assert_eq!(state.last_transition_at, 0);
    
    // 测试时间戳为最大值的情况
    set_block_timestamp(u64::MAX);
    ans.update_tone_from(owner, executor_id, 200000).unwrap();
    
    let state = ans.get_executor_state(executor_id);
    assert_eq!(state.updated_at, u64::MAX);
}
```

## 8. 回归测试

### 8.1 Solidity 兼容性测试

#### 测试用例 8.1.1: ABI 兼容性
**描述**: 验证原生实现的 ABI 与 Solidity 版本完全兼容
**输入**: 相同的函数调用和参数
**预期结果**: 相同的返回值和事件

```rust
#[test]
fn test_abi_compatibility() {
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
        native_ans.update_tone_from(owner, executor_id, tone).unwrap();
        
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

## 测试执行指南

### 测试环境准备
1. 启动 vagus-chain 测试网络
2. 部署原生合约到预定义地址
3. 准备测试数据和账户
4. 配置测试参数

### 测试执行顺序
1. 单元测试 (每个组件独立测试)
2. 集成测试 (组件间交互测试)
3. 端到端测试 (完整工作流测试)
4. 性能测试 (性能和并发测试)
5. 回归测试 (与 Solidity 版本对比)

### 测试结果验证
1. 所有测试用例必须通过
2. 性能指标必须满足要求
3. 与 Solidity 版本行为完全一致
4. 所有事件正确发出
5. 状态一致性得到保证

### 持续集成
- 每次代码提交自动运行测试套件
- 测试失败阻止代码合并
- 定期运行完整回归测试
- 性能回归检测和告警

## 8. 错误处理和边界条件测试

### 8.1 错误处理测试

#### 测试用例 8.1.1: 网络连接失败
**描述**: 验证网络连接失败时的错误处理
**输入**: 无效的 RPC URL
**预期结果**: 连接错误，适当的错误消息

```rust
#[test]
fn test_network_connection_failure() {
    let result = ANSStateManager::new_with_url("http://invalid-url:1234");
    assert!(result.is_err());
    // 验证错误类型为网络连接错误
}
```

#### 测试用例 8.1.2: 合约调用超时
**描述**: 验证合约调用超时的处理
**输入**: 设置很短的超时时间
**预期结果**: 超时错误，事务回滚

```rust
#[test]
fn test_contract_call_timeout() {
    let mut ans = ANSStateManager::new();
    let owner = ans.get_owner();

    // 设置 1ms 超时
    ans.set_timeout(Duration::from_millis(1));

    // 执行可能耗时的操作
    let result = ans.update_tone_from(owner, 1, 100000);
    assert!(result.is_err());
    // 验证是超时错误
}
```

### 8.2 并发测试

#### 测试用例 8.2.1: 多执行器并发更新
**描述**: 验证多个执行器同时更新的处理
**输入**: 100 个执行器同时更新 tone
**预期结果**: 所有更新成功，状态一致

```rust
#[tokio::test]
async fn test_concurrent_executor_updates() {
    let ans = Arc::new(Mutex::new(ANSStateManager::new()));
    let owner = ans.lock().await.get_owner();

    let mut handles = vec![];

    // 创建 100 个并发任务
    for i in 1..=100 {
        let ans_clone = Arc::clone(&ans);
        let owner_clone = owner;

        let handle = tokio::spawn(async move {
            let mut ans = ans_clone.lock().await;
            ans.update_tone_from(owner_clone, i, 100000).await.unwrap();

            let state = ans.get_executor_state(i).await.unwrap();
            assert_eq!(state.state, SAFE);
            assert_eq!(state.tone, 100000);
        });

        handles.push(handle);
    }

    // 等待所有任务完成
    for handle in handles {
        handle.await.unwrap();
    }
}
```

## 9. 端到端集成测试

### 9.1 完整协议流程测试

#### 测试用例 9.1.1: 完整能力生命周期
**描述**: 验证从 AEP 提交到能力撤销的完整流程
**输入**: 完整的协议交互序列
**预期结果**: 所有步骤成功执行，状态一致

```rust
#[tokio::test]
async fn test_complete_capability_lifecycle() {
    // 初始化所有组件
    let ans = Arc::new(Mutex::new(ANSStateManager::new()));
    let capability_issuer = Arc::new(Mutex::new(CapabilityIssuer::new()));
    let vagal_brake = Arc::new(Mutex::new(VagalBrake::new()));
    let afferent_inbox = Arc::new(Mutex::new(AfferentInbox::new()));

    let owner = ans.lock().await.get_owner();

    // 1. 提交 AEP
    let aep = create_test_aep(1);
    afferent_inbox.lock().await.submit_aep(aep).await.unwrap();

    // 2. 创建 Intent
    let intent = create_test_intent(1);

    // 3. 预览制动效果
    let (scaled_hash, allowed) = vagal_brake.lock().await.preview_brake(&intent).await.unwrap();
    assert!(allowed);

    // 4. 发行能力令牌
    let token_id = capability_issuer.lock().await.issue_capability(intent.clone(), scaled_hash).await.unwrap();

    // 5. 验证令牌有效
    let is_valid = capability_issuer.lock().await.is_valid(token_id).await.unwrap();
    assert!(is_valid);

    // 6. 更新 ANS 状态到 DANGER
    for _ in 0..3 {
        ans.lock().await.update_tone_from(owner, 1, 350000).await.unwrap();
    }

    // 验证 ReflexArc 触发令牌撤销 (需要实际实现 ReflexArc)
    // let is_valid_after = capability_issuer.lock().await.is_valid(token_id).await.unwrap();
    // assert!(!is_valid_after);
}
```

## 10. 回归测试

### 10.1 与 Solidity 版本对比测试

#### 测试用例 10.1.1: 行为一致性验证
**描述**: 验证原生实现与 Solidity 版本的行为完全一致
**输入**: 相同的输入序列
**预期结果**: 相同的输出和状态变化

```rust
#[test]
fn test_behavior_consistency_with_solidity() {
    // 注意: 这需要实际的 Solidity 合约部署来对比
    // 在实际实现中，这里会调用已部署的 Solidity 合约

    let native_ans = ANSStateManager::new();
    let owner = native_ans.get_owner();

    // 执行测试序列
    let test_sequence = vec![
        (1, 100000), (1, 200000), (1, 350000), (1, 360000), (1, 370000),
        (1, 100000), (1, 120000), (1, 140000), (1, 160000), (1, 180000),
    ];

    for (executor_id, tone) in test_sequence {
        native_ans.update_tone(owner, executor_id, tone).unwrap();

        // 在实际测试中，这里会对比 Solidity 版本的结果
        let state = native_ans.get_executor_state(executor_id).unwrap();
        println!("Executor {}: state={}, tone={}, ctr_danger={}, ctr_safe={}",
                executor_id, state.state, state.tone, state.ctr_danger, state.ctr_safe);
    }

    // 验证最终状态符合迟滞性逻辑
    let final_state = native_ans.get_executor_state(1).unwrap();
    assert_eq!(final_state.state, SAFE); // 应该回到 SAFE 状态
    assert_eq!(final_state.tone, 180000);
}
```

## 测试执行和维护指南

### 本地测试环境设置
```bash
# 1. 启动 vagus-chain L1 测试网络
./scripts/start-testnet.sh

# 2. 运行基础集成测试
cargo test --test integration --features native-contracts

# 3. 运行性能测试
cargo test --test performance --features native-contracts

# 4. 运行回归测试
cargo test --test regression --features native-contracts
```

### CI/CD 集成
```yaml
# .github/workflows/integration-tests.yml
name: Integration Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
      - name: Start Test Network
        run: ./scripts/start-testnet.sh
      - name: Run Tests
        run: cargo test --workspace --features native-contracts
      - name: Performance Tests
        run: cargo test --test performance
```

### 测试覆盖率报告
```bash
# 生成覆盖率报告
cargo install cargo-tarpaulin
cargo tarpaulin --workspace --features native-contracts --out Html

# 查看报告
open tarpaulin-report.html
```

### 性能基准测试
```bash
# 运行基准测试
cargo bench --features native-contracts

# 对比不同实现的性能
./scripts/benchmark-comparison.sh
```

## 故障排除

### 常见测试失败原因

1. **网络连接问题**
   - 确保 vagus-chain L1 测试网络正在运行
   - 检查 RPC URL 配置
   - 验证网络延迟

2. **状态不一致**
   - 检查时钟同步
   - 验证配置参数
   - 对比事件日志

3. **性能问题**
   - 检查系统资源
   - 验证并发设置
   - 监控内存使用

4. **事件丢失**
   - 检查事件订阅
   - 验证事件过滤器
   - 确认异步处理

### 调试技巧

```rust
// 启用详细日志
std::env::set_var("RUST_LOG", "vagus_chain=debug");
tracing_subscriber::fmt::init();

// 添加断点
#[cfg(debug_assertions)]
{
    println!("Debug: executor_id={}, tone={}", executor_id, tone);
    println!("Debug: current_state={:?}", state);
}
```

这个集成测试套件提供了全面的测试覆盖，确保 vagus-chain 原生实现的质量和可靠性。
