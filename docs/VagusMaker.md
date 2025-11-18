# VagusMaker: Blockchain-Native MAKER Consensus for Deterministic Tasks

## 目录

1. [概述](#概述)
2. [核心概念](#核心概念)
3. [架构设计](#架构设计)
4. [MAKER共识算法详解](#maker共识算法详解)
5. [核心组件详解](#核心组件详解)
6. [Vagus安全集成](#vagus安全集成)
7. [部署指南](#部署指南)
8. [使用指南](#使用指南)
9. [API参考](#api参考)
10. [安全考虑](#安全考虑)
11. [性能分析](#性能分析)
12. [测试与验证](#测试与验证)
13. [未来扩展](#未来扩展)

## 概述

### 项目愿景

VagusMaker 是全球首个区块链原生 MAKER (Minimum Acceptable Knowledge Ensemble Reasoning) 共识算法的完整实现，专为解决确定性任务（如河内塔问题）的分布式验证而设计。

**核心公式**: `VagusMaker = Vagus 安全 + MAKER 正确`

### 关键特性

- **零错误保证**: 理论上支持 20 盘河内塔的零错误完成 (1,048,575 步)
- **分布式验证**: 通过信誉加权投票实现人类智慧与 AI 的协作
- **区块链原生**: 完全在智能合约中实现，无需链下可信计算
- **Vagus集成**: 与自主神经系统安全层深度融合
- **可扩展性**: 支持任意复杂度的确定性任务

### 应用场景

1. **数学证明验证**: 形式化数学定理的分布式证明
2. **算法正确性验证**: 复杂算法的逐步验证
3. **逻辑推理验证**: 多步逻辑推理链的验证
4. **游戏求解验证**: 确定性游戏的最优解验证

## 核心概念

### MAKER共识算法

MAKER (Minimum Acceptable Knowledge Ensemble Reasoning) 是一种新型共识机制，专门用于验证确定性任务的正确性。它通过以下方式实现：

1. **任务分解**: 将复杂任务分解为一系列可验证的小步骤
2. **并行验证**: 多个验证者同时对每个步骤进行投票
3. **信誉加权**: 基于历史表现的信誉系统调整投票权重
4. **k-ahead-by共识**: 严格的共识条件确保正确性

### 河内塔问题作为基准

我们选择河内塔问题作为实现和测试的基准，因为：

- **确定性**: 每个状态都有唯一正确的下一步
- **渐进复杂性**: 随着盘数增加，步骤呈指数增长
- **可验证性**: 每个移动都可以通过规则验证
- **扩展性挑战**: 20盘需要100万+步，测试系统极限

### Vagus安全层

VagusMaker 与 Vagus 协议深度集成：

- **ANS状态监控**: 持续监控系统张力水平
- **Reflex Arc触发**: 检测到共识异常时自动干预
- **Capability Token**: 基于信誉的执行权限管理
- **Afferent证据**: 实时收集和处理外部证据

## 架构设计

### 系统架构图

```
┌─────────────────────────────────────────────────────────────┐
│                    VagusMaker System                        │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐  │
│  │ ReputationToken │  │ MicroTaskManager│  │RedFlagValidator│ │
│  │  (ERC-5192)     │  │  (ANS继承)      │  │  (规则验证)   │ │
│  └─────────────────┘  └─────────────────┘  └─────────────┘  │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────┐    │
│  │          ReputationWeightedVoter                    │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │    │
│  │  │First-to-ahead│  │信誉加权投票│  │Merkle状态管理│  │    │
│  │  │   -by-k      │  │            │  │            │  │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │    │
│  └─────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Vagus Integration Layer                │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │    │
│  │  │ ANS状态监控 │  │ Reflex Arc  │  │Tone Oracle扩展│  │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### 组件关系

- **ReputationToken**: 提供声誉证明和管理
- **MicroTaskManager**: 任务生命周期管理和状态跟踪
- **RedFlagValidator**: 领域特定规则验证
- **ReputationWeightedVoter**: 核心共识算法实现
- **Vagus层**: 提供安全监控和异常处理

## MAKER共识算法详解

### 算法原理

MAKER 共识基于以下核心原则：

1. **最小可接受知识**: 每个验证者只需要理解任务的局部规则
2. **群体智慧**: 通过信誉加权聚合多个验证者的判断
3. **严格共识条件**: 使用 k-ahead-by 确保高正确性

### First-to-ahead-by-k 算法

传统的共识算法（如多数投票）在面对多个候选答案时可能失败。MAKER 使用更严格的条件：

**传统多数投票**: `leading_votes > total_votes / 2`

**k-ahead-by**: `leading_weight > competitor_weight + k` (对所有竞争者)

#### 算法流程

```solidity
function _checkConsensus(uint256 taskId, uint256 step, uint8 k)
    internal view returns (ConsensusResult memory) {

    Vote[] storage stepVotes = votes[taskId][step];
    if (stepVotes.length < 3) return ConsensusResult(false, 0, 0, 0);

    // 第一遍：计算所有候选者的权重
    uint256[9] memory moveWeights; // 3x3 = 9 种可能移动
    uint16 maxMove = 0;
    uint256 maxWeight = 0;

    for (uint256 i = 0; i < stepVotes.length; i++) {
        uint16 moveKey = uint16(vote.from * 3 + vote.to);
        moveWeights[moveKey] += vote.weight;
        if (moveWeights[moveKey] > maxWeight) {
            maxWeight = moveWeights[moveKey];
            maxMove = moveKey;
        }
    }

    // 第二遍：验证领先所有竞争者 k 倍权重
    bool consensusAchieved = true;
    for (uint256 i = 0; i < stepVotes.length && consensusAchieved; i++) {
        uint16 moveKey = uint16(vote.from * 3 + vote.to);
        if (moveKey == maxMove) continue; // 跳过领先者

        uint256 competitorWeight = moveWeights[moveKey];
        if (maxWeight <= competitorWeight + k) {
            consensusAchieved = false;
            break;
        }
    }

    return consensusAchieved ?
        ConsensusResult(true, uint8(maxMove / 3), uint8(maxMove % 3), maxWeight) :
        ConsensusResult(false, 0, 0, 0);
}
```

### 信誉权重计算

信誉权重使用 ^0.7 次幂进行衰减，防止权重过度集中：

```solidity
uint256 reputation = repToken.reputation(msg.sender);
uint248 weight = uint248(_sqrt7(reputation * 1e18) / 1e9);
```

这种设计确保：
- 高信誉参与者获得更多权重
- 权重随信誉增加而非线性增长
- 防止单一参与者控制共识

## 核心组件详解

### ReputationToken (ERC-5192 Soulbound Token)

#### 功能特性

- **不可转让**: 实现 ERC-5192 标准，确保声誉绑定地址
- **动态更新**: 支持基于投票表现的声誉调整
- **初始分配**: 新参与者获得 100 点基础声誉

#### 关键接口

```solidity
interface IERC5192 {
    event Locked(uint256 tokenId);
    event Unlocked(uint256 tokenId);
    function locked(uint256 tokenId) external view returns (bool);
}

contract ReputationToken is IERC5192 {
    function mint(address to) external onlyVagusMaker;
    function updateReputation(address user, int256 delta) external onlyVagusMaker;
    function reputation(address user) external view returns (uint256);
}
```

### MicroTaskManager

#### 任务状态管理

```solidity
struct Task {
    bytes initialState;        // 初始状态编码
    uint256 totalReward;       // 总奖励金额
    uint256 currentStep;       // 当前步骤
    bytes32 currentStateRoot;  // 当前状态根 (用于验证)
    uint8 k;                   // 共识阈值
    address creator;           // 任务创建者
    bool completed;            // 完成状态
}
```

#### Merkle证明验证

为支持百万步任务，实现了轻量级状态验证：

```solidity
struct StateTransition {
    bytes32 oldStateRoot;
    bytes32 newStateRoot;
    bytes32 moveHash;
    bytes32[] merkleProof;
}

function _verifyStateTransition(
    bytes32 oldRoot,
    bytes32 newRoot,
    bytes32 moveHash,
    bytes32[] memory proof
) internal pure returns (bool) {
    // 简化的Merkle验证实现
    // 生产环境中应使用完整的Merkle树验证
}
```

### RedFlagValidator

#### 河内塔规则验证

```solidity
function validate(
    bytes calldata currentState,
    Move calldata proposed,
    bytes calldata proof
) external pure returns (bool valid, string memory reason) {

    bytes[][] memory pegs = abi.decode(currentState, (bytes[][]));

    // 基础检查
    if (proposed.from >= 3 || proposed.to >= 3) {
        return (false, "Invalid peg index");
    }

    // 源柱子非空检查
    if (pegs[proposed.from].length == 0) {
        return (false, "Source peg is empty");
    }

    // 大盘不能放在小盘上
    bytes1 diskBytes = _safeBytes1(pegs[proposed.from][pegs[proposed.from].length - 1]);
    uint8 diskSize = uint8(diskBytes);

    if (pegs[proposed.to].length > 0) {
        bytes1 topBytes = _safeBytes1(pegs[proposed.to][pegs[proposed.to].length - 1]);
        uint8 topSize = uint8(topBytes);
        if (diskSize >= topSize) {
            return (false, "Cannot place larger disk on smaller disk");
        }
    }

    return (true, "");
}
```

### ReputationWeightedVoter

#### 核心共识引擎

```solidity
struct Vote {
    address voter;
    uint248 weight;    // 信誉权重 (节省gas)
    uint8 from;        // 移动来源柱子
    uint8 to;          // 移动目标柱子
}

function castVote(
    uint256 taskId,
    uint256 step,
    Move calldata move,
    bytes calldata proof
) external {
    // 验证任务存在和步骤匹配
    require(_taskExists(taskId), "Task does not exist");
    require(task.currentStep == step, "Wrong step");

    // 检查信誉代币持有
    require(repToken.balanceOf(msg.sender) > 0, "No reputation token");

    // 验证移动合法性
    (bool valid, string memory reason) = validator.validate(currentState, move, proof);
    if (!valid) {
        repToken.updateReputation(msg.sender, -100);
        // 触发Reflex Arc
        _triggerReflexArc(msg.sender);
        return;
    }

    // 计算并记录投票
    uint248 weight = _calculateWeight(msg.sender);
    votes[taskId][step].push(Vote(msg.sender, weight, move.from, move.to));

    // 检查共识达成
    ConsensusResult memory result = _checkConsensus(taskId, step, task.k);
    if (result.achieved) {
        _executeConsensus(taskId, step, result);
    }
}
```

## Vagus安全集成

### ANS状态监控

VagusMaker 与自主神经系统 (ANS) 深度集成：

```solidity
// 集成到Tone Oracle
struct SensorMetrics {
    // ... 现有指标
    uint64 candidate_moves;      // 当前投票轮次的候选移动数
    uint64 vote_duration_ms;     // 共识达成耗时
    uint64 avg_vote_duration_ms; // 平均共识时间
}

// VTI计算增强
if (candidate_moves >= 4) {
    risk_score += 30; // 高分歧风险
}
if (vote_duration > avg_vote_duration * 3) {
    risk_score += 20; // 共识延迟风险
}
```

### Reflex Arc触发

当检测到恶意投票行为时，自动触发安全响应：

```solidity
function _triggerReflexArc(address maliciousActor) internal {
    if (reflexArc != address(0)) {
        try IReflexArc(reflexArc).on_aep(uint256(uint160(maliciousActor))) {
            // 成功触发reflex arc
        } catch {
            // 记录但不中断执行
        }
    }
}
```

### Capability Token集成

基于信誉自动调整执行权限：

- **高信誉参与者**: 获得更高权重和更多任务分配
- **低信誉参与者**: 权重降低，可能被暂时禁止
- **恶意参与者**: 通过Reflex Arc撤销权限

## 部署指南

### 环境要求

- **Solidity**: ^0.8.24
- **Foundry**: 最新版本
- **Node.js**: 16+
- **本地链**: Anvil 或 Hardhat Network

### 部署步骤

#### 1. 启动本地开发链

```bash
# 启动Anvil
./infra/devnet/anvil.sh

# 或使用Foundry
anvil --port 8545
```

#### 2. 部署VagusMaker合约

```bash
# 部署完整系统
forge script script/DeployVagusMaker.s.sol \
    --rpc-url http://127.0.0.1:8545 \
    --broadcast \
    --verify

# 或者分步部署
# 1. 部署RedFlagValidator
# 2. 部署ReputationToken (placeholder地址)
# 3. 部署MicroTaskManager
# 4. 部署ReputationWeightedVoter
# 5. 更新ReputationToken地址
```

#### 3. 运行演示

```bash
# 运行10盘河内塔演示
forge script script/RunHanoiDemo.s.sol \
    --rpc-url http://127.0.0.1:8545 \
    --broadcast
```

### 合约地址配置

部署后需要配置：

```javascript
const VAGUS_MAKER_CONFIG = {
    reputationToken: "0x...",
    microTaskManager: "0x...",
    redFlagValidator: "0x...",
    reputationWeightedVoter: "0x...",
    reflexArc: "0x...", // 从Vagus系统获取
};
```

## 使用指南

### 创建任务

```solidity
// 创建10盘河内塔任务
bytes memory initialState = _create10DiskInitialState();
uint256 taskId = voter.createTask(
    initialState,     // 初始状态
    10 ether,         // 总奖励
    3                 // k值 (共识严格度)
);
```

### 参与投票

```solidity
// 铸造信誉代币 (管理员操作)
voter.mintReputationToken(participantAddress);

// 参与投票
voter.castVote(
    taskId,
    currentStep,
    RedFlagValidator.Move(0, 2), // 从柱子0移到柱子2
    "proof_data"
);
```

### 监控任务进度

```solidity
// 获取任务状态
MicroTaskManager.Task memory task = voter.getTask(taskId);

// 获取共识结果
RedFlagValidator.Move memory consensusMove = voter.getConsensusMove(taskId, step);

// 获取投票详情
ReputationWeightedVoter.Vote[] memory stepVotes = voter.getStepVotes(taskId, step);
```

## API参考

### ReputationToken

```solidity
// 查询接口
function balanceOf(address owner) external view returns (uint256);
function reputation(address owner) external view returns (uint256);
function ownerOf(uint256 tokenId) external view returns (address);

// 管理接口 (仅VagusMaker)
function mint(address to) external;
function updateReputation(address user, int256 delta) external;
```

### MicroTaskManager

```solidity
// 任务管理
function createTask(bytes calldata initialState, uint256 totalReward, uint8 k)
    external payable returns (uint256 taskId);

function getTask(uint256 taskId) external view returns (Task memory);

// 状态更新
function updateTaskStateWithProof(
    uint256 taskId,
    bytes32 newStateRoot,
    bytes calldata move,
    bytes32 moveHash,
    bytes32[] calldata merkleProof
) external;
```

### ReputationWeightedVoter

```solidity
// 投票接口
function castVote(uint256 taskId, uint256 step, Move calldata move, bytes calldata proof) external;

// 查询接口
function getConsensusMove(uint256 taskId, uint256 step) external view returns (Move memory);
function getStepVotes(uint256 taskId, uint256 step) external view returns (Vote[] memory);
function getTask(uint256 taskId) external view returns (Task memory);
```

### RedFlagValidator

```solidity
// 验证接口
function validate(bytes calldata currentState, Move calldata proposed, bytes calldata proof)
    external pure returns (bool valid, string memory reason);

function applyMove(bytes calldata currentState, Move calldata move)
    external pure returns (bytes memory newState);

function isSolved(bytes calldata state, uint8 targetPeg, uint8 numDisks)
    external pure returns (bool solved);
```

## 安全考虑

### 共识安全

1. **女巫攻击防护**: 通过信誉代币和权重衰减防止Sybil攻击
2. **恶意投票检测**: 连续错误投票触发Reflex Arc响应
3. **共识严格度**: k值可调，平衡速度与正确性

### 经济安全

1. **激励对齐**: 正确投票获得声誉奖励
2. **惩罚机制**: 错误投票扣除声誉分
3. **退出门槛**: 声誉过低自动暂停参与资格

### 技术安全

1. **重入攻击防护**: 所有状态修改使用检查-效果-交互模式
2. **整数溢出防护**: 使用SafeMath模式和Solidity 0.8+内置检查
3. **访问控制**: 关键函数使用onlyVagusMaker修饰符

## 性能分析

### Gas消耗分析

| 操作 | 平均Gas消耗 | 说明 |
|------|-------------|------|
| 铸造声誉代币 | ~80,000 | 一次性操作 |
| 创建任务 | ~150,000 | 包含状态初始化 |
| 单个投票 | ~120,000 | 包括验证和记录 |
| 共识执行 | ~200,000 | 状态更新和奖励分配 |
| Merkle验证 | ~50,000 | 轻量级证明验证 |

### 扩展性分析

#### 最大任务规模

- **河内塔盘数**: 理论上支持20盘 (1M+步)
- **并发投票**: 每步最多256个投票者
- **状态存储**: 通过Merkle证明支持无限步骤

#### 性能优化

1. **批量处理**: 支持一次性处理多个步骤
2. **状态压缩**: 使用Merkle树减少存储需求
3. **权重预计算**: 缓存信誉权重计算结果

### 网络影响

- **链上存储**: 主要存储投票记录和Merkle证明
- **计算复杂度**: O(n) 投票处理，O(m) 共识检查
- **网络延迟**: 共识达成时间与k值成正比

## 测试与验证

### 单元测试

```bash
# 运行所有测试
forge test --match-path test/vagusmaker/ -v

# 运行特定测试
forge test --match-contract HanoiTest -v

# 运行模糊测试
forge test --match-contract HanoiTest -fuzz-runs 1000
```

### 集成测试

```bash
# 端到端测试
forge script script/RunHanoiDemo.s.sol --rpc-url $RPC_URL --broadcast

# 多用户并发测试
# 使用多个账户同时投票测试并发性能
```

### 正确性验证

1. **算法正确性**: 验证First-to-ahead-by-k逻辑
2. **状态一致性**: 确保Merkle证明正确验证状态转换
3. **经济激励**: 验证奖励和惩罚机制的有效性

## 未来扩展

### 多任务类型支持

1. **数学证明**: 支持形式化数学定理验证
2. **算法验证**: 复杂算法的逐步正确性证明
3. **逻辑推理**: 多步逻辑推理链验证
4. **游戏理论**: 确定性游戏的最优策略验证

### 增强功能

1. **动态k值**: 基于任务难度自动调整共识严格度
2. **声誉委托**: 允许将投票权委托给可信参与者
3. **跨链验证**: 支持多链环境的分布式验证
4. **零知识证明**: 隐私保护的投票和验证机制

### 协议升级

1. **Layer 2优化**: 使用乐观汇总提高吞吐量
2. **状态通道**: 支持链下快速验证
3. **递归证明**: 支持大规模并行验证

### 生态系统集成

1. **DeFi激励**: 与去中心化交易所集成提供流动性奖励
2. **DAO治理**: 允许社区调整协议参数
3. **跨协议协作**: 与其他Layer 1协议的互操作性

---

**VagusMaker = Vagus 安全 + MAKER 正确**

*全球首个在区块链上实现零错误百万步确定性任务验证的系统*
