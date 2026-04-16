use std::time::Duration;

use dioxus::{
    cli_config::fullstack_address_or_localhost,
    prelude::*,
    server::{
        ServeConfig,
        axum::{Extension, Router},
    },
};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
};

use super::App;

#[derive(Clone)]
pub struct AppCtx {
    pub req_tx: mpsc::UnboundedSender<Request>,
}

pub enum Request {
    States(bool, oneshot::Sender<Vec<String>>),
    OnceMonitor(
        String,
        Duration,
        String,
        oneshot::Sender<Result<bool, String>>,
    ),
    RemoveOnce(oneshot::Sender<usize>),
}

pub async fn serve(req_tx: mpsc::UnboundedSender<Request>) -> Result<()> {
    let listen = fullstack_address_or_localhost();
    let listener = TcpListener::bind(listen).await?;

    let ctx = AppCtx { req_tx };

    let router = Router::new()
        .serve_dioxus_application(ServeConfig::new(), App)
        .layer(Extension(ctx));

    dioxus::fullstack::axum::serve(listener, router.into_make_service())
        .await?;

    Ok(())
}
