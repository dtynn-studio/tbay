use std::str::FromStr;

pub mod event;
pub mod hub;
pub mod indicator;
pub mod k;
pub mod monitor;
pub mod prelude;
pub mod res;
pub mod util;

use crate::prelude::{Error, Result};

pub trait Builder {
    type Target;
    fn build(&self, s: &str) -> Result<Self::Target>;
}

pub trait Args: Clone + FromStr<Err = Error> {
    type Type;
    type Target;

    fn new(args: Self::Type) -> Self;

    fn key(&self) -> String;

    fn build(self) -> Result<Self::Target>;
}
