use crate::prelude::*;

pub mod alert;
pub mod cross;
pub mod hold;
pub mod rate;
pub mod read;
pub mod touch;

#[derive(Debug, Clone)]
pub struct Msg {
    pub normal: String,
    pub tty: String,
}

#[derive(Debug, Default)]
pub struct State {
    pub temp: Option<(OffsetDateTime, Msg)>,
    pub perm: Option<(OffsetDateTime, Msg)>,
}

pub trait Monitor {
    fn key(&self) -> &str;
    fn deps(&self) -> Vec<&str>;
    // fn calc(&self, kctx: &KCtx) -> Option<String>;
    // fn update(&mut self, kctx: &KCtx) -> Option<String>;
    fn apply(&mut self, kctx: &KCtx);
    fn state(&self) -> &State;
    fn take_alerts(&mut self) -> Vec<(OffsetDateTime, Msg)>;
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
