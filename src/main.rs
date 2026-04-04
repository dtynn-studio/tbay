use clap::Parser;
use tbay::{
    cmd::{Args, Cmds},
    logger::logger_init,
    prelude::Result,
};

#[tokio::main]
pub async fn main() -> Result<()> {
    logger_init();

    let args = Args::parse();
    match args.cmds {
        Cmds::Simple(simple) => simple.run().await,
        Cmds::Watch(watch) => watch.run().await,
    }
}
