use clap::Parser;
use tbay::{
    cmd::{Args, Cmds},
    logger::logger_init,
    prelude::Result,
};

pub fn main() -> Result<()> {
    logger_init();

    let args = Args::parse();
    match args.cmds {
        Cmds::Simple(simple) => simple.run(),
    }
}
