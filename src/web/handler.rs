use dioxus::prelude::*;
#[cfg(feature = "server")]
use {
    super::{AppCtx, Request},
    dioxus::fullstack::axum::Extension,
    tokio::sync::oneshot,
};

#[server(ctx: Extension<AppCtx>)]
pub async fn get_states() -> Result<Vec<String>> {
    let (resp_tx, resp_rx) = oneshot::channel();
    ctx.req_tx
        .send(Request::States(resp_tx))
        .context("send req via chan")?;
    let lines = resp_rx.await.context("recv resp from chan")?;
    Ok(lines)
}
