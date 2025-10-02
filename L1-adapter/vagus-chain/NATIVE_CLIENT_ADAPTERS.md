# Native Client Adapters for Vagus-Chain L1

## 概述

本文档描述了如何更新 Tone Oracle 和 Vagus Gateway 客户端代码，使其与 vagus-chain L1 原生协议交互，而不是与部署的智能合约交互。

## 1. 原生合约地址配置

### 预定义地址常量
```rust
// 在 vagus-chain 客户端库中定义
pub mod native_addresses {
    use ethers::types::Address;
    
    pub const ANS_STATE_MANAGER: Address = Address([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x01
    ]);
    
    pub const CAPABILITY_ISSUER: Address = Address([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x02
    ]);
    
    pub const VAGAL_BRAKE: Address = Address([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x03
    ]);
    
    pub const AFFERENT_INBOX: Address = Address([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x04
    ]);
    
    pub const REFLEX_ARC: Address = Address([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x05
    ]);
}
```

## 2. 配置管理和模式检测

### 2.1 环境变量配置

```bash
# 原生模式配置
export VAGUS_USE_NATIVE_CONTRACTS=true          # 启用原生模式
export VAGUS_CHAIN_RPC_URL=http://localhost:26657  # vagus-chain RPC URL
export VAGUS_CHAIN_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80  # 私钥

# 回退配置 (可选)
export VAGUS_FALLBACK_RPC_URL=http://localhost:8545  # 回退到 Solidity 合约
export VAGUS_SOLIDITY_ANS_MANAGER=0x123...            # Solidity ANSStateManager 地址
```

### 2.2 配置文件示例

```yaml
# config/native-client.yaml
vagus_chain:
  # 原生模式配置
  native_mode: true
  rpc_url: "http://localhost:26657"
  private_key: "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

  # 流量分配 (用于渐进式迁移)
  traffic_split:
    native: 80    # 80% 流量使用原生合约
    solidity: 20  # 20% 流量使用 Solidity 合约

  # 健康检查配置
  health_check:
    interval: 30s
    timeout: 5s
    failure_threshold: 3

  # 回退配置
  fallback:
    enabled: true
    rpc_url: "http://localhost:8545"
    ans_manager_address: "0x1234567890123456789012345678901234567890"
    capability_issuer_address: "0x2345678901234567890123456789012345678901"

# 性能调优
performance:
  rpc_timeout: 50ms  # 原生实现更快
  batch_size: 100    # 批处理大小
  retry_attempts: 3
  retry_delay: 100ms

# 监控配置
monitoring:
  metrics_enabled: true
  prometheus_port: 9090
  log_level: "info"
```

### 2.3 模式检测逻辑

```rust
#[derive(Debug, Clone)]
pub enum ChainMode {
    Native,      // 使用 vagus-chain 原生合约
    Solidity,    // 使用 Solidity 智能合约
    Hybrid {     // 混合模式 (渐进式迁移)
        native_weight: u32,
        solidity_weight: u32,
    },
}

impl ChainMode {
    pub fn from_env() -> Result<Self> {
        let native_mode = std::env::var("VAGUS_USE_NATIVE_CONTRACTS")
            .map(|v| v == "true")
            .unwrap_or(false);

        if !native_mode {
            return Ok(ChainMode::Solidity);
        }

        // 检查是否有流量分配配置
        if let (Ok(native_weight), Ok(solidity_weight)) = (
            std::env::var("VAGUS_NATIVE_TRAFFIC_WEIGHT").map(|v| v.parse()).unwrap_or(Ok(100)),
            std::env::var("VAGUS_SOLIDITY_TRAFFIC_WEIGHT").map(|v| v.parse()).unwrap_or(Ok(0)),
        ) {
            if native_weight < 100 || solidity_weight > 0 {
                return Ok(ChainMode::Hybrid {
                    native_weight,
                    solidity_weight,
                });
            }
        }

        Ok(ChainMode::Native)
    }
}
```

## 3. 更新 Tone Oracle 客户端

