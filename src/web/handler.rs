use std::{collections::BTreeMap, time::Duration};

use dioxus::prelude::*;
#[cfg(feature = "server")]
use {
    super::serve::{AppCtx, Request},
    dioxus::{CapturedError, fullstack::axum::Extension},
    tokio::sync::oneshot,
};

const SYMBOL_BTCUSDT: &str = "btcusdt";
const SYMBOL_ETHUSDT: &str = "ethusdt";

#[server(ctx: Extension<AppCtx>)]
pub async fn get_states(load_states: bool) -> Result<Vec<String>> {
    let (resp_tx, resp_rx) = oneshot::channel();
    ctx.req_tx
        .send(Request::States(load_states, resp_tx))
        .context("send req via chan")?;
    let lines = resp_rx.await.context("recv resp from chan")?;
    Ok(lines)
}

pub async fn get_pairs() -> Result<BTreeMap<String, Vec<Duration>>> {
    let ds = vec![
        Duration::from_mins(3),
        Duration::from_mins(15),
        Duration::from_hours(1),
        Duration::from_hours(4),
        Duration::from_days(1),
    ];

    let pairs = BTreeMap::from_iter([
        (SYMBOL_ETHUSDT.to_owned(), ds.clone()),
        (SYMBOL_BTCUSDT.to_owned(), ds),
    ]);

    Ok(pairs)
}

#[server(ctx: Extension<AppCtx>)]
pub async fn add_monitor(
    symbol: String,
    d: Duration,
    key: String,
) -> Result<bool> {
    let (resp_tx, resp_rx) = oneshot::channel();
    let req = Request::Monitor(symbol, d, key, resp_tx);
    ctx.req_tx.send(req).context("send req via chan")?;
    let res = resp_rx.await.context("recv resp from chan")?;
    res.map_err(CapturedError::msg)
}
