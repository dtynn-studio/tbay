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

macro_rules! impl_builder {
    ($name:ident: $args:ident => $target:ident) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $name;

        impl Builder for $name {
            type Target = $target;

            fn build(&self, s: &str) -> Result<Self::Target> {
                let args: $args = s.parse()?;
                args.build()
            }
        }
    };
}

use impl_builder;
