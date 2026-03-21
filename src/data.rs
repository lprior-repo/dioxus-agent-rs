#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Data layer - types, enums, configuration
//! Pure data structures with no logic

use clap::{Parser, Subcommand};
use std::time::Duration;
use url::Url;

/// CLI argument parser
#[derive(Parser, Debug)]
#[command(name = "dioxus-agent-rs")]
#[command(about = "Rust-based WebDriver Agent for browser automation", long_about = None)]
pub struct Cli {
    /// URL to navigate to (default: <http://localhost:8080>)
    #[arg(short, long, default_value = "http://localhost:8080")]
    pub url: String,

    /// Timeout in seconds (default: 10)
    #[arg(short, long, default_value = "10")]
    pub timeout: u64,

    /// Run the browser in headed mode (visible)
    #[arg(long)]
    pub no_headless: bool,

    /// Output all results as structured JSON
    #[arg(long)]
    pub json: bool,

    /// Automatically wait for Dioxus hydration before interacting
    #[arg(long)]
    pub auto_wait: bool,

    /// Enable AI execution tracing by providing a directory path to save the trace to
    #[arg(long)]
    pub trace: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

/// All available commands
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    // ============ Navigation Commands ============
    Dom,
    Title,
    Url,
    Refresh,
    Back,
    Forward,

    // ============ Element Interaction ============
    Click {
        selector: String,
    },
    DoubleClick {
        selector: String,
    },
    RightClick {
        selector: String,
    },
    Hover {
        selector: String,
    },
    Text {
        selector: String,
        value: String,
    },
    Clear {
        selector: String,
    },
    Submit {
        selector: String,
    },
    Select {
        selector: String,
        value: String,
    },
    Check {
        selector: String,
    },
    Uncheck {
        selector: String,
    },

    // ============ Element Queries ============
    GetText {
        selector: String,
    },
    Attr {
        selector: String,
        attribute: String,
    },
    Classes {
        selector: String,
    },
    TagName {
        selector: String,
    },
    Visible {
        selector: String,
    },
    Enabled {
        selector: String,
    },
    Selected {
        selector: String,
    },
    Count {
        selector: String,
    },
    FindAll {
        selector: String,
    },
    Exists {
        selector: String,
    },

    // ============ JavaScript & Execution ============
    Eval {
        js: String,
    },
    InjectCss {
        css: String,
    },

    // ============ Screenshot ============
    Screenshot {
        path: String,
    },
    ElementScreenshot {
        selector: String,
        path: String,
    },

    // ============ Viewport & Scrolling ============
    Viewport {
        width: u32,
        height: u32,
    },
    Scroll {
        selector: String,
    },
    ScrollBy {
        x: i32,
        y: i32,
    },

    // ============ Keyboard ============
    Key {
        key: String,
    },
    KeyCombo {
        combo: String,
    },

    // ============ Storage ============
    Cookies,
    SetCookie {
        name: String,
        value: String,
        domain: Option<String>,
        path: Option<String>,
    },
    DeleteCookie {
        name: String,
    },
    LocalGet {
        key: String,
    },
    LocalSet {
        key: String,
        value: String,
    },
    LocalRemove {
        key: String,
    },
    LocalClear,
    SessionGet {
        key: String,
    },
    SessionSet {
        key: String,
        value: String,
    },
    SessionClear,

    // ============ Console ============
    Console,
    ConsoleLog {
        #[arg(default_value = "log")]
        r#type: String,
    },

    // ============ Waiting ============
    Wait {
        selector: String,
    },
    WaitGone {
        selector: String,
    },
    WaitNav,
    WaitHydration,

    // ============ Dioxus-Specific ============
    DioxusState,
    DioxusClick {
        target: String,
    },
    SemanticTree,

    // ============ AI Agent Extended ============
    Upload {
        selector: String,
        path: String,
    },
    FillForm {
        json_data: String,
    },
    NetworkLogs,
    AssertText {
        selector: String,
        expected: String,
    },
    AssertVisible {
        selector: String,
    },
    AssertExists {
        selector: String,
    },

    // ============ AI Agent Advanced Capabilities ============
    FuzzyClick {
        text: String,
    },
    WaitNetworkIdle,
    ScrollToText {
        container: String,
        text: String,
    },
    ExtractTable {
        selector: String,
    },

    // ============ "God-Tier" Playwright Features ============
    MockRoute {
        url_pattern: String,
        response_json: String,
        #[arg(default_value = "200")]
        status: u16,
    },
    ShadowClick {
        selector: String,
    },
    DragAndDrop {
        source: String,
        target: String,
    },
    ExportState {
        path: String,
    },
    ImportState {
        path: String,
    },
    /// Wait for an element to become visible, enabled, and stop animating
    WaitStable {
        selector: String,
    },
    /// Compare a screenshot against a baseline image (Visual Regression)
    AssertScreenshot {
        /// The CSS selector (or leave empty for full page)
        #[arg(short, long)]
        selector: Option<String>,
        /// Path to the baseline image
        baseline: String,
        /// Path to save the current screenshot if it fails
        failure_path: String,
        /// Allowed percentage of pixel difference (0.0 to 100.0)
        #[arg(default_value = "1.0")]
        tolerance: f64,
    },

    // ============ Style & Interactive ============
    Style {
        selector: String,
        property: String,
    },
    Repl,
    ScreenshotAnnotated {
        path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserMode {
    Headless,
    Headed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Standard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitStrategy {
    Auto,
    Manual,
}

/// Runtime configuration after validation
#[derive(Debug, Clone)]
pub struct Config {
    pub url: Url,
    pub timeout: Duration,
    pub mode: BrowserMode,
    pub output: OutputFormat,
    pub wait: WaitStrategy,
    pub trace: Option<String>,
    pub command: Commands,
}

/// JSON Output format for AI agents
#[derive(serde::Serialize)]
pub struct CommandOutput {
    pub success: bool,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub logs: Vec<String>,
}
