# Vagus Native Interface Specification

## 概述

本文档定义了将 Vagus 核心 Solidity 合约迁移到 vagus-chain L1 原生协议所需的接口规范。这些接口将作为 L1 团队实现原生模块的精确规范。

## 核心合约地址映射

### 预定义地址
以下地址将在 vagus-chain L1 中预定义，作为原生协议的一部分：

```
ANSStateManager:     0x0000000000000000000000000000000000000001
CapabilityIssuer:    0x0000000000000000000000000000000000000002
VagalBrake:          0x0000000000000000000000000000000000000003
AfferentInbox:       0x0000000000000000000000000000000000000004
ReflexArc:           0x0000000000000000000000000000000000000005
```

## 1. ANSStateManager 原生接口

### 预编译地址
```
Address: 0x0000000000000000000000000000000000000001
```

### 状态模型

#### HysteresisConfig 结构
```rust
pub struct HysteresisConfig {
    pub danger_enter_tone: u32,     // ppm (进入 DANGER 阈值)
    pub safe_exit_tone: u32,        // ppm (退出 DANGER 到 SAFE，> danger_enter_tone)
    pub shutdown_enter_tone: u32,   // ppm (进入 SHUTDOWN 阈值)
    pub n_danger_enter: u8,         // 进入 DANGER 的连续计数
    pub n_safe_exit: u8,            // 退出 DANGER 的连续计数
    pub n_shutdown_enter: u8,       // 进入 SHUTDOWN 的连续计数
    pub dwell_min_sec: u32,         // 状态转换之间的最小停留时间
}
```

#### ExecutorState 结构
```rust
pub struct ExecutorState {
    pub state: u8,                  // 0 SAFE, 1 DANGER, 2 SHUTDOWN
    pub tone: u32,                  // 当前 tone 值 (ppm)
    pub updated_at: u64,            // 最后更新时间戳
    pub last_transition_at: u64,    // 最后状态转换时间戳
    pub ctr_danger: u8,             // 连续 danger 进入计数器
    pub ctr_safe: u8,               // 连续 safe 退出计数器
    pub ctr_shutdown: u8,           // 连续 shutdown 进入计数器
}
```

### 函数接口

#### updateTone
**函数签名**: `updateTone(uint256 executorId, uint32 tone)`
**函数选择器**: `0xabab5daf`
**输入编码**:
- `executorId: uint256` - 32 bytes (offset: 0x04-0x24)
- `tone: uint32` - 4 bytes (offset: 0x24-0x28)

**输出**: 无返回值，发出 `VagalToneUpdated` 事件

**权限**: 仅限 owner 调用

#### guardFor
**函数签名**: `guardFor(uint256 executorId, bytes32 actionId)`
**函数选择器**: `0xc42e8a0b`
**输入编码**:
- `executorId: uint256` - 32 bytes (offset: 0x04-0x24)
- `actionId: bytes32` - 32 bytes (offset: 0x24-0x44)

**输出编码**:
- `scalingFactor: uint256` - 32 bytes (offset: 0x00-0x20)
- `allowed: bool` - 1 byte (offset: 0x20-0x21)

#### getExecutorState
**函数签名**: `getExecutorState(uint256 executorId)`
**函数选择器**: `0x74323ce8`
**输入编码**:
- `executorId: uint256` - 32 bytes (offset: 0x04-0x24)

**输出编码**:
- `state: uint8` - 1 byte (offset: 0x00-0x01)
- `tone: uint32` - 4 bytes (offset: 0x20-0x24)
- `updatedAt: uint64` - 8 bytes (offset: 0x40-0x48)

### 事件规范

#### VagalToneUpdated
**事件签名**: `VagalToneUpdated(uint256 indexed tone, uint8 indexed state, uint256 updatedAt)`
**事件主题哈希**: `0xab0867b13e7cff2521b3e4b31e3351d0dafd16c152680a15a4c68f0c587fbb35`
**数据编码**:
- `tone` (indexed): 32 bytes - topic[1]
- `state` (indexed): 1 byte (padded to 32 bytes) - topic[2]
- `updatedAt`: 32 bytes (offset: 0x00-0x20)

## 2. CapabilityIssuer 原生接口

### 预编译地址
```
Address: 0x0000000000000000000000000000000000000002
```

### 状态模型

