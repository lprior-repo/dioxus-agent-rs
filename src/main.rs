#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! dioxus-agent-rs - Functional Rust WebDriver CLI for browser automation
//! 
//! Architecture: Data → Calculations → Actions
//! - Data: Command types, error types, configuration
//! - Calculations: Pure functions for command validation, parsing, transformation
//! - Actions: Async `WebDriver` operations at the shell boundary

mod calculations;
mod actions;
mod data;

use clap::Parser;
use crate::data::Cli;
use crate::calculations::validate_inputs;
use crate::actions::execute_command;

/// Entry point - minimal async shell
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();
    
    // Validate CLI inputs - pure calculation
    let config = validate_inputs(&cli)
        .map_err(|e| anyhow::anyhow!("Invalid CLI arguments: {e}"))?;
    
    // Execute the command - impure action at shell boundary
    execute_command(config).await
        .map_err(|e| anyhow::anyhow!("Command execution failed: {e}"))
}
