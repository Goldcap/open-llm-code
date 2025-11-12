mod config;
mod error;
mod types;

use clap::{Parser, Subcommand};
use error::Result;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ollm")]
#[command(about = "Open LLM Code - AI coding assistant with pluggable backends", long_about = None)]
struct Cli {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive REPL
    Repl,

    /// Generate example configuration file
    Init {
        /// Output path for config file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Show version information
    Version,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("open_llm_code={}", log_level))
        .init();

    match cli.command {
        Some(Commands::Init { output }) => {
            let config_example = config::Config::example();
            let output_path = output.unwrap_or_else(|| {
                let mut p = dirs::home_dir().expect("Cannot determine home directory");
                p.push(".config");
                p.push("open-llm-code");
                std::fs::create_dir_all(&p).ok();
                p.push("config.toml");
                p
            });

            std::fs::write(&output_path, config_example)
                .map_err(|e| error::OllmError::Config(format!("Failed to write config: {}", e)))?;

            println!("✅ Created example config at: {}", output_path.display());
            println!("Edit this file and add your API keys/credentials");
            Ok(())
        }

        Some(Commands::Version) => {
            println!("ollm v{}", env!("CARGO_PKG_VERSION"));
            println!("A Rust-based AI coding assistant with pluggable LLM backends");
            Ok(())
        }

        Some(Commands::Repl) | None => {
            println!("🚀 Open LLM Code v{}", env!("CARGO_PKG_VERSION"));
            println!();
            println!("Loading configuration...");

            // Load config
            let _config = config::Config::load(cli.config)?;

            println!("✅ Configuration loaded");
            println!();
            println!("🔧 REPL mode not yet implemented");
            println!("   Run `ollm init` to generate a config file");

            Ok(())
        }
    }
}
