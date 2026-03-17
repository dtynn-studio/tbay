use std::path::Path;

use serde::Deserialize;

use crate::prelude::*;

#[derive(Deserialize)]
pub struct Config {
    pub pairs: Vec<Pair>,
}

#[derive(Deserialize)]
pub struct Pair {
    pub name: String,
    pub interval: String,
    pub monitors: Vec<String>,
}

pub fn load_config(p: impl AsRef<Path>) -> Result<Config> {
    let content = std::fs::read(p.as_ref()).with_context(|_| FileCtx {
        path: p.as_ref().to_path_buf(),
    })?;

    let cfg = toml::from_slice(&content)?;
    Ok(cfg)
}