### 3.1 原生 ANSStateManager 接口

```rust
// oracle/tone-oracle/src/native_client.rs
use ethers::prelude::*;
use std::sync::Arc;
use crate::native_addresses::ANS_STATE_MANAGER;

/// 原生 ANSStateManager 客户端
pub struct NativeANSStateManager {
    provider: Arc<Provider<Http>>,
    wallet: LocalWallet,
    contract: NativeANSStateManagerContract<Provider<Http>>,
}

impl NativeANSStateManager {
    pub async fn new(rpc_url: &str, private_key: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let wallet: LocalWallet = private_key.parse()?;
        let wallet = wallet.with_chain_id(31337u64); // vagus-chain L1 chain ID
        
        let contract = NativeANSStateManagerContract::new(
            ANS_STATE_MANAGER,
            Arc::new(provider.clone()),
        );
        
        Ok(Self {
            provider,
            wallet,
            contract,
        })
    }
    
    /// 更新执行器的 tone 值
    pub async fn update_tone(&self, executor_id: u256, tone: u32) -> Result<(), Box<dyn std::error::Error>> {
        let client = SignerMiddleware::new(self.provider.clone(), self.wallet.clone());
        let contract = NativeANSStateManagerContract::new(
            self.contract.address(),
            Arc::new(client),
        );
        
        let tx = contract.update_tone(executor_id, tone);
        let pending_tx = tx.send().await?;
        let receipt = pending_tx.confirmations(1).await?;
        
        tracing::info!("Updated ANS tone: ExecutorId={}, Tone={}, TxHash={:?}",
                      executor_id, tone, receipt.unwrap().transaction_hash);
        
        Ok(())
    }
    
    /// 获取执行器状态
    pub async fn get_executor_state(&self, executor_id: u256) -> Result<(u8, u32, u64), Box<dyn std::error::Error>> {
        let (state, tone, updated_at) = self.contract.get_executor_state(executor_id).call().await?;
        Ok((state, tone, updated_at))
    }
    
    /// 获取保护信息
    pub async fn get_guard(&self, executor_id: u256, action_id: [u8; 32]) -> Result<(u256, bool), Box<dyn std::error::Error>> {
        let (scaling_factor, allowed) = self.contract.guard_for(executor_id, action_id).call().await?;
        Ok((scaling_factor, allowed))
    }
}

// 原生合约 ABI 定义
abigen!(
    NativeANSStateManagerContract,
    r#"[
        function updateTone(uint256 executorId, uint32 tone) external
        function getExecutorState(uint256 executorId) external view returns (uint8 state, uint32 tone, uint64 updatedAt)
        function guardFor(uint256 executorId, bytes32 actionId) external view returns (uint256 scalingFactor, bool allowed)
        event VagalToneUpdated(uint256 indexed tone, uint8 indexed state, uint256 updatedAt)
    ]"#,
);
```

### 2.2 更新 Tone Oracle 主库

```rust
// oracle/tone-oracle/src/lib.rs (更新部分)
use crate::native_client::NativeANSStateManager;

/// 更新后的区块链配置
#[derive(Debug, Clone)]
pub struct BlockchainConfig {
    pub rpc_url: String,
    pub private_key: String,
    // 不再需要合约地址，使用预定义的原生地址
}

/// 更新后的区块链集成
pub struct BlockchainOracle {
    ans_manager: NativeANSStateManager,
}

impl BlockchainOracle {
    /// 创建新的区块链 Oracle
    pub async fn new(config: &BlockchainConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let ans_manager = NativeANSStateManager::new(&config.rpc_url, &config.private_key).await?;
        
        Ok(Self {
            ans_manager,
        })
    }
    
    /// 更新 ANS 状态
    pub async fn update_tone(&self, executor_id: u256, vti_value: u64, suggested_state: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 将 VTI 值转换为 ppm (parts per million)
        let tone_ppm = (vti_value * 100) as u32; // 假设 VTI 是 0-100 的百分比
        
        // 调用原生合约
        self.ans_manager.update_tone(executor_id, tone_ppm).await?;
        
        tracing::info!("Updated native ANS tone: ExecutorId={}, VTI={}, Tone={}ppm, State={}",
                      executor_id, vti_value, tone_ppm, suggested_state);
        
        Ok(())
    }
    
    /// 获取当前 ANS 状态
    pub async fn get_current_state(&self, executor_id: u256) -> Result<(u8, u32, u64), Box<dyn std::error::Error>> {
        self.ans_manager.get_executor_state(executor_id).await
    }
}
```

