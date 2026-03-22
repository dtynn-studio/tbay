//! HTTP 客户端：获取历史 K线数据

use reqwest::Client;

use super::{
    convert::{KlineSummaries, KlineSummary},
    proxy::ProxyConfig,
};
use crate::prelude::*;

const FUTURES_API_BASE: &str = "https://fapi.binance.com";

pub struct BinanceHttpClient {
    inner: Client,
    testnet: bool,
    verbose: bool,
}

impl BinanceHttpClient {
    pub fn new(
        proxy: ProxyConfig,
        testnet: bool,
        verbose: bool,
    ) -> Result<Self> {
        let mut builder = Client::builder();

        if let Some(url) = proxy.to_reqwest_proxy_url() {
            builder =
                builder.proxy(reqwest::Proxy::all(&url).map_err(|e| {
                    Error::Msg {
                        reason: format!("reqwest proxy: {e}").into(),
                    }
                })?);
        }

        let inner = builder.build().map_err(|e| Error::Msg {
            reason: format!("build reqwest client: {e}").into(),
        })?;
        Ok(Self {
            inner,
            testnet,
            verbose,
        })
    }

    pub fn get_klines(
        &self,
        symbol: &str,
        interval: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<KlineSummaries> {
        let handle = tokio::runtime::Handle::current();
        handle
            .block_on(self._get_klines(symbol, interval, start_time, end_time))
    }

    async fn _get_klines(
        &self,
        symbol: &str,
        interval: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<KlineSummaries> {
        let base = if self.testnet {
            "https://testnet.binancefuture.com"
        } else {
            FUTURES_API_BASE
        };

        let url = format!(
            "{}/fapi/v1/klines?symbol={}&interval={}&startTime={}&endTime={}&limit=1500",
            base, symbol, interval, start_time, end_time
        );

        if self.verbose {
            tracing::debug!("GET {}", url);
        }

        let resp =
            self.inner.get(&url).send().await.map_err(|e| Error::Msg {
                reason: format!("reqwest send: {e}").into(),
            })?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| Error::Msg {
            reason: format!("read body: {e}").into(),
        })?;

        if !status.is_success() {
            return Err(Error::Msg {
                reason: format!("HTTP {}: {}", status.as_u16(), body).into(),
            });
        }

        let klines: Vec<Vec<serde_json::Value>> = serde_json::from_str(&body)
            .map_err(|e| Error::Msg {
            reason: format!("parse json: {e}").into(),
        })?;

        let summaries = klines
            .into_iter()
            .map(|row| {
                Ok(KlineSummary {
                    open_time: row.first().and_then(|v| v.as_i64()).unwrap_or(0),
                    open: row
                        .get(1)
                        .and_then(|v| v.as_str())
                        .unwrap_or("0")
                        .to_string(),
                    high: row
                        .get(2)
                        .and_then(|v| v.as_str())
                        .unwrap_or("0")
                        .to_string(),
                    low: row
                        .get(3)
                        .and_then(|v| v.as_str())
                        .unwrap_or("0")
                        .to_string(),
                    close: row
                        .get(4)
                        .and_then(|v| v.as_str())
                        .unwrap_or("0")
                        .to_string(),
                    volume: row
                        .get(5)
                        .and_then(|v| v.as_str())
                        .unwrap_or("0")
                        .to_string(),
                    close_time: row
                        .get(6)
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    number_of_trades: row
                        .get(8)
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(KlineSummaries::AllKlineSummaries(summaries))
    }
}