#### TokenMeta 结构
```rust
pub struct TokenMeta {
    pub executor_id: u256,
    pub action_id: [u8; 32],
    pub scaled_limits_hash: [u8; 32],
    pub issued_at: u64,
    pub expires_at: u64,
    pub revoked: bool,
    pub issuer: [u8; 20],
}
```

#### Intent 结构
```rust
pub struct Intent {
    pub executor_id: u256,
    pub action_id: [u8; 32],
    pub params: Vec<u8>,
    pub envelope_hash: [u8; 32],
    pub pre_state_root: [u8; 32],
    pub not_before: u64,
    pub not_after: u64,
    pub max_duration_ms: u32,
    pub max_energy_j: u32,
    pub planner: [u8; 20],
    pub nonce: u256,
}
```

### 函数接口

#### issueCapability
**函数签名**: `issueCapability((uint256,uint256,bytes,bytes32,bytes32,uint64,uint64,uint32,uint32,address,uint256),bytes32)`
**函数选择器**: `0xe93b46ed`
**输入编码**:
- `intent` (tuple): 动态编码 (offset: 0x04-0x24)
  - `executorId: uint256` - 32 bytes
  - `actionId: bytes32` - 32 bytes
  - `params: bytes` - 动态
  - `envelopeHash: bytes32` - 32 bytes
  - `preStateRoot: bytes32` - 32 bytes
  - `notBefore: uint64` - 8 bytes
  - `notAfter: uint64` - 8 bytes
  - `maxDurationMs: uint32` - 4 bytes
  - `maxEnergyJ: uint32` - 4 bytes
  - `planner: address` - 20 bytes
  - `nonce: uint256` - 32 bytes
- `scaledLimitsHash: bytes32` - 32 bytes

**输出编码**:
- `tokenId: uint256` - 32 bytes (offset: 0x00-0x20)

**权限**: 公开调用，内部验证

#### revoke
**函数签名**: `revoke(uint256 tokenId, uint8 reason)`
**函数选择器**: `0x14f6b1fb`
**输入编码**:
- `tokenId: uint256` - 32 bytes (offset: 0x04-0x24)
- `reason: uint8` - 1 byte (offset: 0x24-0x25)

**输出**: 无返回值，发出 `CapabilityRevoked` 事件

**权限**: 仅限 owner 或 reflexArc

#### isValid
**函数签名**: `isValid(uint256 tokenId)`
**函数选择器**: `0xf577a500`
**输入编码**:
- `tokenId: uint256` - 32 bytes (offset: 0x04-0x24)

**输出编码**:
- `valid: bool` - 1 byte (offset: 0x00-0x01)

#### activeTokensOf
**函数签名**: `activeTokensOf(uint256 executorId)`
**函数选择器**: `0x0e760237`
**输入编码**:
- `executorId: uint256` - 32 bytes (offset: 0x04-0x24)

**输出编码**:
- `tokenIds: uint256[]` - 动态数组 (offset: 0x00-0x20)

### 事件规范

#### CapabilityIssued
**事件签名**: `CapabilityIssued(uint256 indexed tokenId, uint256 indexed executorId, address indexed planner, bytes32 actionId, uint256 expiresAt, bytes32 paramsHashSha256, bytes32 paramsHashKeccak, bytes32 preStateRootSha256, bytes32 preStateRootKeccak)`
**事件主题哈希**: `0x1297a386d439f0ecb7e6da68d81b7b3df0ceab2269a95f78adf8030b6be05106`
**数据编码**:
- `tokenId` (indexed): 32 bytes - topic[1]
- `executorId` (indexed): 32 bytes - topic[2]
- `planner` (indexed): 20 bytes (padded to 32 bytes) - topic[3]
- `actionId`: 32 bytes (offset: 0x00-0x20)
- `expiresAt`: 32 bytes (offset: 0x20-0x40)
- `paramsHashSha256`: 32 bytes (offset: 0x40-0x60)
- `paramsHashKeccak`: 32 bytes (offset: 0x60-0x80)
- `preStateRootSha256`: 32 bytes (offset: 0x80-0xA0)
- `preStateRootKeccak`: 32 bytes (offset: 0xA0-0xC0)

#### CapabilityRevoked
**事件签名**: `CapabilityRevoked(uint256 indexed tokenId, uint8 reason)`
**事件主题哈希**: `0x9c3b727b72e2688a4cd4d6ebcf75a21ddc5b40348c97030a21013dcf5aa8347c`
**数据编码**:
- `tokenId` (indexed): 32 bytes - topic[1]
- `reason`: 1 byte (offset: 0x00-0x01)