### 2.3 更新 Tone Oracle 主程序

```rust
// oracle/tone-oracle/src/main.rs (更新部分)
use tone_oracle::{BlockchainConfig, SensorMetrics, ToneOracle, VtiConfig, VtiResult};
use vagus_chain::{ChainClient, ChainClientFactory, ChainConfig, ChainType};

/// 更新后的应用状态
#[derive(Clone)]
struct AppState {
    oracle: Arc<Mutex<ToneOracle>>,
    chain_clients: HashMap<ChainType, Arc<dyn ChainClient>>,
    native_ans_manager: Option<Arc<NativeANSStateManager>>, // 新增
}

/// 处理传感器指标提交
async fn handle_submit_metrics(
    State(state): State<AppState>,
    Json(payload): Json<SubmitMetricsRequest>,
) -> Result<Json<VtiResponse>, StatusCode> {
    let metrics = SensorMetrics {
        executor_id: payload.executor_id,
        human_distance_mm: payload.human_distance_mm,
        temperature_celsius: payload.temperature_celsius,
        energy_consumption_j: payload.energy_consumption_j,
        jerk_m_s3: payload.jerk_m_s3,
        timestamp_ms: payload.timestamp_ms.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        }),
    };

    // 处理指标并计算 VTI
    let mut oracle = state.oracle.lock().await;
    let result = oracle.process_metrics(metrics).await.map_err(|e| {
        tracing::error!("Failed to process metrics: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 如果启用了原生 ANS 管理器，直接更新
    if let Some(ans_manager) = &state.native_ans_manager {
        if let Some(vti_result) = &result {
            if vti_result.suggested_state != "UNKNOWN" {
                let executor_id = u256::from(payload.executor_id);
                if let Err(e) = ans_manager.update_tone(executor_id, vti_result.vti_value, &vti_result.suggested_state).await {
                    tracing::warn!("Failed to update native ANS state: {}", e);
                }
            }
        }
    }

    // 同时更新多链客户端（用于跨链同步）
    for (chain_type, client) in &state.chain_clients {
        if let Some(vti_result) = &result {
            let suggested_state = match vti_result.suggested_state.as_str() {
                "SAFE" => vagus_chain::ANSState::SAFE,
                "DANGER" => vagus_chain::ANSState::DANGER,
                "SHUTDOWN" => vagus_chain::ANSState::SHUTDOWN,
                _ => continue,
            };

            match client.update_tone(vti_result.vti_value, suggested_state).await {
                Ok(_) => {
                    tracing::info!("Updated ANS state on {:?} chain", chain_type);
                }
                Err(e) => {
                    tracing::warn!("Failed to update ANS state on {:?} chain: {}", chain_type, e);
                }
            }
        }
    }

    Ok(Json(VtiResponse {
        success: true,
        vti_result: result,
        error: None,
    }))
}

/// 创建应用状态
async fn create_app_state() -> Result<AppState, Box<dyn std::error::Error>> {
    let config = VtiConfig::default();
    let oracle = Arc::new(Mutex::new(ToneOracle::new(config)));
    
    let mut chain_clients = HashMap::new();
    let mut native_ans_manager = None;
    
    // 如果配置了原生 ANS 管理器
    if let Ok(blockchain_config) = std::env::var("BLOCKCHAIN_CONFIG") {
        let config: BlockchainConfig = serde_json::from_str(&blockchain_config)?;
        let ans_manager = NativeANSStateManager::new(&config.rpc_url, &config.private_key).await?;
        native_ans_manager = Some(Arc::new(ans_manager));
    }
    
    // 配置多链客户端
    // ... 现有代码 ...
    
    Ok(AppState {
        oracle,
        chain_clients,
        native_ans_manager,
    })
}
```

