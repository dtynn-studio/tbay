#![feature(duration_constructors)]

#[cfg(feature = "server")]
pub mod cmd;
#[cfg(feature = "server")]
pub mod common;
#[cfg(feature = "server")]
pub mod config;
#[cfg(feature = "server")]
pub mod event;
#[cfg(feature = "server")]
pub mod hub;
#[cfg(feature = "server")]
pub mod indicator;
#[cfg(feature = "server")]
pub mod k;
#[cfg(feature = "server")]
pub mod logger;
#[cfg(feature = "server")]
pub mod monitor;
#[cfg(feature = "server")]
pub mod notifier;
#[cfg(feature = "server")]
pub mod prelude;
#[cfg(feature = "server")]
pub mod res;
#[cfg(feature = "server")]
pub mod util;
pub mod web;

#[cfg(feature = "server")]
use common::impl_builder;
