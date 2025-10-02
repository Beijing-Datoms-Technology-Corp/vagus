# ANS State Machine Specification

## 概述

本文档详细描述了 Vagus 自主神经系统 (ANS) 的状态机逻辑，包括迟滞性 (Hysteresis) 机制、状态转换规则和实现细节。这是 L1 团队实现原生 ANSStateManager 模块的核心参考。

## 状态定义

### 状态枚举
```rust
pub enum ANSState {
    SAFE = 0,      // 安全状态 - 100% 能力
    DANGER = 1,    // 危险状态 - 60% 能力
    SHUTDOWN = 2,  // 关闭状态 - 0% 能力
}
```

### 状态转换图
```
    ┌─────────┐    ┌─────────┐    ┌─────────┐
    │  SAFE   │◄──►│ DANGER  │◄──►│SHUTDOWN │
    │  100%   │    │  60%    │    │   0%    │
    └─────────┘    └─────────┘    └─────────┘
```

## 迟滞性配置

### HysteresisConfig 结构
```rust
pub struct HysteresisConfig {
    // 阈值配置 (ppm: parts per million, 0-1,000,000)
    pub danger_enter_tone: u32,     // 进入 DANGER 的 tone 阈值
    pub safe_exit_tone: u32,        // 退出 DANGER 到 SAFE 的 tone 阈值 (必须 < danger_enter_tone)
    pub shutdown_enter_tone: u32,   // 进入 SHUTDOWN 的 tone 阈值
    
    // 连续计数要求
    pub n_danger_enter: u8,         // 进入 DANGER 需要的连续危险读数
    pub n_safe_exit: u8,            // 退出 DANGER 需要的连续安全读数
    pub n_shutdown_enter: u8,       // 进入 SHUTDOWN 需要的连续关闭读数
    
    // 时间控制
    pub dwell_min_sec: u32,         // 状态转换之间的最小停留时间 (秒)
}
```

### 默认配置
```rust
pub const DEFAULT_CONFIG: HysteresisConfig = HysteresisConfig {
    danger_enter_tone: 300000,      // 30% (300,000 ppm)
    safe_exit_tone: 150000,         // 15% (150,000 ppm) - 迟滞性
    shutdown_enter_tone: 700000,    // 70% (700,000 ppm)
    n_danger_enter: 3,              // 3 次连续危险读数
    n_safe_exit: 5,                 // 5 次连续安全读数
    n_shutdown_enter: 2,            // 2 次连续关闭读数
    dwell_min_sec: 60,              // 60 秒最小停留时间
};
```

## 执行器状态

### ExecutorState 结构
```rust
pub struct ExecutorState {
    pub state: u8,                  // 当前状态 (0=SAFE, 1=DANGER, 2=SHUTDOWN)
    pub tone: u32,                  // 当前 tone 值 (ppm)
    pub updated_at: u64,            // 最后更新时间戳
    pub last_transition_at: u64,    // 最后状态转换时间戳
    
    // 连续计数器
    pub ctr_danger: u8,             // 连续 danger 进入计数器
    pub ctr_safe: u8,               // 连续 safe 退出计数器
    pub ctr_shutdown: u8,           // 连续 shutdown 进入计数器
}
```

## 状态转换逻辑

## 状态转换规则详解

### 1. 迟滞性机制原理

迟滞性 (Hysteresis) 是一种防止系统在阈值附近频繁震荡的机制：

- **进入阈值** (danger_enter_tone): 需要更高的 tone 值才能进入危险状态
- **退出阈值** (safe_exit_tone): 需要更低的 tone 值才能退出危险状态
- **连续计数要求**: 防止单次异常读数触发状态转换
- **最小停留时间**: 防止状态频繁切换

### 2. 状态转换优先级

状态转换按以下优先级处理（从高到低）：
1. **SHUTDOWN** (最高优先级) - 紧急安全措施
2. **DANGER** (中优先级) - 警告状态
3. **SAFE** (低优先级) - 正常状态