## 3. 更新 Vagus Gateway 客户端

### 3.1 原生合约客户端

```rust
// gateway/crates/vagus-chain/src/native_clients.rs
use ethers::prelude::*;
use std::sync::Arc;
use crate::native_addresses::*;

/// 原生 CapabilityIssuer 客户端
pub struct NativeCapabilityIssuer {
    provider: Arc<Provider<Http>>,
    wallet: LocalWallet,
    contract: NativeCapabilityIssuerContract<Provider<Http>>,
}

impl NativeCapabilityIssuer {
    pub async fn new(rpc_url: &str, private_key: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let wallet: LocalWallet = private_key.parse()?;
        let wallet = wallet.with_chain_id(31337u64);
        
        let contract = NativeCapabilityIssuerContract::new(
            CAPABILITY_ISSUER,
            Arc::new(provider.clone()),
        );
        
        Ok(Self {
            provider,
            wallet,
            contract,
        })
    }
    
    /// 发行能力令牌
    pub async fn issue_capability(
        &self,
        intent: &Intent,
        scaled_limits_hash: [u8; 32],
    ) -> Result<u256, Box<dyn std::error::Error>> {
        let client = SignerMiddleware::new(self.provider.clone(), self.wallet.clone());
        let contract = NativeCapabilityIssuerContract::new(
            self.contract.address(),
            Arc::new(client),
        );
        
        let tx = contract.issue_capability(intent.clone(), scaled_limits_hash);
        let pending_tx = tx.send().await?;
        let receipt = pending_tx.confirmations(1).await?;
        
        // 从事件中提取 token ID
        let token_id = extract_token_id_from_receipt(&receipt.unwrap())?;
        
        tracing::info!("Issued capability token: TokenId={}, ExecutorId={}, TxHash={:?}",
                      token_id, intent.executor_id, receipt.unwrap().transaction_hash);
        
        Ok(token_id)
    }
    
    /// 撤销能力令牌
    pub async fn revoke_capability(&self, token_id: u256, reason: u8) -> Result<(), Box<dyn std::error::Error>> {
        let client = SignerMiddleware::new(self.provider.clone(), self.wallet.clone());
        let contract = NativeCapabilityIssuerContract::new(
            self.contract.address(),
            Arc::new(client),
        );
        
        let tx = contract.revoke(token_id, reason);
        let pending_tx = tx.send().await?;
        let _receipt = pending_tx.confirmations(1).await?;
        
        tracing::info!("Revoked capability token: TokenId={}, Reason={}", token_id, reason);
        
        Ok(())
    }
    
    /// 检查令牌有效性
    pub async fn is_valid(&self, token_id: u256) -> Result<bool, Box<dyn std::error::Error>> {
        let valid = self.contract.is_valid(token_id).call().await?;
        Ok(valid)
    }
}

/// 原生 VagalBrake 客户端
pub struct NativeVagalBrake {
    provider: Arc<Provider<Http>>,
    wallet: LocalWallet,
    contract: NativeVagalBrakeContract<Provider<Http>>,
}

impl NativeVagalBrake {
    pub async fn new(rpc_url: &str, private_key: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let wallet: LocalWallet = private_key.parse()?;
        let wallet = wallet.with_chain_id(31337u64);
        
        let contract = NativeVagalBrakeContract::new(
            VAGAL_BRAKE,
            Arc::new(provider.clone()),
        );
        
        Ok(Self {
            provider,
            wallet,
            contract,
        })
    }
    
    /// 预览制动效果
    pub async fn preview_brake(&self, intent: &Intent) -> Result<([u8; 32], bool), Box<dyn std::error::Error>> {
        let (scaled_limits_hash, allowed) = self.contract.preview_brake(intent.clone()).call().await?;
        Ok((scaled_limits_hash, allowed))
    }
    
    /// 带制动发行能力
    pub async fn issue_with_brake(&self, intent: &Intent) -> Result<u256, Box<dyn std::error::Error>> {
        let client = SignerMiddleware::new(self.provider.clone(), self.wallet.clone());
        let contract = NativeVagalBrakeContract::new(
            self.contract.address(),
            Arc::new(client),
        );
        
        let tx = contract.issue_with_brake(intent.clone());
        let pending_tx = tx.send().await?;
        let receipt = pending_tx.confirmations(1).await?;
        
        let token_id = extract_token_id_from_receipt(&receipt.unwrap())?;
        
        tracing::info!("Issued capability with brake: TokenId={}, ExecutorId={}", 
                      token_id, intent.executor_id);
        
        Ok(token_id)
    }
}

/// 原生 AfferentInbox 客户端
pub struct NativeAfferentInbox {
    provider: Arc<Provider<Http>>,
    wallet: LocalWallet,
    contract: NativeAfferentInboxContract<Provider<Http>>,
}

impl NativeAfferentInbox {
    pub async fn new(rpc_url: &str, private_key: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let wallet: LocalWallet = private_key.parse()?;
        let wallet = wallet.with_chain_id(31337u64);
        
        let contract = NativeAfferentInboxContract::new(
            AFFERENT_INBOX,
            Arc::new(provider.clone()),
        );
        
        Ok(Self {
            provider,
            wallet,
            contract,
        })
    }
    
    /// 提交传入证据包
    pub async fn post_aep(
        &self,
        executor_id: u256,
        state_root: [u8; 32],
        metrics_hash: [u8; 32],
        signature: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = SignerMiddleware::new(self.provider.clone(), self.wallet.clone());
        let contract = NativeAfferentInboxContract::new(
            self.contract.address(),
            Arc::new(client),
        );
        
        let tx = contract.post_aep(executor_id, state_root, metrics_hash, signature);
        let pending_tx = tx.send().await?;
        let _receipt = pending_tx.confirmations(1).await?;
        
        tracing::info!("Posted AEP: ExecutorId={}, StateRoot={:?}", executor_id, state_root);
        
        Ok(())
    }
    
    /// 获取最新状态根
    pub async fn latest_state_root(&self, executor_id: u256) -> Result<[u8; 32], Box<dyn std::error::Error>> {
        let state_root = self.contract.latest_state_root(executor_id).call().await?;
        Ok(state_root)
    }
}

// 原生合约 ABI 定义
abigen!(
    NativeCapabilityIssuerContract,
    r#"[
        function issueCapability((uint256 executorId, bytes32 actionId, bytes params, bytes32 envelopeHash, bytes32 preStateRoot, uint64 notBefore, uint64 notAfter, uint32 maxDurationMs, uint32 maxEnergyJ, address planner, uint256 nonce) intent, bytes32 scaledLimitsHash) external returns (uint256 tokenId)
        function revoke(uint256 tokenId, uint8 reason) external
        function isValid(uint256 tokenId) external view returns (bool)
        function activeTokensOf(uint256 executorId) external view returns (uint256[] memory)
        event CapabilityIssued(uint256 indexed tokenId, uint256 indexed executorId, address indexed planner, bytes32 actionId, uint256 expiresAt, bytes32 paramsHashSha256, bytes32 paramsHashKeccak, bytes32 preStateRootSha256, bytes32 preStateRootKeccak)
        event CapabilityRevoked(uint256 indexed tokenId, uint8 reason)
    ]"#,
);

abigen!(
    NativeVagalBrakeContract,
    r#"[
        function previewBrake((uint256 executorId, bytes32 actionId, bytes params, bytes32 envelopeHash, bytes32 preStateRoot, uint64 notBefore, uint64 notAfter, uint32 maxDurationMs, uint32 maxEnergyJ, address planner, uint256 nonce) intent) external view returns (bytes32 scaledLimitsHash, bool allowed)
        function issueWithBrake((uint256 executorId, bytes32 actionId, bytes params, bytes32 envelopeHash, bytes32 preStateRoot, uint64 notBefore, uint64 notAfter, uint32 maxDurationMs, uint32 maxEnergyJ, address planner, uint256 nonce) intent) external returns (uint256 tokenId)
    ]"#,
);

abigen!(
    NativeAfferentInboxContract,
    r#"[
        function postAEP(uint256 executorId, bytes32 stateRoot, bytes32 metricsHash, bytes signature) external
        function latestStateRoot(uint256 executorId) external view returns (bytes32)
        event AEPPosted(uint256 indexed executorId, bytes32 stateRoot, bytes32 metricsHash)
    ]"#,
);
```

