use std::{path::Path, time::Duration};

use serde::Deserialize;

use crate::prelude::*;

#[derive(Deserialize)]
pub struct Config {
    pub pairs: Vec<Pair>,

    #[serde(default)]
    pub notify: Vec<Notify>,
}

#[derive(Deserialize)]
pub struct Pair {
    pub name: String,
    #[serde(default)]
    pub intervals: Vec<Interval>,
}

#[derive(Deserialize)]
pub struct Interval {
    #[serde(with = "humantime_serde")]
    #[serde(default)]
    pub name: Option<Duration>,
    #[serde(default)]
    pub monitors: Vec<String>,
}

#[derive(Deserialize)]
pub struct NotifyCmdArgs {
    pub bin: String,
    pub args: Vec<String>,
}

#[derive(Deserialize)]
pub struct DingTalkArgs {
    pub token: String,
    #[serde(default)]
    pub secret: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(tag = "type")]
pub enum Notify {
    #[default]
    No,
    Cmd(NotifyCmdArgs),
    DingTalk(DingTalkArgs),
}

pub fn load_config(p: impl AsRef<Path>) -> Result<Config> {
    let content = std::fs::read(p.as_ref()).with_context(|_| FileCtx {
        path: p.as_ref().to_path_buf(),
    })?;

    let cfg = toml::from_slice(&content)?;
    Ok(cfg)
}