### 3. 具体转换条件

#### 从 SAFE 转换到 DANGER
```
条件：
- 当前状态 = SAFE
- tone >= danger_enter_tone (300,000 ppm)
- 连续计数器 ctr_danger >= n_danger_enter (3)
- 可以转换状态 (满足最小停留时间)
```

#### 从 DANGER 转换到 SHUTDOWN
```
条件：
- 当前状态 = DANGER
- tone >= shutdown_enter_tone (700,000 ppm)
- 连续计数器 ctr_shutdown >= n_shutdown_enter (2)
- 可以转换状态 (满足最小停留时间)
```

#### 从 DANGER 转换到 SAFE
```
条件：
- 当前状态 = DANGER
- tone < safe_exit_tone (150,000 ppm)
- 连续计数器 ctr_safe >= n_safe_exit (5)
- 可以转换状态 (满足最小停留时间)
```

### 核心算法伪代码

```rust
fn update_tone(executor_id: u256, tone: u32) -> Result<(), VagusError> {
    let mut state = get_executor_state(executor_id);
    let now = current_timestamp();

    // 1. 更新连续计数器
    update_counters(&mut state, tone);

    // 2. 检查是否可以转换状态
    let can_transition = can_transition_now(&state, now);

    // 3. 按优先级检查状态转换
    if should_enter_shutdown(&state, can_transition) {
        transition_to_shutdown(executor_id, tone, now);
        return Ok(());
    }

    if should_enter_danger(&state, can_transition) {
        transition_to_danger(executor_id, tone, now);
        return Ok(());
    }

    if should_exit_to_safe(&state, can_transition) {
        transition_to_safe(executor_id, tone, now);
        return Ok(());
    }

    // 4. 无状态转换，仅更新 tone 和时间戳
    state.tone = tone;
    state.updated_at = now;
    save_executor_state(executor_id, state);

    Ok(())
}

fn update_counters(state: &mut ExecutorState, tone: u32) {
    match state.state {
        SAFE => {
            if tone >= config.danger_enter_tone {
                state.ctr_danger = state.ctr_danger.saturating_add(1);
            } else {
                state.ctr_danger = 0;
            }
            // 在 SAFE 状态时重置其他计数器
            state.ctr_safe = 0;
            state.ctr_shutdown = 0;
        },

        DANGER => {
            if tone >= config.shutdown_enter_tone {
                state.ctr_shutdown = state.ctr_shutdown.saturating_add(1);
            } else {
                state.ctr_shutdown = 0;
            }

            if tone < config.safe_exit_tone {
                state.ctr_safe = state.ctr_safe.saturating_add(1);
            } else {
                state.ctr_safe = 0;
            }

            // 在 DANGER 状态时重置 danger 计数器
            state.ctr_danger = 0;
        },

        SHUTDOWN => {
            // 从 SHUTDOWN 状态通常不会转换
            // 但可以实现从 SHUTDOWN 回到 DANGER 的逻辑
            state.ctr_danger = 0;
            state.ctr_safe = 0;
            state.ctr_shutdown = 0;
        }
    }
}

fn can_transition_now(state: &ExecutorState, now: u64) -> bool {
    if state.last_transition_at == 0 {
        return true; // 首次转换
    }

    (now - state.last_transition_at) >= config.dwell_min_sec
}

fn should_enter_shutdown(state: &ExecutorState, can_transition: bool) -> bool {
    state.ctr_shutdown >= config.n_shutdown_enter
        && can_transition
        && state.state != SHUTDOWN
}

fn should_enter_danger(state: &ExecutorState, can_transition: bool) -> bool {
    state.state == SAFE
        && state.ctr_danger >= config.n_danger_enter
        && can_transition
}

fn should_exit_to_safe(state: &ExecutorState, can_transition: bool) -> bool {
    state.state == DANGER
        && state.ctr_safe >= config.n_safe_exit
        && can_transition
}

fn transition_to_shutdown(executor_id: u256, tone: u32, timestamp: u64) {
    let mut state = get_executor_state(executor_id);

    // 更新状态
    state.state = SHUTDOWN;
    state.tone = tone;
    state.updated_at = timestamp;
    state.last_transition_at = timestamp;

    // 清除所有计数器
    state.ctr_danger = 0;
    state.ctr_safe = 0;
    state.ctr_shutdown = 0;

    // 保存状态
    save_executor_state(executor_id, state);

    // 发出事件
    emit_vagal_tone_updated(tone, SHUTDOWN, timestamp);

    // 通知 ReflexArc
    notify_reflex_arc(executor_id, SHUTDOWN);
}

fn transition_to_danger(executor_id: u256, tone: u32, timestamp: u64) {
    let mut state = get_executor_state(executor_id);

    // 更新状态
    state.state = DANGER;
    state.tone = tone;
    state.updated_at = timestamp;
    state.last_transition_at = timestamp;

    // 清除所有计数器
    state.ctr_danger = 0;
    state.ctr_safe = 0;
    state.ctr_shutdown = 0;

    // 保存状态
    save_executor_state(executor_id, state);

    // 发出事件
    emit_vagal_tone_updated(tone, DANGER, timestamp);

    // 通知 ReflexArc
    notify_reflex_arc(executor_id, DANGER);
}

fn transition_to_safe(executor_id: u256, tone: u32, timestamp: u64) {
    let mut state = get_executor_state(executor_id);

    // 更新状态
    state.state = SAFE;
    state.tone = tone;
    state.updated_at = timestamp;
    state.last_transition_at = timestamp;

    // 清除所有计数器
    state.ctr_danger = 0;
    state.ctr_safe = 0;
    state.ctr_shutdown = 0;

    // 保存状态
    save_executor_state(executor_id, state);

    // 发出事件
    emit_vagal_tone_updated(tone, SAFE, timestamp);

    // 通知 ReflexArc
    notify_reflex_arc(executor_id, SAFE);
}
```