## 3. VagalBrake 原生接口

### 预编译地址
```
Address: 0x0000000000000000000000000000000000000003
```

### 函数接口

#### issueWithBrake
**ABI 编码**: `0x67890123` (函数选择器)
**输入参数**:
- `intent: Intent` (动态结构体)

**输出**:
- `tokenId: uint256` (32 bytes)

#### previewBrake
**ABI 编码**: `0x78901234` (函数选择器)
**输入参数**:
- `intent: Intent` (动态结构体)

**输出**:
- `scaledLimitsHash: bytes32` (32 bytes)
- `allowed: bool` (1 byte)

## 4. AfferentInbox 原生接口

### 预编译地址
```
Address: 0x0000000000000000000000000000000000000004
```

### 状态模型

#### Evidence 结构
```rust
pub struct Evidence {
    pub state_root: [u8; 32],
    pub metrics_hash: [u8; 32],
    pub timestamp: u64,
    pub attestor: [u8; 20],
}
```

### 函数接口

#### postAEP
**ABI 编码**: `0x89012345` (函数选择器)
**输入参数**:
- `executorId: uint256` (32 bytes)
- `stateRoot: bytes32` (32 bytes)
- `metricsHash: bytes32` (32 bytes)
- `signature: bytes` (动态字节数组)

**输出**: 无返回值，发出 `AEPPosted` 事件

**权限**: 仅限授权 attestor

#### latestStateRoot
**ABI 编码**: `0x90123456` (函数选择器)
**输入参数**:
- `executorId: uint256` (32 bytes)

**输出**:
- `stateRoot: bytes32` (32 bytes)

### 事件规范

#### AEPPosted
**事件签名**: `AEPPosted(uint256 indexed executorId, bytes32 stateRoot, bytes32 metricsHash)`

## 5. ReflexArc 原生接口

### 预编译地址
```
Address: 0x0000000000000000000000000000000000000005
```

### 函数接口

#### on_state_change
**ABI 编码**: `0xa0123456` (函数选择器)
**输入参数**:
- `executorId: uint256` (32 bytes)
- `newState: uint8` (1 byte)

**输出**: 无返回值，可能发出 `ReflexTriggered` 事件

**权限**: 仅限 ANSStateManager

#### on_aep
**ABI 编码**: `0xb1234567` (函数选择器)
**输入参数**:
- `executorId: uint256` (32 bytes)

**输出**: 无返回值，可能发出 `ReflexTriggered` 事件

**权限**: 仅限 AfferentInbox

### 事件规范

#### ReflexTriggered
**事件签名**: `ReflexTriggered(uint256 indexed executorId, string reason, uint256 revokedCount, uint256 triggeredAt)`

## 存储模型

### Merkle Trie 键结构

#### ANSStateManager 存储
```
键: "ans_state_manager:executor:{executorId}"
值: ExecutorState (序列化)
```

#### CapabilityIssuer 存储
```
键: "capability_issuer:token:{tokenId}"
值: TokenMeta (序列化)

键: "capability_issuer:active:{executorId}"
值: uint256[] (序列化)
```

#### AfferentInbox 存储
```
键: "afferent_inbox:evidence:{executorId}"
值: Evidence (序列化)
```

## 错误处理

### 自定义错误码
```rust
pub enum VagusError {
    Unauthorized = 1,
    InvalidInput = 2,
    StateMismatch = 3,
    ANSBlocked = 4,
    IntentExpired = 5,
    CircuitBreakerOpen = 6,
    RateLimited = 7,
    UnauthorizedAttestor = 8,
    InvalidSignature = 9,
}
```

## 实现要求

### 性能要求
- 所有函数调用应在 100ms 内完成
- 状态查询应在 10ms 内完成
- 事件发出不应阻塞函数执行

### 安全要求
- 所有输入参数必须验证
- 权限检查必须在函数开始时进行
- 状态转换必须原子性执行

### 兼容性要求
- ABI 编码必须与 Solidity 版本完全兼容
- 事件签名必须与现有客户端代码兼容
- 状态模型必须支持现有数据结构

## 测试验证

### 单元测试
每个原生函数都必须有对应的单元测试，验证：
- 正常情况下的行为
- 边界条件处理
- 错误情况处理
- 权限验证

### 集成测试
必须验证：
- 合约间交互
- 事件发出
- 状态一致性
- 性能要求

### 回归测试
必须确保原生实现与 Solidity 版本行为完全一致。
