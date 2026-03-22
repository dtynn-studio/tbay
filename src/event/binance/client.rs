use std::collections::BTreeMap;

use reqwest::{Client, Proxy};
use reqwest_websocket::{Upgrade, WebSocket};
use serde::de::DeserializeOwned;

use crate::prelude::*;

const FUTURES_MAINNET: &str = "https://fapi.binance.com";
const FUTURES_TESTNET: &str = "https://testnet.binancefuture.com";

const FUTURES_WS_MAINNET: &str = "wss://fstream.binance.com";
const FUTURES_WS_TESTNET: &str = "wss://fstream.binancefuture.com";

pub struct Config {
    pub testnet: bool,
    pub proxy: Option<String>,
}

pub struct Query(BTreeMap<&'static str, String>);

impl Query {
    pub fn add(&mut self, key: &'static str, val: impl ToString) {
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

    pub async fn get_json<T: DeserializeOwned>(
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

    pub async fn connect_ws(
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
