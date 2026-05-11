use std::time::Duration;

use dioxus::prelude::*;
#[cfg(feature = "server")]
use {
    super::serve::{AppCtx, Request},
    dioxus::{CapturedError, fullstack::axum::Extension},
    tokio::sync::oneshot,
};

use crate::web::types::StatesKind;

// const SYMBOL_BTCUSDT: &str = "btcusdt";
// const SYMBOL_ETHUSDT: &str = "ethusdt";

#[server(ctx: Extension<AppCtx>)]
pub async fn get_states(kind: StatesKind) -> Result<Vec<String>> {
    let (resp_tx, resp_rx) = oneshot::channel();
    ctx.req_tx
        .send(Request::States(kind, resp_tx))
        .context("send req via chan")?;
    let lines = resp_rx.await.context("recv resp from chan")?;
    Ok(lines)
}

#[server(ctx: Extension<AppCtx>)]
pub async fn get_pairs() -> Result<(Vec<String>, Vec<Duration>)> {
    let (resp_tx, resp_rx) = oneshot::channel();
    ctx.req_tx
        .send(Request::Pairs(resp_tx))
        .context("send req via chan")?;
    let pairs = resp_rx.await.context("recv resp from chan")?;
    Ok(pairs)
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
