use std::str::FromStr;

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
        #[derive(Debug, Clone, Copy, Default)]
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

pub(crate) use impl_builder;
