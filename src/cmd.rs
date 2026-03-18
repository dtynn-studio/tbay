use clap::{Parser, Subcommand};

mod simple;

pub use simple::SimpleArgs;

#[derive(Subcommand)]
pub enum Cmds {
    #[command(name = "simple")]
    Simple(SimpleArgs),
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
