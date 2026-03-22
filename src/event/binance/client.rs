use std::collections::BTreeMap;

use reqwest::{Client, Proxy};
use reqwest_websocket::{Upgrade, WebSocket};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, from_value};

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
}
