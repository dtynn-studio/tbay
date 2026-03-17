use crate::prelude::*;

pub mod cross;

pub trait Monitor {
    fn key(&self) -> &str;
    fn deps(&self) -> Vec<&str>;
    fn calc(&self, kctx: &KCtx) -> Option<String>;
    fn update(&mut self, kctx: &KCtx) -> Option<String>;
    fn terminated(&self) -> bool;
}

pub struct BuilderAny<B: Builder> {
    inner: B,
}

impl<B: Builder> BuilderAny<B> {
    pub fn wrap(inner: B) -> Self {
        Self { inner }
    }
}

impl<B: Builder<Target: Monitor> + 'static> Builder for BuilderAny<B> {
    type Target = Box<dyn Monitor>;

    fn build(&self, s: &str) -> Result<Self::Target> {
        let raw = self.inner.build(s)?;
        Ok(Box::new(raw))
    }
}
