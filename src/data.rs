#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Data layer - types, enums, configuration
//! Pure data structures with no logic

pub mod types;

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

    /// The browser automation engine to use
    #[arg(long, value_enum, default_value_t = Engine::Cdp)]
    pub engine: Engine,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::ValueEnum, Debug, Clone, PartialEq, Eq)]
pub enum Engine {
    /// Pure CDP via chromiumoxide (Zero-setup, Chrome only)
    Cdp,
    /// Dual-Driver: W3C `WebDriver` (fantoccini) + CDP (Requires chromedriver on port 4444)
    Dual,
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
        selector: crate::data::types::Selector,
    },
    DoubleClick {
        selector: crate::data::types::Selector,
    },
    RightClick {
        selector: crate::data::types::Selector,
    },
    Hover {
        selector: crate::data::types::Selector,
    },
    Text {
        selector: crate::data::types::Selector,
        value: crate::data::types::InputValue,
    },
    Clear {
        selector: crate::data::types::Selector,
    },
    Submit {
        selector: crate::data::types::Selector,
    },
    Select {
        selector: crate::data::types::Selector,
        value: crate::data::types::InputValue,
    },
    Check {
        selector: crate::data::types::Selector,
    },
    Uncheck {
        selector: crate::data::types::Selector,
    },

    // ============ Element Queries ============
    GetText {
        selector: crate::data::types::Selector,
    },
    Attr {
        selector: crate::data::types::Selector,
        attribute: String,
    },
    Classes {
        selector: crate::data::types::Selector,
    },
    TagName {
        selector: crate::data::types::Selector,
    },
    Visible {
        selector: crate::data::types::Selector,
    },
    Enabled {
        selector: crate::data::types::Selector,
    },
    Selected {
        selector: crate::data::types::Selector,
    },
    Count {
        selector: crate::data::types::Selector,
    },
    FindAll {
        selector: crate::data::types::Selector,
    },
    Exists {
        selector: crate::data::types::Selector,
    },

    // ============ JavaScript & Execution ============
    Eval {
        js: crate::data::types::JsPayload,
    },
    InjectCss {
        css: crate::data::types::CssPayload,
    },

    // ============ Screenshot ============
    Screenshot {
        path: crate::data::types::FilePath,
    },
    ElementScreenshot {
        selector: crate::data::types::Selector,
        path: crate::data::types::FilePath,
    },

    // ============ Viewport & Scrolling ============
    Viewport {
        width: u32,
        height: u32,
    },
    Scroll {
        selector: crate::data::types::Selector,
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
        value: crate::data::types::InputValue,
        domain: Option<String>,
        path: Option<String>,
    },
    DeleteCookie {
        name: String,
    },
    LocalGet {
        key: crate::data::types::StorageKey,
    },
    LocalSet {
        key: crate::data::types::StorageKey,
        value: crate::data::types::InputValue,
    },
    LocalRemove {
        key: crate::data::types::StorageKey,
    },
    LocalClear,
    SessionGet {
        key: crate::data::types::StorageKey,
    },
    SessionSet {
        key: crate::data::types::StorageKey,
        value: crate::data::types::InputValue,
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
        selector: crate::data::types::Selector,
    },
    WaitGone {
        selector: crate::data::types::Selector,
    },
    WaitNav,
    WaitHydration,

    // ============ Dioxus-Specific ============
    DioxusState,
    DioxusClick {
        target: crate::data::types::Selector,
    },
    SemanticTree,

    // ============ AI Agent Extended ============
    Upload {
        selector: crate::data::types::Selector,
        path: crate::data::types::FilePath,
    },
    FillForm {
        json_data: String,
    },
    NetworkLogs,
    AssertText {
        selector: crate::data::types::Selector,
        expected: crate::data::types::ExpectedText,
    },
    AssertVisible {
        selector: crate::data::types::Selector,
    },
    AssertExists {
        selector: crate::data::types::Selector,
    },

    // ============ AI Agent Advanced Capabilities ============
    FuzzyClick {
        text: crate::data::types::ExpectedText,
    },
    WaitNetworkIdle,
    ScrollToText {
        container: crate::data::types::Selector,
        text: crate::data::types::ExpectedText,
    },
    ExtractTable {
        selector: crate::data::types::Selector,
    },

    // ============ "God-Tier" Playwright Features ============
    MockRoute {
        url_pattern: String,
        response_json: String,
        #[arg(default_value = "200")]
        status: u16,
    },
    ShadowClick {
        selector: crate::data::types::Selector,
    },
    DragAndDrop {
        source: crate::data::types::Selector,
        target: crate::data::types::Selector,
    },
    ExportState {
        path: crate::data::types::FilePath,
    },
    ImportState {
        path: crate::data::types::FilePath,
    },
    /// Wait for an element to become visible, enabled, and stop animating
    WaitStable {
        selector: crate::data::types::Selector,
    },
    /// Compare a screenshot against a baseline image (Visual Regression)
    AssertScreenshot {
        /// The CSS selector (or leave empty for full page)
        #[arg(short, long)]
        selector: Option<crate::data::types::Selector>,
        /// Path to the baseline image
        baseline: crate::data::types::FilePath,
        /// Path to save the current screenshot if it fails
        failure_path: crate::data::types::FilePath,
        /// Allowed percentage of pixel difference (0.0 to 100.0)
        #[arg(default_value = "1.0")]
        tolerance: f64,
    },

    // ============ Style & Interactive ============
    Style {
        selector: crate::data::types::Selector,
        property: String,
    },
    Repl,
    ScreenshotAnnotated {
        path: crate::data::types::FilePath,
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
    pub engine: Engine,
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