### 3.2 更新 ChainClient 实现

```rust
// gateway/crates/vagus-chain/src/native_chain_client.rs
use super::{ChainClient, ChainConfig, ChainType, Guard, ANSState};
use crate::native_clients::*;
use anyhow::Result;

/// 原生链客户端实现
pub struct NativeChainClient {
    ans_manager: NativeANSStateManager,
    capability_issuer: NativeCapabilityIssuer,
    vagal_brake: NativeVagalBrake,
    afferent_inbox: NativeAfferentInbox,
}

impl NativeChainClient {
    pub async fn new(config: &ChainConfig) -> Result<Self> {
        let rpc_url = &config.rpc_url;
        let private_key = config.private_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Private key required for native client"))?;
        
        let ans_manager = NativeANSStateManager::new(rpc_url, private_key).await?;
        let capability_issuer = NativeCapabilityIssuer::new(rpc_url, private_key).await?;
        let vagal_brake = NativeVagalBrake::new(rpc_url, private_key).await?;
        let afferent_inbox = NativeAfferentInbox::new(rpc_url, private_key).await?;
        
        Ok(Self {
            ans_manager,
            capability_issuer,
            vagal_brake,
            afferent_inbox,
        })
    }
}

#[async_trait::async_trait]
impl ChainClient for NativeChainClient {
    async fn submit_aep(&self, aep: &AfferentEvidencePacket) -> Result<String> {
        let executor_id = u256::from(aep.executor_id);
        let state_root = aep.state_root;
        let metrics_hash = aep.metrics_hash;
        let signature = aep.signature.clone();
        
        self.afferent_inbox.post_aep(executor_id, state_root, metrics_hash, signature).await?;
        
        Ok(format!("aep_{}", aep.executor_id))
    }
    
    async fn issue_with_brake(
        &self,
        intent: &Intent,
        scaled_limits_hash: &[u8; 32],
        expires_at: u64,
    ) -> Result<String> {
        let mut intent_clone = intent.clone();
        intent_clone.not_after = expires_at;
        
        let token_id = self.vagal_brake.issue_with_brake(&intent_clone).await?;
        
        Ok(token_id.to_string())
    }
    
    async fn revoke_capability(&self, token_id: &str, reason: u8) -> Result<()> {
        let token_id = u256::from_str_radix(token_id, 10)?;
        self.capability_issuer.revoke_capability(token_id, reason).await?;
        Ok(())
    }
    
    async fn get_guard(&self, action_id: &[u8; 32]) -> Result<Guard> {
        // 使用默认执行器 ID，在实际实现中应该从上下文获取
        let executor_id = u256::from(1);
        
        let (scaling_factor, allowed) = self.ans_manager.get_guard(executor_id, *action_id).await?;
        
        Ok(Guard {
            scaling_factor: scaling_factor.as_u64(),
            allowed,
        })
    }
    
    async fn get_ans_state(&self) -> Result<ANSState> {
        let executor_id = u256::from(1); // 默认执行器 ID
        let (state, _tone, _updated_at) = self.ans_manager.get_executor_state(executor_id).await?;
        
        let ans_state = match state {
            0 => ANSState::SAFE,
            1 => ANSState::DANGER,
            2 => ANSState::SHUTDOWN,
            _ => ANSState::SAFE,
        };
        
        Ok(ans_state)
    }
    
    async fn update_tone(&self, vti: u64, suggested_state: ANSState) -> Result<()> {
        let executor_id = u256::from(1); // 默认执行器 ID
        let tone_ppm = (vti * 100) as u32; // 转换为 ppm
        
        self.ans_manager.update_tone(executor_id, tone_ppm).await?;
        
        Ok(())
    }
}
```

