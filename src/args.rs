use clap::Parser;

#[derive(Parser)]
#[command(name = "tinybot")]
pub struct Args {
    #[arg(short, long, default_value = "tinybot.toml")]
    pub config: String,
}
