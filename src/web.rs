use dioxus::{
    cli_config::fullstack_address_or_localhost,
    prelude::*,
    server::{ServeConfig, axum::Router},
};
use tokio::net::TcpListener;

use crate::prelude::{NetworkCtx, Result, ResultExt};

#[component]
pub fn App() -> Element {
    rsx! {
        "TBay"
    }
}

#[cfg(feature = "server")]
pub async fn serve() -> Result<()> {
    let listen = fullstack_address_or_localhost();
    let listener =
        ResultExt::context(TcpListener::bind(listen).await, NetworkCtx)?;

    let router =
        Router::new().serve_dioxus_application(ServeConfig::new(), App);

    ResultExt::context(
        dioxus::fullstack::axum::serve(listener, router.into_make_service())
        // .with_graceful_shutdown(shutdown())
        .await,
        NetworkCtx,
    )?;

    Ok(())
}
