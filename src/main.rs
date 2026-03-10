use anyhow::Result;
use clap::{Parser, Subcommand};
use fantoccini::ClientBuilder;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "dioxus-agent-rs")]
#[command(about = "Rust-based WebDriver Agent for browser automation", long_about = None)]
struct Cli {
    /// URL to navigate to (default: http://localhost:8080)
    #[arg(short, long, default_value = "http://localhost:8080")]
    url: String,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get the full HTML of the page
    Dom,
    /// Click an element matching the CSS selector
    Click {
        /// The CSS selector to click
        selector: String,
    },
    /// Set the value of an input element
    Text {
        /// The CSS selector to target
        selector: String,
        /// The value to set
        value: String,
    },
    /// Evaluate arbitrary JavaScript
    Eval {
        /// JavaScript expression to evaluate
        js: String,
    },
    /// Take a full-page screenshot
    Screenshot {
        /// Path to save the screenshot
        path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Connect to a local Chrome instance via WebDriver
    // First, ensure chromedriver is running on localhost:4444
    let mut caps = serde_json::Map::new();
    let chrome_opts = serde_json::json!({
        "args": ["headless", "no-sandbox", "disable-dev-shm-usage", "disable-gpu"]
    });
    caps.insert("goog:chromeOptions".to_string(), chrome_opts);

    let mut client = ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:4444")
        .await?;

    // Navigate to the URL
    client.goto(&cli.url).await?;
    
    // Give Dioxus time to hydrate
    tokio::time::sleep(Duration::from_millis(500)).await;

    match &cli.command {
        Commands::Dom => {
            let source = client.source().await?;
            println!("{}", source);
        }
        Commands::Click { selector } => {
            let el = client.find(fantoccini::Locator::Css(selector)).await?;
            el.click().await?;
            println!("Clicked {}", selector);
        }
        Commands::Text { selector, value } => {
            let el = client.find(fantoccini::Locator::Css(selector)).await?;
            el.send_keys(value).await?;
            println!("Set {} to {}", selector, value);
        }
        Commands::Eval { js } => {
            let result = client.execute(&js, vec![]).await?;
            println!("{}", result);
        }
        Commands::Screenshot { path } => {
            // Take screenshot using Chrome's native screenshot capability
            let png_data = client.screenshot().await?;
            std::fs::write(path, png_data)?;
            println!("Saved screenshot to {}", path);
        }
    }

    client.close().await?;
    Ok(())
}