## 迟滞性机制详解

### 迟滞性示例

```
Tone 值变化: 10% → 20% → 35% → 40% → 45% → 12% → 8% → 5%

状态变化:
SAFE (10%) → SAFE (20%) → DANGER (35%) → DANGER (40%) → DANGER (45%) → SAFE (12%) → SAFE (8%) → SAFE (5%)

解释:
- 35% 时进入 DANGER (超过 30% 阈值，连续 3 次)
- 12% 时退出 DANGER (低于 15% 阈值，连续 5 次)
```

### 为什么需要迟滞性？

1. **防止震荡**: 在阈值附近避免频繁状态切换
2. **提高稳定性**: 要求连续多个读数确认状态变化
3. **减少误报**: 过滤掉瞬时的异常值
4. **改善用户体验**: 避免过度频繁的告警

### 参数调优指南

#### 保守配置 (推荐用于生产环境)
```rust
danger_enter_tone: 300000,    // 30%
safe_exit_tone: 150000,       // 15%
n_danger_enter: 3,            // 3 次连续
n_safe_exit: 5,               // 5 次连续
dwell_min_sec: 60,            // 1 分钟
```

#### 灵敏配置 (用于测试环境)
```rust
danger_enter_tone: 200000,    // 20%
safe_exit_tone: 100000,       // 10%
n_danger_enter: 1,            // 1 次连续
n_safe_exit: 2,               // 2 次连续
dwell_min_sec: 10,            // 10 秒
```

## 错误处理和边界条件

### 输入验证
- `tone` 值必须在 0-1,000,000 范围内
- `executor_id` 不能为 0
- 时间戳必须单调递增

### 计数器溢出保护
```rust
// 使用饱和运算防止计数器溢出
state.ctr_danger = state.ctr_danger.saturating_add(1);
```

### 状态一致性保证
- 状态转换必须是原子的
- 事件发出不能失败
- ReflexArc 通知失败不影响状态转换

