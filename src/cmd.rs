use clap::{Parser, Subcommand};

mod simple;
mod watch;

pub use simple::SimpleArgs;
pub use watch::WatchArgs;

#[derive(Subcommand)]
pub enum Cmds {
    #[command(name = "simple")]
    Simple(SimpleArgs),

    #[command(name = "watch")]
    Watch(WatchArgs),
}

#[derive(Parser)]
pub struct Args {
    #[arg(long, default_value_t = false, global = true)]
    pub testnet: bool,

    #[arg(long, default_value_t = false, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub cmds: Cmds,
}
