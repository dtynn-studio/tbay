use std::time::Duration;

use dioxus::prelude::*;

pub mod components;
pub mod handler;
#[cfg(feature = "server")]
pub mod serve;

use components::{
    dropdown_menu::{
        DropdownMenu, DropdownMenuContent, DropdownMenuItem,
        DropdownMenuTrigger,
    },
    sheet::{
        Sheet, SheetClose, SheetContent, SheetFooter, SheetHeader, SheetSide,
        SheetTitle,
    },
};

#[component]
pub fn App() -> Element {
    let mut load_states = use_signal(|| true);
    let state_resource = use_resource(move || async move {
        let is_states = *load_states.read();
        handler::get_states(is_states).await
    });

    let mut add_sheet_show = use_signal(|| false);

    let pairs_resource = use_resource(handler::get_pairs);

    let pairs_loaded =
        || matches!(&*pairs_resource.value().read(), Some(Ok(_pairs)));

    // add monitor
    let mut add_sheet_symbol = use_signal(|| "".to_owned());
    let mut add_sheet_interval = use_signal(|| Option::<Duration>::None);
    let mut add_sheet_key = use_signal(|| "".to_owned());

    let available_for_add_sheet = || {
        !add_sheet_symbol.read().trim().is_empty()
            && !add_sheet_key.read().trim().is_empty()
            && add_sheet_interval.read().is_some()
    };

    let mut add_monitor_action = use_action(handler::add_monitor);

    let mut add_sheet_msg = use_signal(|| Option::<(String, bool)>::None);

    let mut reset_add_sheet_infos = move || {
        add_sheet_symbol.set("".to_owned());
        add_sheet_interval.take();
        add_sheet_key.set("".to_owned());
    };

    use_effect(move || {
        let Some(res) = add_monitor_action.value() else {
            return;
        };

        match res {
            Ok(added) => {
                if *added.read() {
                    add_sheet_msg.set(Some(("已添加".to_owned(), false)));
                } else {
                    add_sheet_msg.set(Some(("已存在".to_owned(), false)));
                }

                reset_add_sheet_infos();
            }
            Err(e) => {
                add_sheet_msg.set(Some((e.to_string(), true)));
            }
        }
    });

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }
        Stylesheet { href: asset!("/assets/dx-components-theme.css") }

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
                    onclick: move |_| {
                        add_sheet_show.set(true);
                    },

                    disabled: !pairs_loaded(),

                    class: "flex-1 text-center",
                    "添加"
                }
            }
        }

        Sheet {
            open: add_sheet_show(),
            on_open_change: move |v: bool| {
                add_sheet_show.set(v);

                if !v {
                    reset_add_sheet_infos();
                }

            },
            SheetContent {
                side: SheetSide::Top,

                SheetHeader {
                    SheetTitle {
                        "添加监控",
                    }
                },

                div {
                    class: "flex flex-col justify-center items-center px-2",

                    DropdownMenu {
                        class: "dropdown-menu flex-1 px-3 pb-2 w-full",
                        default_open: false,
                        disabled: !pairs_loaded(),

                        DropdownMenuTrigger {
                            class: "dropdown-menu-trigger w-full",
                            if add_sheet_symbol.read().is_empty() {
                                "标的"
                            } else {
                                {add_sheet_symbol.read().to_owned()}
                            }
                        }

                        DropdownMenuContent {
                            class: "dropdown-menu-content",

                            if let Some(Ok((symbols, _intervals))) = &*pairs_resource.value().read() {
                                for (i, symbol) in symbols.iter().enumerate() {
                                    DropdownMenuItem::<String> {
                                        class: "dropdown-menu-item",
                                        value: symbol.to_owned(),
                                        index: i,
                                        on_select: move |value: String| {
                                            add_sheet_msg.set(None);
                                            add_sheet_symbol.set(value);
                                        },
                                        {symbol.to_owned()}
                                    }
                                }
                            }
                        }
                    }

                    DropdownMenu {
                        class: "dropdown-menu flex-1 px-3 pb-2 w-full",
                        default_open: false,
                        disabled: !pairs_loaded(),

                        DropdownMenuTrigger {
                            class: "dropdown-menu-trigger w-full",

                            if let Some(d) = &*add_sheet_interval.read() {
                                {humantime::Duration::from(*d).to_string()}
                            } else {
                                "周期"
                            }
                        }

                        DropdownMenuContent {
                            class: "dropdown-menu-content",

                            if let Some(Ok((_symbols, intervals))) = &*pairs_resource.value().read() {
                                for (i, interval) in intervals.iter().enumerate() {
                                    DropdownMenuItem::<Duration> {
                                        class: "dropdown-menu-item",
                                        value: *interval,
                                        index: i,
                                        on_select: move |value: Duration| {
                                            add_sheet_msg.set(None);
                                            add_sheet_interval.set(Some(value));
                                        },
                                        {humantime::Duration::from(*interval).to_string()}
                                    }
                                }
                            }
                        }
                    }

                    div {
                        class: "flex-1 w-full px-3 pb-2",
                        input {
                            class: "w-full px-1 h-10 border border-gray-300 rounded-lg focus:outline-none focus:border-blue-500",
                            value: "{add_sheet_key}",
                            oninput: move |evt| {
                                add_sheet_msg.set(None);
                                add_sheet_key.set(evt.value());
                            },
                            placeholder: "Monitor Key",
                        }
                    }

                    p {
                        style: "white-space: pre-wrap; word-wrap: break-word;",
                        class: "flex-1 px-4",
                        class: if let Some((_,true)) = add_sheet_msg.read().as_ref() {
                            "text-red"
                        } else {
                            "text-black"
                        },

                        if let Some((msg, _)) = add_sheet_msg.read().as_ref() {
                            {msg.to_owned()}
                        }
                    }
                },

                SheetFooter {
                    class: "flex h-10 pb-4",

                    button {
                        class: "flex-1 bg-blue-500 text-white disabled:bg-blue-300 disabled:text-white-400",
                        disabled: !available_for_add_sheet(),
                        onclick: move |_| {
                            let Some(interval) = add_sheet_interval.read().as_ref().copied() else {
                                return;
                            };

                            let symbol = add_sheet_symbol.read().trim().to_owned();
                            if symbol.is_empty() {
                                return;
                            }

                            let key = add_sheet_key.read().trim().to_owned();
                            if key.is_empty() {
                                return;
                            }

                            add_monitor_action.call(symbol, interval, key);
                        },
                        "提交"
                    }

                    SheetClose {
                        class: "flex-1",
                        "取消"
                    }
                },
            },

        }
    }
}
