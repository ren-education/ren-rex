mod cli;
mod commands;
mod output;
mod wire;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let cli = cli::Cli::parse();
    if cli.remote.is_some() {
        eprintln!("--remote is not implemented in v1; the CLI runs in-process");
        std::process::exit(64);
    }
    let code = commands::dispatch(cli).await;
    std::process::exit(code);
}

fn init_tracing() {
    let format = std::env::var("REX_LOG_FORMAT").unwrap_or_else(|_| "pretty".into());
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new("rex=info,warn"))
        .unwrap();
    match format.as_str() {
        "json" => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .init();
        }
        _ => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_target(false)
                .init();
        }
    }
}