### 3.3 更新 ChainClientFactory

```rust
// gateway/crates/vagus-chain/src/lib.rs (更新部分)
use crate::native_chain_client::NativeChainClient;

impl ChainClientFactory {
    pub async fn create_client(config: ChainConfig) -> Result<Arc<dyn ChainClient>> {
        match config.chain_type {
            ChainType::EVM => {
                // 检查是否使用原生合约
                if is_native_chain(&config) {
                    let client = NativeChainClient::new(&config).await?;
                    Ok(Arc::new(client))
                } else {
                    // 使用传统的智能合约客户端
                    create_evm_client(config).await
                }
            }
            ChainType::Cosmos => {
                // 创建 CosmWasm 客户端
                create_cosmos_client(config).await
            }
        }
    }
}

/// 检查是否使用原生链
fn is_native_chain(config: &ChainConfig) -> bool {
    // 检查是否配置了原生合约地址
    config.contract_addresses.contains_key("ans_state_manager") &&
    config.contract_addresses["ans_state_manager"] == "0x0000000000000000000000000000000000000001"
}
```

## 4. 配置更新

### 4.1 环境变量配置

```bash
# .env 文件
# 原生链配置
VAGUS_CHAIN_RPC_URL=http://localhost:26657
VAGUS_CHAIN_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80

# 启用原生模式
VAGUS_USE_NATIVE_CONTRACTS=true

# 多链配置（用于跨链同步）
MULTICHAIN_ENABLED=true
EVM_RPC_URL=http://localhost:8545
COSMOS_RPC_URL=http://localhost:26657
```

