#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! dioxus-agent-rs - Functional Rust `WebDriver` CLI for browser automation
//!
//! Architecture: Data → Calculations → Actions
//! - Data: Command types, error types, configuration
//! - Calculations: Pure functions for command validation, parsing, transformation
//! - Actions: Async `WebDriver` operations at the shell boundary

mod actions;
mod calculations;
mod data;

use crate::actions::execute_command;
use crate::calculations::validate_inputs;
use crate::data::Cli;
use clap::Parser;

/// Entry point - minimal async shell
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    // Validate CLI inputs - pure calculation
    let config = match validate_inputs(&cli) {
        Ok(c) => c,
        Err(e) => {
            if cli.json {
                let output = crate::data::CommandOutput {
                    success: false,
                    command: format!("{:?}", cli.command).split_whitespace().next().unwrap_or("unknown").to_string(),
                    target: None,
                    data: serde_json::Value::Null,
                    error: Some(format!("Invalid CLI arguments: {e}")),
                    logs: vec![],
                };
                println!("{}", serde_json::to_string(&output).unwrap_or_else(|_| r#"{"success":false,"command":"unknown","data":null,"error":"Failed to serialize output","logs":[]}"#.to_string()));
                std::process::exit(1);
            } else {
                return Err(anyhow::anyhow!("Invalid CLI arguments: {e}"));
            }
        }
    };

    // Execute the command - impure action at shell boundary
    if let Err(e) = execute_command(config).await {
        if cli.json {
            let output = crate::data::CommandOutput {
                success: false,
                command: format!("{:?}", cli.command).split_whitespace().next().unwrap_or("unknown").to_string(),
                target: None,
                data: serde_json::Value::Null,
                error: Some(format!("Command execution failed: {e}")),
                logs: vec![],
            };
            println!("{}", serde_json::to_string(&output).unwrap_or_else(|_| r#"{"success":false,"command":"unknown","data":null,"error":"Failed to serialize output","logs":[]}"#.to_string()));
            std::process::exit(1);
        } else {
            return Err(anyhow::anyhow!("Command execution failed: {e}"));
        }
    }
    
    Ok(())
}
