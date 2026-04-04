# Binance 集成重构设计方案

## 背景

当前 tbay 使用 `binance-rs` crate 仅调用极少量功能：
- REST API：`FuturesMarket::get_klines()` 获取历史K线
- WebSocket：`FuturesWebSockets` 订阅实时K线

存在的问题：
1. 大量未使用的依赖代码
2. WebSocket 无法配置代理（代码层面和环境变量均不支持）

## 目标

1. 移除 `binance` crate，减少冗余依赖
2. 支持 SOCKS5 代理配置（HTTP 和 WebSocket 均生效）
3. 对外接口保持不变，最小化迁移成本

## 技术方案

### 技术选型

| 组件 | 选型 | 说明 |
|------|------|------|
| HTTP 客户端 | `reqwest` | 原生支持 SOCKS5 代理（`socks` feature） |
| WebSocket | `reqwest-websocket` | 基于 reqwest 的 WebSocket 升级，复用代理配置 |
| Async Runtime | `tokio` | 为 WebSocket 事件循环提供 runtime |
| Stream | `futures-util` | Stream 扩展 trait |

### 架构设计

```
::event::binance/
├── mod.rs              # 模块入口，统一导出
├── proxy.rs            # 代理配置（ProxyConfig）
├── client.rs           # 统一的 reqwest client（HTTP + WebSocket 共用）
├── convert.rs          # 数据转换（Binance JSON → 内部 K 类型）
└── fut.rs              # FutClient + BinanceDataSource（对外接口）
```

### 新增依赖

```toml
reqwest = { version = "0.12", features = ["json", "socks", "rustls"] }
reqwest-websocket = "0.6"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
futures-util = "0.3"
```

移除：
```toml
binance = "0.21.2"
```

### ProxyConfig（proxy.rs）

```rust
#[derive(Clone, Debug, Default)]
pub struct ProxyConfig {
    pub socks5: Option<Socks5Config>,
}

#[derive(Clone, Debug)]
pub struct Socks5Config {
    pub addr: String,          // "host:port"
    pub username: Option<String>,
    pub password: Option<String>,
}

impl ProxyConfig {
    /// 从环境变量构建
    /// 支持 SOCKS5_PROXY=socks5://user:pass@host:port
    pub fn from_env() -> Option<Self> { ... }
}
```

### BinanceClient（client.rs）

统一的 reqwest `Client`，SOCKS5 代理一次配置，HTTP 和 WebSocket 共用。

```rust
pub struct BinanceClient {
    inner: reqwest::Client,
    proxy: ProxyConfig,
}

impl BinanceClient {
    pub fn new(proxy: ProxyConfig) -> Result<Self> {
        let mut builder = reqwest::Client::builder();
        if let Some(socks5) = &proxy.socks5 {
            let mut proxy_url = format!("socks5://{}", socks5.addr);
            if let (Some(u), Some(p)) = (&socks5.username, &socks5.password) {
                proxy_url = format!("socks5://{u}:{p}@{}", socks5.addr);
            }
            builder = builder.proxy(reqwest::Proxy::all(&proxy_url)?);
        }
        let inner = builder.build().context(BuildClientCtx)?;
        Ok(Self { inner, proxy })
    }

    /// HTTP 请求获取历史K线（同步接口，内部 block_on）
    pub fn get_klines(
        &self,
        symbol: &str,
        interval: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<KlineSummaries> {
        tokio::runtime::Handle::current().block_on(self._get_klines(...))
    }

    async fn _get_klines(&self, ...) -> Result<KlineSummaries> { ... }

    /// WebSocket 连接
    pub async fn ws_connect(&self, url: &str) -> Result<WebSocket> { ... }
}
```

### FutClient（fut.rs）

保留现有 API，内部替换为 reqwest：

```rust
impl FutClient {
    /// 向后兼容的构造函数（默认从环境变量读取代理）
    pub fn new(testnet: bool, verbose: bool) -> Result<Self> {
        Self::with_proxy(testnet, verbose, ProxyConfig::from_env().unwrap_or_default())
    }

    /// 支持显式传入代理配置
    pub fn with_proxy(testnet: bool, verbose: bool, proxy: ProxyConfig) -> Result<Self> {
        let client = BinanceClient::new(proxy)?;
        Ok(Self { client, testnet, verbose })
    }

    pub fn load_history(&self, target: &Target, count: usize) -> Result<Vec<K>> {
        // 调用 client.get_klines()（内部 block_on）
    }
}
```

### BinanceDataSource（fut.rs）

保留现有 `DataSource` trait 实现，内部替换为 reqwest-websocket：

```rust
impl DataSource for BinanceDataSource {
    /// 向后兼容的构造函数（默认从环境变量读取代理）
    fn new(event_tx: EventChanTx) -> Self {
        Self::with_proxy(event_tx, ProxyConfig::from_env().unwrap_or_default())
    }

    /// 支持显式传入代理配置
    fn with_proxy(event_tx: EventChanTx, proxy: ProxyConfig) -> Self {
        Self { event_tx, proxy }
    }

    fn start(self, targets: Vec<Target>) -> Result<impl SubscribeStopper> {
        let (res_tx, res_rx) = bounded(1);
        let running = Arc::new(AtomicBool::new(true));
        let event_tx = self.event_tx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let client = match BinanceClient::new(self.proxy.clone()) {
                Ok(c) => c,
                Err(e) => {
                    _ = res_tx.send(Err(e));
                    return;
                }
            };

            let connect_result = rt.block_on(async {
                let url = "wss://fstream.binance.com/ws/...";
                client.ws_connect(url).await
            });

            match connect_result {
                Ok(ws) => {
                    _ = res_tx.send(Ok(()));
                    // 事件循环：通过 Stream 获取消息，转为 K 后通过 crossbeam-channel 发送
                }
                Err(e) => {
                    _ = res_tx.send(Err(e));
                }
            }
        });

        // 接收 res_rx 确认连接建立成功
        res_rx.recv().map_err(|_| Error::Msg { reason: "channel broken".into() })??;

        Ok(BinanceSubscriberStopper { _handler: handler, running })
    }
}
```

## 迁移路径

### 步骤 1：替换依赖

```toml
# Cargo.toml
# 删除
binance = "0.21.2"

# 添加
reqwest = { version = "0.12", features = ["json", "socks", "rustls"] }
reqwest-websocket = "0.6"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
futures-util = "0.3"
```

### 步骤 2：创建新模块

- `src/event/binance/proxy.rs`
- `src/event/binance/client.rs`
- `src/event/binance/convert.rs`

### 步骤 3：重写 fut.rs

- 移除 binance crate imports
- 重写 `FutClient` 内部实现
- 重写 `BinanceDataSource::start()` 内部实现
- 保留 `kline_event_to_k`、`continuous_kline_event_to_k`、`kline_summary_to_k` 转换逻辑（调整以适配 reqwest 的 JSON 解析）

### 步骤 4：验证

CLI 层默认从 `SOCKS5_PROXY` 环境变量读取代理，零改动即可运行。如需自定义代理，使用 `with_proxy` 变体。

## 错误处理

- reqwest 错误：通过 `?` 传播，包装为 `Error::Msg`
- tokio runtime 创建失败：通过 res channel 传递
- WebSocket 连接失败：通过 res channel 传递

## 测试验证

1. `make build` - 编译检查
2. 不使用代理运行 simple/watch 命令
3. 设置 `SOCKS5_PROXY` 环境变量后运行，验证代理生效
