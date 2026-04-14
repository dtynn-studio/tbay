use dioxus::prelude::*;
#[cfg(feature = "server")]
use {
    super::serve::{AppCtx, Request},
    dioxus::fullstack::axum::Extension,
    tokio::sync::oneshot,
};

#[server(ctx: Extension<AppCtx>)]
pub async fn get_states(load_states: bool) -> Result<Vec<String>> {
    let (resp_tx, resp_rx) = oneshot::channel();
    ctx.req_tx
        .send(Request::States(load_states, resp_tx))
        .context("send req via chan")?;
    let lines = resp_rx.await.context("recv resp from chan")?;
    Ok(lines)
}
