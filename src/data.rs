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

    #[command(subcommand)]
    pub command: Commands,
}

/// All available commands - 50+ commands from SPEC.md
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    // ============ Navigation Commands ============
    /// Get the full HTML of the page
    Dom,
    /// Get page title
    Title,
    /// Get current URL
    Url,
    /// Refresh the page
    Refresh,
    /// Go back in history
    Back,
    /// Go forward in history
    Forward,

    // ============ Element Interaction ============
    /// Click an element by CSS selector
    Click {
        /// The CSS selector to click
        selector: String,
    },
    /// Double-click an element
    DoubleClick {
        /// The CSS selector
        selector: String,
    },
    /// Right-click (context menu)
    RightClick {
        /// The CSS selector
        selector: String,
    },
    /// Hover over element
    Hover {
        /// The CSS selector
        selector: String,
    },
    /// Set input value
    Text {
        /// The CSS selector
        selector: String,
        /// The value to set
        value: String,
    },
    /// Clear input field
    Clear {
        /// The CSS selector
        selector: String,
    },
    /// Submit form
    Submit {
        /// The CSS selector (form)
        selector: String,
    },
    /// Select dropdown option
    Select {
        /// The CSS selector (select element)
        selector: String,
        /// The option value to select
        value: String,
    },
    /// Check checkbox/radio
    Check {
        /// The CSS selector
        selector: String,
    },
    /// Uncheck checkbox
    Uncheck {
        /// The CSS selector
        selector: String,
    },

    // ============ Element Queries ============
    /// Get element text content
    GetText {
        /// The CSS selector
        selector: String,
    },
    /// Get attribute value
    Attr {
        /// The CSS selector
        selector: String,
        /// The attribute name
        attribute: String,
    },
    /// Get CSS classes
    Classes {
        /// The CSS selector
        selector: String,
    },
    /// Get element tag name
    TagName {
        /// The CSS selector
        selector: String,
    },
    /// Check if visible
    Visible {
        /// The CSS selector
        selector: String,
    },
    /// Check if enabled
    Enabled {
        /// The CSS selector
        selector: String,
    },
    /// Check if selected
    Selected {
        /// The CSS selector
        selector: String,
    },
    /// Count matching elements
    Count {
        /// The CSS selector
        selector: String,
    },
    /// Get all element HTML
    FindAll {
        /// The CSS selector
        selector: String,
    },
    /// Check if element exists
    Exists {
        /// The CSS selector
        selector: String,
    },

    // ============ JavaScript & Execution ============
    /// Execute JavaScript
    Eval {
        /// JavaScript expression to evaluate
        js: String,
    },
    /// Inject CSS into page
    InjectCss {
        /// CSS to inject
        css: String,
    },

    // ============ Screenshot ============
    /// Take full-page screenshot
    Screenshot {
        /// Path to save the screenshot
        path: String,
    },
    /// Take element screenshot
    ElementScreenshot {
        /// The CSS selector
        selector: String,
        /// Path to save the screenshot
        path: String,
    },

    // ============ Viewport & Scrolling ============
    /// Set viewport size
    Viewport {
        /// Width in pixels
        width: u32,
        /// Height in pixels
        height: u32,
    },
    /// Scroll element into view
    Scroll {
        /// The CSS selector
        selector: String,
    },
    /// Scroll by pixels
    ScrollBy {
        /// X offset
        x: i32,
        /// Y offset
        y: i32,
    },

    // ============ Keyboard ============
    /// Press keyboard key
    Key {
        /// Key to press (e.g., Enter, Escape, Tab)
        key: String,
    },
    /// Press key combination (e.g., Control+a)
    KeyCombo {
        /// Key combination (e.g., Control+Shift+Delete)
        combo: String,
    },

    // ============ Storage ============
    /// Get all cookies
    Cookies,
    /// Set cookie
    SetCookie {
        /// Cookie name
        name: String,
        /// Cookie value
        value: String,
        /// Domain (optional)
        domain: Option<String>,
        /// Path (optional)
        path: Option<String>,
    },
    /// Delete cookie
    DeleteCookie {
        /// Cookie name
        name: String,
    },
    /// Get localStorage item
    LocalGet {
        /// Key
        key: String,
    },
    /// Set localStorage item
    LocalSet {
        /// Key
        key: String,
        /// Value
        value: String,
    },
    /// Remove localStorage item
    LocalRemove {
        /// Key
        key: String,
    },
    /// Clear localStorage
    LocalClear,
    /// Get sessionStorage item
    SessionGet {
        /// Key
        key: String,
    },
    /// Set sessionStorage item
    SessionSet {
        /// Key
        key: String,
        /// Value
        value: String,
    },
    /// Clear sessionStorage
    SessionClear,

    // ============ Console ============
    /// Get all console messages
    Console,
    /// Get console messages by type (log, warn, error, info, debug)
    ConsoleLog {
        /// Console type
        #[arg(default_value = "log")]
        r#type: String,
    },

    // ============ Waiting ============
    /// Wait for element to appear
    Wait {
        /// The CSS selector
        selector: String,
    },
    /// Wait for element to disappear
    WaitGone {
        /// The CSS selector
        selector: String,
    },
    /// Wait for navigation
    WaitNav,
    /// Wait for Dioxus hydration
    WaitHydration,

    // ============ Dioxus-Specific ============
    /// Get Dioxus internal state
    DioxusState,
    /// Click Dioxus element by data-target attribute
    DioxusClick {
        /// The target ID
        target: String,
    },

    // ============ Style ============
    /// Get computed style property
    Style {
        /// The CSS selector
        selector: String,
        /// CSS property name
        property: String,
    },
}

/// Runtime configuration after validation
#[derive(Debug, Clone)]
pub struct Config {
    pub url: String,
    pub timeout: Duration,
    pub command: Commands,
}

impl From<Cli> for Config {
    fn from(cli: Cli) -> Self {
        Self {
            url: cli.url,
            timeout: Duration::from_secs(cli.timeout),
            command: cli.command,
        }
    }
}
