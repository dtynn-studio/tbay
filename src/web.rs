use std::borrow::Cow;

use dioxus::{
    cli_config::fullstack_address_or_localhost,
    prelude::*,
    server::{
        ServeConfig,
        axum::{Extension, Router},
    },
};
use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
};

use crate::prelude::{NetworkCtx, Result, ResultExt};

#[derive(Clone)]
pub struct AppCtx {
    pub req_tx: mpsc::UnboundedSender<(Request, oneshot::Sender<Response>)>,
}

#[component]
pub fn App() -> Element {
    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        meta {
            charset: "utf-8",
        }

        meta {
            name: "viewport",
            content: "width=device-width, initial-scale=1.0",
        },

        div {
            class: "h-screen w-full bg-gray-50 flex flex-col",

            main {
                class: "flex-1 overflow-hidden",

                "Mian"
            }

            div {
                class: "h-14 flex-shrink-0 bg-white border-t border-gray-200 flex items-center",

                "Footer"
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Request {
    States,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub err: Cow<'static, str>,
    pub data: Cow<'static, str>,
}

impl Response {
    pub fn new(data: impl Into<Cow<'static, str>>) -> Self {
        Self {
            err: "".into(),
            data: data.into(),
        }
    }

    pub fn err(err: impl Into<Cow<'static, str>>) -> Self {
        Self {
            err: err.into(),
            data: "".into(),
        }
    }
}

#[cfg(feature = "server")]
pub async fn serve(
    req_tx: mpsc::UnboundedSender<(Request, oneshot::Sender<Response>)>,
) -> Result<()> {
    let listen = fullstack_address_or_localhost();
    let listener =
        ResultExt::context(TcpListener::bind(listen).await, NetworkCtx)?;

    let ctx = AppCtx { req_tx };

    let router = Router::new()
        .serve_dioxus_application(ServeConfig::new(), App)
        .layer(Extension(ctx));

    ResultExt::context(
        dioxus::fullstack::axum::serve(listener, router.into_make_service())
        // .with_graceful_shutdown(shutdown())
        .await,
        NetworkCtx,
    )?;

    Ok(())
}
