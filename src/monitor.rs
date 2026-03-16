use crate::prelude::*;

pub trait Monitor {
    fn key(&self) -> &str;
    fn deps(&self) -> Vec<&str>;
    fn calc(&self, kctx: &KCtx);
    fn update(&mut self, kctx: &KCtx) -> bool;
}
