use std::time::Duration;

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

pub async fn get_pairs() -> Result<(Vec<&'static str>, Vec<Duration>)> {
    let symbols = vec![SYMBOL_ETHUSDT, SYMBOL_BTCUSDT];

    let intervals = vec![
        Duration::from_mins(3),
        Duration::from_mins(15),
        Duration::from_hours(1),
        Duration::from_hours(4),
        Duration::from_days(1),
    ];

    Ok((symbols, intervals))
}

#[server(ctx: Extension<AppCtx>)]
pub async fn add_once_monitor(
    symbol: String,
    interval: Duration,
    key: String,
) -> Result<bool> {
    let (resp_tx, resp_rx) = oneshot::channel();
    let req = Request::OnceMonitor(symbol, interval, key, resp_tx);
    ctx.req_tx.send(req).context("send req via chan")?;
    let res = resp_rx.await.context("recv resp from chan")?;
    res.map_err(CapturedError::msg)
}

#[server(ctx: Extension<AppCtx>)]
pub async fn remove_once_monitors() -> Result<usize> {
    let (resp_tx, resp_rx) = oneshot::channel();
    let req = Request::RemoveOnce(resp_tx);
    ctx.req_tx.send(req).context("send req via chan")?;
    let count = resp_rx.await.context("recv resp from chan")?;
    Ok(count)
}
