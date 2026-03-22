//! WebSocket 客户端：订阅实时 K线数据

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use futures_util::StreamExt;
use reqwest::Client;
use reqwest_websocket::{Message, Upgrade, WebSocket};

use super::{
    convert::{ContinuousKlineEvent, KlineEvent},
    proxy::ProxyConfig,
};
use crate::prelude::*;

/// WebSocket 连接控制
pub struct WsConnection {
    pub running: Arc<AtomicBool>,
}

impl WsConnection {
    /// 通过 reqwest-websocket 建立 WebSocket 连接（SOCKS5 代理自动生效）
    pub async fn connect(
        proxy: &ProxyConfig,
        url: &str,
    ) -> Result<(Self, WebSocket)> {
        let mut builder = Client::builder();

        if let Some(proxy_url) = proxy.to_reqwest_proxy_url() {
            builder = builder.proxy(reqwest::Proxy::all(&proxy_url).map_err(
                |e| Error::Msg {
                    reason: format!("reqwest proxy: {e}").into(),
                },
            )?);
        }

        let client = builder.build().map_err(|e| Error::Msg {
            reason: format!("build reqwest client: {e}").into(),
        })?;

        let ws = client
            .get(url)
            .upgrade()
            .send()
            .await
            .map_err(|e| Error::Msg {
                reason: format!("ws connect: {e}").into(),
            })?
            .into_websocket()
            .await
            .map_err(|e| Error::Msg {
                reason: format!("ws upgrade: {e}").into(),
            })?;

        Ok((
            Self {
                running: Arc::new(AtomicBool::new(true)),
            },
            ws,
        ))
    }

    pub fn disconnect(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

/// 尝试解析 JSON
pub fn try_parse<T: serde::de::DeserializeOwned>(text: &str) -> Option<T> {
    serde_json::from_str(text).ok()
}

/// 构建订阅消息
pub fn build_subscribe_msg(streams: &[String]) -> String {
    let params: Vec<&str> = streams.iter().map(|s| s.as_str()).collect();
    serde_json::to_string(&serde_json::json!({
        "method": "SUBSCRIBE",
        "params": params,
        "id": 1
    }))
    .unwrap_or_default()
}

/// 构建取消订阅消息
pub fn build_unsubscribe_msg(streams: &[String]) -> String {
    let params: Vec<&str> = streams.iter().map(|s| s.as_str()).collect();
    serde_json::to_string(&serde_json::json!({
        "method": "UNSUBSCRIBE",
        "params": params,
        "id": 2
    }))
    .unwrap_or_default()
}

/// 事件循环：在 tokio runtime 中运行
pub async fn run_event_loop(
    mut ws: WebSocket,
    running: Arc<AtomicBool>,
    mut on_kline: impl FnMut(KlineEvent) -> Result<()> + Send + 'static,
    mut on_continuous_kline: impl FnMut(ContinuousKlineEvent) -> Result<()>
    + Send
    + 'static,
) {
    while running.load(Ordering::Relaxed) {
        tokio::select! {
            result = ws.next() => {
                match result {
                    Some(Ok(Message::Text(text))) => {
                        if let Some(kline) = try_parse::<KlineEvent>(&text)
                            && let Err(e) = on_kline(kline)
                        {
                            tracing::warn!(?e, "kline handler error");
                        } else if let Some(ck) = try_parse::<ContinuousKlineEvent>(&text)
                            && let Err(e) = on_continuous_kline(ck)
                        {
                            tracing::warn!(?e, "continuous kline handler error");
                        }
                    }
                    Some(Ok(Message::Close { .. })) | None => {
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::warn!(?e, "ws error");
                        break;
                    }
                    _ => {}
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                // keep alive
            }
        }
    }
}