### 4.2 配置文件更新

```yaml
# config/native.yaml
vagus_chain:
  rpc_url: "http://localhost:26657"
  chain_id: 31337
  native_contracts:
    enabled: true
    addresses:
      ans_state_manager: "0x0000000000000000000000000000000000000001"
      capability_issuer: "0x0000000000000000000000000000000000000002"
      vagal_brake: "0x0000000000000000000000000000000000000003"
      afferent_inbox: "0x0000000000000000000000000000000000000004"
      reflex_arc: "0x0000000000000000000000000000000000000005"

multichain:
  enabled: true
  chains:
    - type: "evm"
      rpc_url: "http://localhost:8545"
      chain_id: 31337
    - type: "cosmos"
      rpc_url: "http://localhost:26657"
      chain_id: "vagus-test-1"
```

## 5. 部署和测试

### 5.1 构建更新后的客户端

```bash
# 构建 Tone Oracle
cd oracle/tone-oracle
cargo build --release --features native-contracts

# 构建 Vagus Gateway
cd gateway
cargo build --release --features native-contracts
```

### 5.2 测试原生集成

```bash
# 启动 vagus-chain L1
./infra/devnet/vagus-chain.sh

# 启动 Tone Oracle（原生模式）
export VAGUS_USE_NATIVE_CONTRACTS=true
export VAGUS_CHAIN_RPC_URL=http://localhost:26657
cargo run --bin tone-oracle start

# 启动 Vagus Gateway（原生模式）
export VAGUS_USE_NATIVE_CONTRACTS=true
cargo run --bin vagus-gateway start --executor-id 1 --chain evm --rpc-url http://localhost:26657
```

### 5.3 验证功能

```bash
# 测试传感器指标提交
curl -X POST http://localhost:3000/submit-metrics \
  -H "Content-Type: application/json" \
  -d '{
    "executor_id": 1,
    "human_distance_mm": 1000.0,
    "temperature_celsius": 25.0,
    "energy_consumption_j": 100.0,
    "jerk_m_s3": 5.0
  }'

# 验证 ANS 状态更新
curl http://localhost:3000/health
```

