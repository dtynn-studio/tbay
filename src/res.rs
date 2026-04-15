use std::{borrow::Cow, fmt::Display, path::PathBuf};

use snafu::Snafu;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("parse `{raw}` for {usage}"))]
    Parse {
        raw: String,
        usage: Cow<'static, str>,
        source: std::io::Error,
    },

    #[snafu(display("{reason}"))]
    Msg {
        reason: Cow<'static, str>,
    },

    #[snafu(display("datetime {field}: {source}"))]
    Datetime {
        field: &'static str,
        source: time::error::ComponentRange,
    },

    DatetimeOffset {
        source: time::error::IndeterminateOffset,
    },

    #[snafu(display("decimal {field}: {source}"))]
    Decimal {
        field: &'static str,
        source: rust_decimal::Error,
    },

    #[snafu(display("file {path:?}: {source}"))]
    File {
        path: PathBuf,
        source: std::io::Error,
    },

    TomlDe {
        source: toml::de::Error,
    },

    Signal {
        source: ctrlc::Error,
    },

    ParseDuration {
        source: humantime::DurationError,
    },

    Reqwest {
        source: reqwest::Error,
    },

    Websocket {
        source: reqwest_websocket::Error,
    },

    Json {
        source: serde_json::Error,
    },

    Term {
        source: std::io::Error,
    },

    Execute {
        source: std::io::Error,
    },

    Network {
        source: std::io::Error,
    },

    DingTalk {
        source: dingtalk_sdk::Error,
    },
}

impl From<time::error::IndeterminateOffset> for Error {
    fn from(source: time::error::IndeterminateOffset) -> Self {
        Error::DatetimeOffset { source }
    }
}

impl From<toml::de::Error> for Error {
    fn from(value: toml::de::Error) -> Self {
        Error::TomlDe { source: value }
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Error::Reqwest { source: value }
    }
}

impl From<reqwest_websocket::Error> for Error {
    fn from(value: reqwest_websocket::Error) -> Self {
        Error::Websocket { source: value }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Json { source: value }
    }
}

pub trait Unexpected<T> {
    fn unexpected(self, msg: &str) -> Error;
}

impl<T: Display> Unexpected<T> for T {
    fn unexpected(self, usage: &str) -> Error {
        Error::Msg {
            reason: format!(
                "unexpected value {self} of type {}: {usage}",
                std::any::type_name::<T>()
            )
            .into(),
        }
    }
}

pub trait Required<T> {
    fn required(self, usage: &str) -> Result<T>;
}

impl<T> Required<T> for Option<T> {
    fn required(self, usage: &str) -> Result<T> {
        self.ok_or_else(|| Error::Msg {
            reason: format!(
                "{usage}: a value of {} is required",
                std::any::type_name::<T>()
            )
            .into(),
        })
    }
}

pub mod prelude {
    pub use snafu::prelude::*;

    pub use super::*;
}