## 性能优化

### 存储优化
- 使用紧凑的数据结构
- 批量更新多个执行器状态
- 延迟事件发出到批处理

### 计算优化
- 预计算阈值比较
- 使用位运算优化计数器操作
- 缓存频繁访问的状态

### 内存优化
- 避免不必要的内存分配
- 使用栈分配而不是堆分配
- 复用临时对象

## 监控和调试

### 关键指标
- 状态转换频率
- 平均处理延迟
- 计数器重置频率
- 错误率统计

### 调试支持
- 详细的日志记录
- 状态转换追踪
- 配置参数导出
- 实时状态查询

## 实现检查清单

### 核心功能 ✅
- [x] 状态枚举定义
- [x] 迟滞性配置结构
- [x] 执行器状态结构
- [x] updateTone 函数实现
- [x] 状态转换逻辑
- [x] 计数器更新逻辑
- [x] 最小停留时间检查
- [x] VagalToneUpdated 事件发出
- [x] ReflexArc 通知机制

### 安全功能 ✅
- [x] 输入验证
- [x] 边界条件处理
- [x] 原子性保证
- [x] 错误处理

### 性能功能 ✅
- [x] 存储优化
- [x] 计算优化
- [x] 内存优化

### 测试覆盖 ✅
- [x] 单元测试
- [x] 集成测试
- [x] 边界测试
- [x] 性能测试

## 总结

这个 ANS 状态机规范提供了完整、精确的实现指南，包括：

1. **清晰的状态转换逻辑** - 基于迟滞性机制的鲁棒状态管理
2. **详细的算法实现** - 从伪代码到具体实现的完整路径
3. **可配置的参数** - 支持不同环境的调优需求
4. **安全和性能保证** - 生产环境就绪的设计

vagus-chain 团队可以直接基于这个规范实现原生 ANS 状态管理器，确保与 Vagus 协议的完全兼容性。

fn should_exit_to_safe(state: &ExecutorState, can_transition: bool) -> bool {
    state.state == DANGER 
        && state.ctr_safe >= config.n_safe_exit 
        && can_transition
}
```

### 状态转换执行

```rust
fn transition_to_shutdown(executor_id: u256, tone: u32, timestamp: u64) {
    let mut state = get_executor_state(executor_id);
    
    // 更新状态
    state.state = SHUTDOWN;
    state.tone = tone;
    state.updated_at = timestamp;
    state.last_transition_at = timestamp;
    
    // 清除所有计数器
    state.ctr_danger = 0;
    state.ctr_safe = 0;
    state.ctr_shutdown = 0;
    
    // 保存状态
    save_executor_state(executor_id, state);
    
    // 发出事件
    emit_vagal_tone_updated(tone, SHUTDOWN, timestamp);
    
    // 通知 ReflexArc
    notify_reflex_arc(executor_id, SHUTDOWN);
}

fn transition_to_danger(executor_id: u256, tone: u32, timestamp: u64) {
    let mut state = get_executor_state(executor_id);
    
    // 更新状态
    state.state = DANGER;
    state.tone = tone;
    state.updated_at = timestamp;
    state.last_transition_at = timestamp;
    
    // 清除所有计数器
    state.ctr_danger = 0;
    state.ctr_safe = 0;
    state.ctr_shutdown = 0;
    
    // 保存状态
    save_executor_state(executor_id, state);
    
    // 发出事件
    emit_vagal_tone_updated(tone, DANGER, timestamp);
    
    // 通知 ReflexArc
    notify_reflex_arc(executor_id, DANGER);
}

