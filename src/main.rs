#[cfg(feature = "server")]
use {
    clap::Parser,
    tbay::{
        cmd::{Args, Cmds},
        logger::logger_init,
        prelude::Result,
    },
};

#[cfg(feature = "server")]
#[tokio::main]
pub async fn main() -> Result<()> {
    logger_init();

    let args = Args::parse();
    match args.cmds {
        Cmds::Simple(simple) => simple.run().await,
        Cmds::Watch(watch) => watch.run().await,
    }
}

#[cfg(not(feature = "server"))]
pub fn main() {
    dioxus::launch(tbay::web::App);
}
