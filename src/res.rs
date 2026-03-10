use std::{borrow::Cow, fmt::Display};

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
    Msg { reason: Cow<'static, str> },
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

pub mod prelude {
    pub use snafu::prelude::*;

    pub use super::*;
}