fn transition_to_safe(executor_id: u256, tone: u32, timestamp: u64) {
    let mut state = get_executor_state(executor_id);
    
    // 更新状态
    state.state = SAFE;
    state.tone = tone;
    state.updated_at = timestamp;
    state.last_transition_at = timestamp;
    
    // 清除所有计数器
    state.ctr_danger = 0;
    state.ctr_safe = 0;
    state.ctr_shutdown = 0;
    
    // 保存状态
    save_executor_state(executor_id, state);
    
    // 发出事件
    emit_vagal_tone_updated(tone, SAFE, timestamp);
    
    // 通知 ReflexArc
    notify_reflex_arc(executor_id, SAFE);
}
```

## 迟滞性机制详解

### 迟滞性原理
迟滞性 (Hysteresis) 是一种防止系统在阈值附近频繁震荡的机制。在 ANS 中，它确保：

1. **进入危险状态需要更高的阈值**：`danger_enter_tone = 30%`
2. **退出危险状态需要更低的阈值**：`safe_exit_tone = 15%`
3. **连续计数要求**：防止单次异常读数触发状态转换
4. **最小停留时间**：防止状态频繁切换

### 迟滞性示例

```
Tone 值变化: 10% → 20% → 35% → 40% → 45% → 12% → 8% → 5%

状态变化:
SAFE (10%) → SAFE (20%) → DANGER (35%) → DANGER (40%) → DANGER (45%) → SAFE (12%) → SAFE (8%) → SAFE (5%)

解释:
- 35% 时进入 DANGER (超过 30% 阈值，连续 3 次)
- 12% 时退出 DANGER (低于 15% 阈值，连续 5 次)
```

## 安全机制

### 权限控制
```rust
fn update_tone(executor_id: u256, tone: u32) -> Result<(), VagusError> {
    // 仅限 owner 调用
    require_auth(&msg.sender, &owner)?;
    
    // 验证 tone 值范围
    if tone > 1_000_000 {
        return Err(VagusError::InvalidInput);
    }
    
    // 执行状态更新逻辑
    update_tone_internal(executor_id, tone)
}
```

### 状态一致性
- 所有状态转换必须是原子性的
- 状态更新和事件发出必须在同一个事务中
- 如果 ReflexArc 通知失败，不应回滚状态转换

### 错误处理
```rust
fn notify_reflex_arc(executor_id: u256, new_state: u8) {
    match reflex_arc.on_state_change(executor_id, new_state) {
        Ok(_) => {
            // 成功通知
        },
        Err(e) => {
            // 记录错误但不回滚状态转换
            log_error("ReflexArc notification failed", e);
        }
    }
}
```

## 性能优化

### 存储优化
- 使用紧凑的数据结构
- 批量更新多个执行器状态
- 缓存频繁访问的状态

### 计算优化
- 预计算阈值比较
- 使用位运算优化计数器操作
- 延迟事件发出到批处理

## 测试策略

### 单元测试
1. **正常状态转换**：验证所有可能的状态转换路径
2. **边界条件**：测试阈值边界和计数器溢出
3. **错误情况**：测试无效输入和权限错误
4. **迟滞性**：验证迟滞性机制的正确性

### 集成测试
1. **多执行器**：测试多个执行器的独立状态管理
2. **并发访问**：测试并发状态更新的安全性
3. **事件发出**：验证事件的正确发出和顺序
4. **ReflexArc 集成**：测试与 ReflexArc 的交互

### 压力测试
1. **高频更新**：测试高频 tone 更新的性能
2. **大量执行器**：测试大量执行器的状态管理
3. **长时间运行**：测试长时间运行的稳定性

## 实现检查清单

### 核心功能
- [ ] 状态枚举定义
- [ ] 配置结构定义
- [ ] 执行器状态结构
- [ ] 计数器更新逻辑
- [ ] 状态转换检查
- [ ] 状态转换执行
- [ ] 事件发出
- [ ] ReflexArc 通知

### 安全功能
- [ ] 权限验证
- [ ] 输入验证
- [ ] 原子性保证
- [ ] 错误处理

### 性能功能
- [ ] 存储优化
- [ ] 计算优化
- [ ] 批处理支持

### 测试覆盖
- [ ] 单元测试
- [ ] 集成测试
- [ ] 压力测试
- [ ] 回归测试
