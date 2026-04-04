use clap::Parser;
use humantime::Duration;
use notify_rust::{Notification, error::Result};

#[derive(Parser)]
pub struct Args {
    #[arg(long, short)]
    pub summary: String,

    #[arg(long, short)]
    pub body: String,

    #[arg(long, short, default_value_t = Duration::from(std::time::Duration::from_secs(10)))]
    pub timeout: Duration,
}

fn main() -> Result<()> {
    let args = Args::parse();

    Notification::new()
        .summary(&args.summary)
        .body(&args.body)
        .timeout(std::time::Duration::from(args.timeout))
        .show()?;

    Ok(())
}