## 6. 迁移检查清单

### 6.1 代码更新
- [ ] 添加原生合约地址常量
- [ ] 实现原生客户端接口
- [ ] 更新 ChainClient 实现
- [ ] 更新配置管理
- [ ] 添加原生模式检测

### 6.2 测试验证
- [ ] 单元测试通过
- [ ] 集成测试通过
- [ ] 端到端测试通过
- [ ] 性能测试通过
- [ ] 回归测试通过

### 6.3 部署准备
- [ ] 构建脚本更新
- [ ] 配置文件更新
- [ ] 文档更新
- [ ] 监控配置更新
- [ ] 备份和回滚计划

### 6.4 生产部署
- [ ] 预生产环境测试
- [ ] 生产环境部署
- [ ] 监控和告警配置
- [ ] 性能基准测试
- [ ] 用户验收测试

## 6. 混合模式实现 (渐进式迁移)

### 6.1 流量分配器

```rust
use rand::Rng;

pub struct TrafficAllocator {
    native_weight: u32,
    solidity_weight: u32,
    total_weight: u32,
}

impl TrafficAllocator {
    pub fn new(native_weight: u32, solidity_weight: u32) -> Self {
        let total_weight = native_weight + solidity_weight;
        Self {
            native_weight,
            solidity_weight,
            total_weight,
        }
    }

    pub fn should_use_native(&self) -> bool {
        if self.total_weight == 0 {
            return false;
        }

        let mut rng = rand::thread_rng();
        let roll = rng.gen_range(0..self.total_weight);
        roll < self.native_weight
    }
}
```

### 6.2 双重客户端实现

```rust
pub struct HybridChainClient {
    native_client: Option<NativeChainClient>,
    solidity_client: Option<EvmChainClient>,
    allocator: TrafficAllocator,
    metrics: Arc<ClientMetrics>,
}

impl HybridChainClient {
    pub async fn new(
        native_config: Option<ChainConfig>,
        solidity_config: Option<ChainConfig>,
        allocator: TrafficAllocator,
    ) -> Result<Self> {
        let native_client = if let Some(config) = native_config {
            Some(NativeChainClient::new(&config).await?)
        } else {
            None
        };

        let solidity_client = if let Some(config) = solidity_config {
            Some(EvmChainClient::new(&config).await?)
        } else {
            None
        };

        Ok(Self {
            native_client,
            solidity_client,
            allocator,
            metrics: Arc::new(ClientMetrics::new()),
        })
    }
}

#[async_trait]
impl ChainClient for HybridChainClient {
    async fn update_tone(&self, vti: u64, state: ANSState) -> Result<()> {
        let use_native = self.allocator.should_use_native();
        let start_time = Instant::now();

        let result = if use_native {
            if let Some(client) = &self.native_client {
                client.update_tone(vti, state).await
            } else {
                // 回退到 Solidity 客户端
                self.metrics.record_fallback();
                self.solidity_client.as_ref()
                    .ok_or(VagusError::ConfigurationError)?
                    .update_tone(vti, state).await
            }
        } else {
            self.solidity_client.as_ref()
                .ok_or(VagusError::ConfigurationError)?
                .update_tone(vti, state).await
        };

        let duration = start_time.elapsed();
        self.metrics.record_operation(use_native, result.is_ok(), duration);

        result
    }
}
```

## 总结

这个客户端适配指南提供了完整的从 Solidity 合约到 vagus-chain 原生协议的客户端迁移路径：

1. **配置管理**: 灵活的模式检测和配置系统
2. **混合模式**: 支持渐进式迁移的流量分配
3. **错误处理**: 健壮的回退和健康检查机制
4. **性能优化**: 原生实现特定的性能调优
5. **监控集成**: 完整的可观测性和调试支持

通过遵循这个指南，Tone Oracle 和 Vagus Gateway 可以无缝迁移到使用 vagus-chain 原生协议，同时保持向后兼容性和系统稳定性。
