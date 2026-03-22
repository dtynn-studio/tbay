#![allow(clippy::result_large_err)]

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};

use humantime::Duration;
use tracing::{debug, warn_span};

use super::client::BinanceHttpClient;
use super::convert::KlineSummaries;
use super::proxy::ProxyConfig;
use crate::{
    event::{DataSource, EventChanTx, K, SubscribeStopper, Target},
    prelude::*,
    util::time::{
        MILLI_SEC, local_from_unix_timestamp_millis_truncated, truncate,
    },
};

fn kline_summary_to_k(
    pair: &str,
    interval: Duration,
    summary: super::convert::KlineSummary,
    finalized: bool,
) -> Result<K> {
    let kraw = KRaw {
        time_begin: local_from_unix_timestamp_millis_truncated(
            "time_begin",
            summary.open_time,
        )?,

        time_end: local_from_unix_timestamp_millis_truncated(
            "time_end",
            summary.close_time,
        )?,

        price_open: Decimal::from_str_radix(&summary.open, 10).context(
            DecimalCtx {
                field: "price_open",
            },
        )?,

        price_close: Decimal::from_str_radix(&summary.close, 10).context(
            DecimalCtx {
                field: "price_close",
            },
        )?,

        price_high: Decimal::from_str_radix(&summary.high, 10).context(
            DecimalCtx {
                field: "price_high",
            },
        )?,

        price_low: Decimal::from_str_radix(&summary.low, 10)
            .context(DecimalCtx { field: "price_low" })?,

        quantity: Decimal::from_str_radix(&summary.volume, 10)
            .context(DecimalCtx { field: "quantity" })?,

        trades: summary.number_of_trades,
        finalized,
    };

    Ok(K {
        symbol: pair.to_lowercase(),
        interval,
        source: "h",
        raw: kraw,
    })
}

// =============================================================================
// BinanceDataSource (WebSocket) - TODO: 实现
// =============================================================================

pub struct BinanceDataSource {
    event_tx: EventChanTx,
    proxy: ProxyConfig,
}

impl DataSource for BinanceDataSource {
    fn new(event_tx: EventChanTx) -> Self {
        Self::with_proxy(event_tx, ProxyConfig::from_env().unwrap_or_default())
    }

    #[allow(refining_impl_trait)]
    fn start(self, _targets: Vec<Target>) -> Result<BinanceSubscriberStopper> {
        // TODO: 使用 reqwest-websocket 实现 WebSocket 订阅
        let _ = self;
        let _ = _targets;
        Err(Error::Msg {
            reason: "BinanceDataSource WebSocket not yet implemented".into(),
        })
    }
}

impl BinanceDataSource {
    pub fn with_proxy(event_tx: EventChanTx, proxy: ProxyConfig) -> Self {
        Self { event_tx, proxy }
    }
}

pub struct BinanceSubscriberStopper {
    _handler: JoinHandle<()>,
    running: Arc<AtomicBool>,
}

impl Drop for BinanceSubscriberStopper {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl SubscribeStopper for BinanceSubscriberStopper {
    fn stop(self) {
        drop(self)
    }
}

// =============================================================================
// FutClient (HTTP)
// =============================================================================

pub struct FutClient {
    client: BinanceHttpClient,
}

impl FutClient {
    pub fn new(testnet: bool, verbose: bool) -> Result<Self> {
        Self::with_proxy(testnet, verbose, ProxyConfig::from_env().unwrap_or_default())
    }

    pub fn with_proxy(testnet: bool, verbose: bool, proxy: ProxyConfig) -> Result<Self> {
        let client = BinanceHttpClient::new(proxy, testnet, verbose)?;
        Ok(Self { client })
    }

    pub fn load_history(
        &self,
        target: &Target,
        count: usize,
    ) -> Result<Vec<K>> {
        let _span = warn_span!("historical ks", sym = target.symbol, int = %target.interval).entered();
        let mut history_ks: Vec<K> = Vec::new();
        let mut first_req = true;
        let d = std::time::Duration::from(target.interval);
        let end_time = OffsetDateTime::now_local()?;
        let next_period_start_millis =
            (truncate("next period start", end_time, d)? + d).unix_timestamp()
                * MILLI_SEC;
        let start_time = end_time - (count as u32) * d;

        loop {
            let request_start = history_ks
                .last()
                .map(|k| k.raw.time_begin + d)
                .unwrap_or(start_time);

            if request_start >= end_time {
                break;
            }

            debug!(start = %request_start, end = %end_time, "request");

            if first_req {
                first_req = false
            } else {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }

            let ksum = self
                .client
                .get_klines(
                    &target.symbol,
                    &target.interval.to_string(),
                    (request_start.unix_timestamp() * MILLI_SEC) as u64,
                    (end_time.unix_timestamp() * MILLI_SEC) as u64,
                )
                .unwrap_or_else(|e| panic!("get klines: {e}"));

            let KlineSummaries::AllKlineSummaries(klines) = ksum;

            let kcount = klines.len();
            if kcount == 0 {
                break;
            }

            let period_start = local_from_unix_timestamp_millis_truncated(
                "period_start",
                klines[0].open_time,
            )
            .and_then(|t| truncate("period_start", t, d))
            .unwrap();
            let period_end = local_from_unix_timestamp_millis_truncated(
                "period_end",
                klines[kcount - 1].open_time,
            )
            .and_then(|t| truncate("period_end", t, d))
            .unwrap();

            debug!(
                start = ?period_start,
                end = ?period_end,
                n = kcount,
                "loaded"
            );

            let converted = klines
                .into_iter()
                .map(|ks| {
                    let finalized = ks.close_time < next_period_start_millis;
                    kline_summary_to_k(
                        &target.symbol,
                        target.interval,
                        ks,
                        finalized,
                    )
                })
                .collect::<Result<Vec<_>>>()
                .expect("convert to kinfos");

            history_ks.extend(converted);
        }

        Ok(history_ks)
    }
}
