use crate::prelude::*;

#[derive(Clone, Copy)]
pub struct BurstArgs {}

impl FromStr for BurstArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        unimplemented!()
    }
}

impl Args for BurstArgs {
    type Type = ();

    type Target = Burst;

    fn new(args: Self::Type) -> Self {
        unimplemented!()
    }

    fn key(&self) -> String {
        unimplemented!()
    }

    fn build(self) -> Result<Self::Target> {
        unimplemented!()
    }
}

pub struct Burst {}

impl Monitor for Burst {
    fn key(&self) -> &str {
        unimplemented!()
    }

    fn deps(&self) -> Vec<&str> {
        unimplemented!()
    }

    fn apply(&mut self, kctx: &KCtx) {
        unimplemented!()
    }

    fn state(&self) -> &State {
        unimplemented!()
    }

    fn take_alerts(&mut self) -> Vec<(OffsetDateTime, Msg)> {
        unimplemented!()
    }

    fn terminated(&self) -> bool {
        false
    }

    fn is_once(&self) -> bool {
        false
    }
}
