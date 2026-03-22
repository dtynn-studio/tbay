#![allow(clippy::result_large_err)]

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};

use crossbeam_channel::bounded;
use futures_util::SinkExt;
use humantime::{parse_duration, Duration};
use tracing::{debug, warn_span};

use super::client::BinanceHttpClient;
use super::convert::{ContinuousKlineEvent, KlineEvent, KlineSummaries};
use super::proxy::ProxyConfig;
use super::ws::{build_subscribe_msg, run_event_loop, WsConnection};
use crate::{
    event::{DataSource, Event, EventChanTx, K, SubscribeStopper, Target},
    prelude::*,
    util::time::{
        MILLI_SEC, local_from_unix_timestamp_millis_truncated, truncate,
    },
};

// =============================================================================
// 数据转换
// =============================================================================

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

fn kline_event_to_k(event: KlineEvent) -> Result<K> {
    let kraw = KRaw {
        time_begin: local_from_unix_timestamp_millis_truncated(
            "time_begin",
            event.kline.open_time,
        )?,

        time_end: local_from_unix_timestamp_millis_truncated(
            "time_end",
            event.kline.close_time,
        )?,

        price_open: Decimal::from_str_radix(&event.kline.open, 10)
            .context(DecimalCtx {
                field: "price_open",
            })?,

        price_close: Decimal::from_str_radix(&event.kline.close, 10)
            .context(DecimalCtx {
                field: "price_close",
            })?,

        price_high: Decimal::from_str_radix(&event.kline.high, 10)
            .context(DecimalCtx {
                field: "price_high",
            })?,

        price_low: Decimal::from_str_radix(&event.kline.low, 10)
            .context(DecimalCtx { field: "price_low" })?,

        quantity: Decimal::from_str_radix(&event.kline.volume, 10)
            .context(DecimalCtx { field: "quantity" })?,

        trades: event.kline.number_of_trades,
        finalized: event.kline.is_final_bar,
    };

    Ok(K {
        symbol: event.kline.symbol.to_lowercase(),
        interval: parse_duration(&event.kline.interval)
            .context(ParseDurationCtx)?
            .into(),
        source: "k",
        raw: kraw,
    })
}

fn continuous_kline_event_to_k(event: ContinuousKlineEvent) -> Result<K> {
    let kraw = KRaw {
        time_begin: local_from_unix_timestamp_millis_truncated(
            "time_begin",
            event.kline.start_time,
        )?,

        time_end: local_from_unix_timestamp_millis_truncated(
            "time_end",
            event.kline.end_time,
        )?,

        price_open: Decimal::from_str_radix(&event.kline.open, 10)
            .context(DecimalCtx {
                field: "price_open",
            })?,

        price_close: Decimal::from_str_radix(&event.kline.close, 10)
            .context(DecimalCtx {
                field: "price_close",
            })?,

        price_high: Decimal::from_str_radix(&event.kline.high, 10)
            .context(DecimalCtx {
                field: "price_high",
            })?,

        price_low: Decimal::from_str_radix(&event.kline.low, 10)
            .context(DecimalCtx { field: "price_low" })?,

        quantity: Decimal::from_str_radix(&event.kline.volume, 10)
            .context(DecimalCtx { field: "quantity" })?,

        trades: event.kline.number_of_trades,
        finalized: event.kline.is_final_bar,
    };

    Ok(K {
        symbol: event.pair.to_lowercase(),
        interval: parse_duration(&event.kline.interval)
            .context(ParseDurationCtx)?
            .into(),
        source: "ck",
        raw: kraw,
    })
}

// =============================================================================
// BinanceDataSource (WebSocket)
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
    fn start(self, targets: Vec<Target>) -> Result<BinanceSubscriberStopper> {
        let (res_tx, res_rx) = bounded::<Result<()>>(1);
        let streams = targets
            .iter()
            .map(|t| t.bn_futures_key())
            .collect::<Vec<_>>();
        let perpetual_streams = targets
            .iter()
            .map(|t| t.bn_futures_perpetual_key())
            .collect::<Vec<_>>();
        let all_streams = streams
            .iter()
            .chain(perpetual_streams.iter())
            .cloned()
            .collect::<Vec<_>>();

        let event_tx = self.event_tx.clone();
        let proxy = self.proxy.clone();

        let handler = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();

            let connect_result = rt.block_on(async {
                let url = "wss://fstream.binance.com/ws";
                WsConnection::connect(&proxy, url).await
            });

            let (conn, mut ws) = match connect_result {
                Ok((conn, ws)) => {
                    _ = res_tx.send(Ok(()));
                    (conn, ws)
                }
                Err(e) => {
                    _ = res_tx.send(Err(e));
                    return;
                }
            };

            // 发送订阅消息
            let subscribe_msg = build_subscribe_msg(&all_streams);
            rt.block_on(async {
                if let Err(e) = ws.send(reqwest_websocket::Message::Text(subscribe_msg.into())).await {
                    tracing::warn!(?e, "subscribe failed");
                }
            });

            // 运行事件循环
            let event_tx_for_kline = event_tx.clone();
            let event_tx_for_continuous = event_tx.clone();
            rt.block_on(run_event_loop(
                ws,
                conn.running.clone(),
                move |evt| {
                    if let Ok(k) = kline_event_to_k(evt) {
                        _ = event_tx_for_kline.send(Event::K(k));
                    }
                    Ok(())
                },
                move |evt| {
                    if let Ok(k) = continuous_kline_event_to_k(evt) {
                        _ = event_tx_for_continuous.send(Event::K(k));
                    }
                    Ok(())
                },
            ));

            debug!("ws event loop ended");
        });

        res_rx.recv().map_err(|_| Error::Msg {
            reason: "result channel broken".into(),
        })??;

        Ok(BinanceSubscriberStopper {
            _handler: handler,
            running: Arc::new(AtomicBool::new(true)),
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

    pub fn with_proxy(
        testnet: bool,
        verbose: bool,
        proxy: ProxyConfig,
    ) -> Result<Self> {
        let client = BinanceHttpClient::new(proxy, testnet, verbose)?;
        Ok(Self { client })
    }

    pub fn load_history(&self, target: &Target, count: usize) -> Result<Vec<K>> {
        let _span =
            warn_span!("historical ks", sym = target.symbol, int = %target.interval).entered();
        let mut history_ks: Vec<K> = Vec::new();
        let mut first_req = true;
        let d = std::time::Duration::from(target.interval);
        let end_time = OffsetDateTime::now_local()?;
        let next_period_start_millis =
            (truncate("next period start", end_time, d)? + d).unix_timestamp() * MILLI_SEC;
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
                    kline_summary_to_k(&target.symbol, target.interval, ks, finalized)
                })
                .collect::<Result<Vec<_>>>()
                .expect("convert to kinfos");

            history_ks.extend(converted);
        }

        Ok(history_ks)
    }
}
