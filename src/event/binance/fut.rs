#![allow(clippy::result_large_err)]

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};

use binance::{
    futures::websockets::{
        FuturesMarket, FuturesWebSockets, FuturesWebsocketEvent,
    },
    model::{ContinuousKlineEvent, KlineEvent},
};
use crossbeam_channel::bounded;
use humantime::parse_duration;
use tracing::debug;

use crate::{
    event::{DataSource, Event, EventChanTx, K, SubscribeStopper, Target},
    prelude::*,
    util::time::local_from_unix_timestamp_millis_truncated,
};

fn kline_event_to_k(event: KlineEvent) -> Result<K> {
    let kraw =
        KRaw {
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
    let kraw =
        KRaw {
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

pub struct BinanceDataSource {
    event_tx: EventChanTx,
}

impl DataSource for BinanceDataSource {
    fn new(event_tx: EventChanTx) -> Self {
        Self { event_tx }
    }

    fn start(self, targets: Vec<Target>) -> Result<impl SubscribeStopper> {
        let (res_tx, res_rx) = bounded(1);
        let running = Arc::new(AtomicBool::new(true));
        let streams = targets
            .iter()
            .map(|t| {
                format!("{}_perpetual@continuousKline_{}", t.symbol, t.interval)
            })
            .collect::<Vec<_>>();

        let running_spawned = running.clone();
        let event_tx = self.event_tx.clone();
        let handler = std::thread::spawn(move || {
            let running = running_spawned;
            let res_tx = res_tx;
            let streams = streams;
            let event_tx = event_tx.clone();

            debug!("socket new");
            let mut socket = FuturesWebSockets::new(move |event| {
                match event {
                    FuturesWebsocketEvent::Kline(evt) => {
                        if let Ok(k) = kline_event_to_k(evt) {
                            _ = event_tx.send(Event::K(k));
                        }
                    }

                    FuturesWebsocketEvent::ContinuousKline(evt) => {
                        if let Ok(k) = continuous_kline_event_to_k(evt) {
                            _ = event_tx.send(Event::K(k));
                        }
                    }

                    _ => {}
                }

                Ok(())
            });

            debug!("socket connect");
            match socket
                .connect_multiple_streams(&FuturesMarket::USDM, &streams)
            {
                Ok(_) => {}
                e @ Err(_) => {
                    _ = res_tx.send(e);
                    return;
                }
            }

            debug!("socket stablized");
            _ = res_tx.send(Ok(()));

            debug!("event loop start");
            _ = socket.event_loop(&running);

            _ = socket.disconnect();
        });

        res_rx.recv().map_err(|_e| Error::Msg {
            reason: "result channel broken".into(),
        })??;

        Ok(BinanceSubscriberStopper {
            _handler: handler,
            running,
        })
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
