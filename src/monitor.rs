use crate::prelude::*;

pub mod alert;
pub mod bb;
pub mod burst;
pub mod cross;
pub mod cross2;
pub mod fbq;
pub mod hold;
pub mod hold2;
pub mod macd;
pub mod pdiff;
pub mod rate;
pub mod reach;
pub mod read;
pub mod rsi;
pub mod shadow;
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

impl State {
    pub fn clear(&mut self) {
        self.temp.take();
        self.perm.take();
    }
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

    fn is_once(&self) -> bool;
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
