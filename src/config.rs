use std::{path::Path, time::Duration};

use serde::Deserialize;

use crate::prelude::*;

#[derive(Deserialize)]
pub struct Config {
    pub symbols: Vec<Symbol>,
}

#[derive(Deserialize)]
pub struct Symbol {
    pub name: String,
    pub intervals: Vec<Interval>,
}

#[derive(Deserialize)]
pub struct Interval {
    #[serde(with = "humantime_serde")]
    pub name: Duration,
    pub monitors: Vec<String>,
}

pub fn load_config(p: impl AsRef<Path>) -> Result<Config> {
    let content = std::fs::read(p.as_ref()).with_context(|_| FileCtx {
        path: p.as_ref().to_path_buf(),
    })?;

    let cfg = toml::from_slice(&content)?;
    Ok(cfg)
}
