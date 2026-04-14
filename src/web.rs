use dioxus::prelude::*;

pub mod handler;
#[cfg(feature = "server")]
pub mod serve;

#[component]
pub fn App() -> Element {
    let mut state_resource = use_resource(handler::get_states);

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

                match &*state_resource.read() {
                    Some(Ok(lines)) => {
                        rsx! {
                            for line in lines {
                                p {
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
                class: "h-14 flex-shrink-0 bg-white border-t border-gray-200 flex items-center",

                onclick: move |_| { state_resource.restart(); },

                "Refresh"
            }

            div {
                class: "h-14 flex-shrink-0 bg-white border-t border-gray-200 flex items-center",

                "Footer"
            }
        }
    }
}
