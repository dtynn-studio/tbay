pub mod event;
pub mod hub;
pub mod indicator;
pub mod k;
pub mod monitor;
pub mod prelude;
pub mod res;
pub mod util;

use crate::prelude::Result;

pub trait Builder {
    type Target;
    fn build(&self, s: &str) -> Result<Self::Target>;
}
