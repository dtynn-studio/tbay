use dioxus::prelude::*;

pub mod handler;
#[cfg(feature = "server")]
pub mod serve;

#[component]
pub fn App() -> Element {
    let mut load_states = use_signal(|| true);
    let state_resource = use_resource(move || async move {
        let is_states = *load_states.read();
        handler::get_states(is_states).await
    });

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
                class: "flex-1 overflow-x-hidden overflow-y-auto",

                match &*state_resource.read() {
                    Some(Ok(lines)) => {
                        rsx! {
                            for line in lines {
                                p {
                                    style: "white-space: pre-wrap; word-wrap: break-word;",
                                    "{line}"
                                }
                            }
                        }
                    },

                    Some(Err(e)) => {
                        rsx! {
                            p {
                                "Error:",
                                span {
                                    "{e}"
                                },
                            }
                        }
                    },
                    None => {
                        rsx! {
                            p { "Loading" }
                        }
                    }
                }
            }

            div {
                class: "h-14 flex-shrink-0 bg-white border-t border-gray-200 flex",

                button {
                    onclick: move |_| {
                        let is_states = *load_states.read();
                        if !is_states {
                            *load_states.write() = true;
                        }
                    },
                    class: "flex-1 text-center",
                    "状态"
                }

                button {
                    onclick: move |_| {
                        let is_states = *load_states.read();
                        if is_states {
                            *load_states.write() = false;
                        }
                    },
                    class: "flex-1 text-center",
                    "均线"
                }

                button {
                    class: "flex-1 text-center",
                    "添加"
                }
            }
        }
    }
}
