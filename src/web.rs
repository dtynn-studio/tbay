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

use crate::prelude::{NetworkCtx, Result, ResultExt};

#[derive(Clone)]
pub struct AppCtx {
    pub req_tx: mpsc::UnboundedSender<Request>,
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

pub enum Request {
    States(oneshot::Sender<Result<Vec<String>, String>>),
}

#[cfg(feature = "server")]
pub async fn serve(req_tx: mpsc::UnboundedSender<Request>) -> Result<()> {
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
