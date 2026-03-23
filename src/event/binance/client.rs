use std::collections::BTreeMap;

use futures_util::{SinkExt, StreamExt};
use reqwest::{Client, Proxy};
use reqwest_websocket::{CloseCode, Message, Upgrade, WebSocket};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, from_value};
use tokio::{
    select,
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};
use tracing::trace;

use crate::prelude::*;

const FUTURES_MAINNET: &str = "https://fapi.binance.com";
const FUTURES_TESTNET: &str = "https://testnet.binancefuture.com";

const FUTURES_WS_MAINNET: &str = "wss://fstream.binance.com";
const FUTURES_WS_TESTNET: &str = "wss://fstream.binancefuture.com";

pub struct Config {
    pub testnet: bool,
    pub proxy: Option<String>,
}

#[derive(Default)]
pub struct Query(BTreeMap<&'static str, String>);

impl Query {
    pub fn set(&mut self, key: &'static str, val: impl ToString) {
        self.0.insert(key, val.to_string());
    }

    pub fn build(&self) -> String {
        let mut query = String::new();
        for (k, v) in self.0.iter() {
            if !query.is_empty() {
                query.push('&');
            }

            query.push_str(k);
            query.push('=');
            query.push_str(v);
        }

        query
    }
}

#[derive(Clone)]
pub struct BnClient {
    client: Client,
    api_base_url: &'static str,
    ws_base_url: &'static str,
}

impl BnClient {
    pub fn new(cfg: Config) -> Result<Self> {
        let mut builder = Client::builder();
        if let Some(purl) = cfg.proxy {
            let proxy = Proxy::all(purl)?;
            builder = builder.proxy(proxy);
        }

        let client = builder.build()?;

        let (api_base_url, ws_base_url) = if cfg.testnet {
            (FUTURES_TESTNET, FUTURES_WS_TESTNET)
        } else {
            (FUTURES_MAINNET, FUTURES_WS_MAINNET)
        };

        Ok(Self {
            client,
            api_base_url,
            ws_base_url,
        })
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: Option<Query>,
    ) -> Result<T> {
        let url = if let Some(q) = query.map(|q| q.build()) {
            format!("{}{path}?{q}", self.api_base_url)
        } else {
            format!("{}{path}", self.api_base_url)
        };

        let resp = self.client.get(url).send().await?.error_for_status()?;
        resp.json().await.map_err(From::from)
    }

    async fn connect_ws(
        &self,
        path: &str,
        query: Option<Query>,
    ) -> Result<WebSocket> {
        let url = if let Some(q) = query.map(|q| q.build()) {
            format!("{}{path}?{q}", self.ws_base_url)
        } else {
            format!("{}{path}", self.ws_base_url)
        };

        let resp = self.client.get(url).upgrade().send().await?;
        resp.into_websocket().await.map_err(From::from)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KlineSummary {
    pub open_time: i64,

    pub open: String,

    pub high: String,

    pub low: String,

    pub close: String,

    pub volume: String,

    pub close_time: i64,

    pub quote_asset_volume: String,

    pub number_of_trades: i64,

    pub taker_buy_base_asset_volume: String,

    pub taker_buy_quote_asset_volume: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum KlineSummaries {
    AllKlineSummaries(Vec<KlineSummary>),
}

fn get_value(row: &[Value], index: usize, name: &'static str) -> Result<Value> {
    row.get(index).required(name).cloned()
}

impl TryFrom<&Vec<Value>> for KlineSummary {
    type Error = Error;

    fn try_from(row: &Vec<Value>) -> Result<Self> {
        Ok(Self {
            open_time: from_value(get_value(row, 0, "open_time")?)?,
            open: from_value(get_value(row, 1, "open")?)?,
            high: from_value(get_value(row, 2, "high")?)?,
            low: from_value(get_value(row, 3, "low")?)?,
            close: from_value(get_value(row, 4, "close")?)?,
            volume: from_value(get_value(row, 5, "volume")?)?,
            close_time: from_value(get_value(row, 6, "close_time")?)?,
            quote_asset_volume: from_value(get_value(
                row,
                7,
                "quote_asset_volume",
            )?)?,
            number_of_trades: from_value(get_value(
                row,
                8,
                "number_of_trades",
            )?)?,
            taker_buy_base_asset_volume: from_value(get_value(
                row,
                9,
                "taker_buy_base_asset_volume",
            )?)?,
            taker_buy_quote_asset_volume: from_value(get_value(
                row,
                10,
                "taker_buy_quote_asset_volume",
            )?)?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Kline {
    #[serde(rename = "t")]
    pub open_time: i64,

    #[serde(rename = "T")]
    pub close_time: i64,

    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "i")]
    pub interval: String,

    #[serde(rename = "f")]
    pub first_trade_id: i64,

    #[serde(rename = "L")]
    pub last_trade_id: i64,

    #[serde(rename = "o")]
    pub open: String,

    #[serde(rename = "c")]
    pub close: String,

    #[serde(rename = "h")]
    pub high: String,

    #[serde(rename = "l")]
    pub low: String,

    #[serde(rename = "v")]
    pub volume: String,

    #[serde(rename = "n")]
    pub number_of_trades: i64,

    #[serde(rename = "x")]
    pub is_final_bar: bool,

    #[serde(rename = "q")]
    pub quote_asset_volume: String,

    #[serde(rename = "V")]
    pub taker_buy_base_asset_volume: String,

    #[serde(rename = "Q")]
    pub taker_buy_quote_asset_volume: String,

    #[serde(skip, rename = "B")]
    pub ignore_me: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContinuousKline {
    #[serde(rename = "t")]
    pub start_time: i64,

    #[serde(rename = "T")]
    pub end_time: i64,

    #[serde(rename = "i")]
    pub interval: String,

    #[serde(rename = "f")]
    pub first_trade_id: i64,

    #[serde(rename = "L")]
    pub last_trade_id: i64,

    #[serde(rename = "o")]
    pub open: String,

    #[serde(rename = "c")]
    pub close: String,

    #[serde(rename = "h")]
    pub high: String,

    #[serde(rename = "l")]
    pub low: String,

    #[serde(rename = "v")]
    pub volume: String,

    #[serde(rename = "n")]
    pub number_of_trades: i64,

    #[serde(rename = "x")]
    pub is_final_bar: bool,

    #[serde(rename = "q")]
    pub quote_volume: String,

    #[serde(rename = "V")]
    pub active_buy_volume: String,

    #[serde(rename = "Q")]
    pub active_volume_buy_quote: String,

    #[serde(skip, rename = "B")]
    pub ignore_me: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KlineEvent {
    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "k")]
    pub kline: Kline,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContinuousKlineEvent {
    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "ps")]
    pub pair: String,

    #[serde(rename = "ct")]
    pub contract_type: String,

    #[serde(rename = "k")]
    pub kline: ContinuousKline,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum FuturesEvents {
    KlineEvent(KlineEvent),
    ContinuousKlineEvent(ContinuousKlineEvent),
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FuturesWebsocketEvent {
    Kline(KlineEvent),
    ContinuousKline(ContinuousKlineEvent),
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum WebsocketEvent {
    Futures(FuturesWebsocketEvent),
    Disconnect { code: CloseCode, reason: String },
    Broken,
}

pub enum WebsocketControl {
    Disconnect,
}

fn parse_ws_event(msg: &str) -> Result<FuturesWebsocketEvent> {
    let value: Value = serde_json::from_str(msg)?;

    if let Some(data) = value.get("data") {
        return parse_ws_event(&data.to_string());
    }

    let event: FuturesEvents = from_value(value)?;
    Ok(match event {
        FuturesEvents::KlineEvent(k) => FuturesWebsocketEvent::Kline(k),
        FuturesEvents::ContinuousKlineEvent(k) => {
            FuturesWebsocketEvent::ContinuousKline(k)
        }
    })
}

pub struct EventStream {}
pub struct EventStreamStopper {}

impl BnClient {
    pub fn get_klines(
        &self,
        symbol: String,
        interval: String,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> Result<KlineSummaries> {
        let mut query = Query::default();
        query.set("symbol", symbol);
        query.set("interval", interval);

        if let Some(t) = start_time {
            query.set("startTime", t);
        }

        if let Some(t) = end_time {
            query.set("endTime", t);
        }

        let data: Vec<Vec<Value>> = tokio::runtime::Handle::current()
            .block_on(async {
                self.get_json("/api/v3/klines", Some(query)).await
            })?;

        let klines = KlineSummaries::AllKlineSummaries(
            data.iter()
                .map(|row| row.try_into())
                .collect::<Result<Vec<KlineSummary>>>()?,
        );

        Ok(klines)
    }

    pub fn subscribe_klines(
        &self,
        targets: &[Target],
    ) -> Result<(
        UnboundedReceiver<WebsocketEvent>,
        UnboundedSender<WebsocketControl>,
    )> {
        let streams = targets
            .iter()
            .map(|t| t.bn_futures_key())
            .collect::<Vec<_>>()
            .join("/");
        let mut query = Query::default();
        query.set("streams", streams);

        let mut ws = tokio::runtime::Handle::current().block_on(async {
            self.connect_ws("/stream", Some(query)).await
        })?;

        let (event_tx, event_rx) = unbounded_channel();
        let (ctrl_tx, mut ctrl_rx) = unbounded_channel();

        tokio::spawn(async move {
            let mut broken_reason = None;

            'event_loop: loop {
                select! {
                    msg_opt = ws.next() => {
                        let Some(Ok(msg)) = msg_opt else {
                            break;
                        };

                        match msg {
                            Message::Text(txt) => {
                                match parse_ws_event(&txt) {
                                    Ok(msg) => {
                                        if let Err(_e) = event_tx.send(WebsocketEvent::Futures(msg)) {
                                            broken_reason.replace("send futures event: chan broken".to_owned());
                                            break 'event_loop;
                                            // TODO handle error
                                        }
                                    },
                                    Err(e) => {
                                        // TODO handle error
                                        trace!("parse ws event: {e}");
                                    },
                                }
                            },


                            Message::Ping(data) => {
                                if let Err(e) = ws.send(Message::Pong(data)).await {
                                    // TODO: handle pong error
                                    broken_reason.replace(format!("send pong: {e}"));
                                    break 'event_loop;
                                };
                            },

                            Message::Pong(_data) => {},
                            Message::Binary(_data) => {},

                            Message::Close{ code, reason } => {
                                trace!(%code, reason, "ws stream closed");
                                if let Err(_e) = event_tx.send(WebsocketEvent::Disconnect{ code, reason }) {
                                    // TODO: handle error
                                    broken_reason.replace("send diconnect event: chan broken".to_owned());
                                }

                                break 'event_loop;
                            },
                        }
                    },

                    _ = ctrl_rx.recv() => {
                        break;
                    },
                }
            }

            if let Some(reason) = broken_reason {
                trace!("event loop broken: {reason}");
                _ = event_tx.send(WebsocketEvent::Broken);
            }
        });

        Ok((event_rx, ctrl_tx))
    }
}
