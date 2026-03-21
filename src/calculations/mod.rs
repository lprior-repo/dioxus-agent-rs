pub mod image_diff;
pub mod js;
pub mod trace;
pub use image_diff::*;
pub use js::*;
pub use trace::*;

/// Calculations layer - pure functions for validation and transformation
use crate::data::{BrowserMode, Cli, Commands, Config, OutputFormat, WaitStrategy};
use std::time::Duration;
use url::Url;

#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("Invalid target URL: {0}")]
    InvalidUrl(url::ParseError),
    #[error("Timeout must be greater than 0")]
    ZeroTimeout,
    #[error("{0} cannot be empty or whitespace")]
    EmptyField(&'static str),
    #[error("Key contains invalid characters, must be alphanumeric/underscore")]
    InvalidStorageKey,
    #[error("Invalid console type, must be log, warn, error, info, or debug")]
    InvalidConsoleType,
    #[error("Javascript contains potentially dangerous patterns")]
    DangerousJavascript,
    #[error("Invalid JSON object: {0}")]
    InvalidJson(serde_json::Error),
    #[error("Viewport width and height must be > 0")]
    ZeroViewport,
    #[error("Cookie name cannot contain null bytes")]
    InvalidCookieName,
}

/// Validates CLI inputs and produces a `Config` object.
///
/// # Errors
///
/// Returns `ValidationError` if the URL is invalid, timeout is zero, or if the command validation fails.
pub fn validate_inputs(cli: &Cli) -> Result<Config, ValidationError> {
    let url = Url::parse(&cli.url).map_err(ValidationError::InvalidUrl)?;
    if cli.timeout == 0 {
        return Err(ValidationError::ZeroTimeout);
    }

    validate_command(&cli.command)?;

    let mode = if cli.no_headless {
        BrowserMode::Headed
    } else {
        BrowserMode::Headless
    };

    let output = if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::Standard
    };

    let wait = if cli.auto_wait {
        WaitStrategy::Auto
    } else {
        WaitStrategy::Manual
    };

    Ok(Config {
        url,
        timeout: Duration::from_secs(cli.timeout),
        mode,
        output,
        wait,
        trace: cli.trace.clone(),
        engine: cli.engine.clone(),
        command: cli.command.clone(),
    })
}

fn validate_non_empty(s: &str, field: &'static str) -> Result<(), ValidationError> {
    if s.trim().is_empty() {
        return Err(ValidationError::EmptyField(field));
    }
    Ok(())
}

fn validate_selector(s: &str) -> Result<(), ValidationError> {
    validate_non_empty(s, "selector")
}

fn validate_key(k: &str) -> Result<(), ValidationError> {
    validate_non_empty(k, "key")
}

fn validate_path(p: &str) -> Result<(), ValidationError> {
    validate_non_empty(p, "path")
}

fn validate_value(v: &str) -> Result<(), ValidationError> {
    validate_non_empty(v, "value")
}

fn validate_text(t: &str) -> Result<(), ValidationError> {
    validate_non_empty(t, "text")
}

fn validate_storage_key(key: &str) -> Result<(), ValidationError> {
    validate_non_empty(key, "key")?;
    let first_char = key
        .chars()
        .next()
        .ok_or(ValidationError::EmptyField("key"))?;
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err(ValidationError::InvalidStorageKey);
    }
    if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ValidationError::InvalidStorageKey);
    }
    Ok(())
}

fn validate_command(cmd: &Commands) -> Result<(), ValidationError> {
    match cmd {
        // Simple selectors
        Commands::Click { selector }
        | Commands::DoubleClick { selector }
        | Commands::RightClick { selector }
        | Commands::Hover { selector }
        | Commands::Clear { selector }
        | Commands::Submit { selector }
        | Commands::GetText { selector }
        | Commands::TagName { selector }
        | Commands::Visible { selector }
        | Commands::Enabled { selector }
        | Commands::Selected { selector }
        | Commands::Count { selector }
        | Commands::FindAll { selector }
        | Commands::Exists { selector }
        | Commands::Scroll { selector }
        | Commands::Classes { selector }
        | Commands::AssertVisible { selector }
        | Commands::AssertExists { selector }
        | Commands::ExtractTable { selector }
        | Commands::ShadowClick { selector }
        | Commands::Wait { selector }
        | Commands::WaitGone { selector } => validate_selector(selector),

        // Selector + Path
        Commands::ElementScreenshot { selector, path } | Commands::Upload { selector, path } => {
            validate_selector(selector)?;
            validate_path(path)
        }

        // Selector + Value
        Commands::Text { selector, value } | Commands::Select { selector, value } => {
            validate_selector(selector)?;
            validate_value(value)
        }

        // Other Selector pairs
        Commands::AssertText { selector, expected } => {
            validate_selector(selector)?;
            validate_non_empty(expected, "expected")
        }
        Commands::Attr {
            selector,
            attribute,
        } => {
            validate_selector(selector)?;
            validate_non_empty(attribute, "attribute")
        }
        Commands::Style { selector, property } => {
            validate_selector(selector)?;
            validate_non_empty(property, "property")
        }

        // Path only
        Commands::Screenshot { path }
        | Commands::ScreenshotAnnotated { path }
        | Commands::ExportState { path }
        | Commands::ImportState { path } => validate_path(path),

        Commands::AssertScreenshot {
            selector,
            baseline,
            failure_path,
            tolerance: _,
        } => {
            if let Some(s) = selector {
                validate_selector(s)?;
            }
            validate_path(baseline)?;
            validate_path(failure_path)
        }

        // Other simple validations
        Commands::Viewport { width, height } => {
            if *width == 0 || *height == 0 {
                Err(ValidationError::ZeroViewport)
            } else {
                Ok(())
            }
        }
        Commands::Key { key } | Commands::KeyCombo { combo: key } => validate_key(key),
        Commands::SetCookie { name, value, .. } => {
            validate_non_empty(name, "name")?;
            validate_value(value)?;
            if name.contains('\0') {
                Err(ValidationError::InvalidCookieName)
            } else {
                Ok(())
            }
        }
        Commands::DeleteCookie { name } => validate_non_empty(name, "name"),

        // Storage and rest
        _ => validate_command_rest(cmd),
    }
}

fn validate_command_rest(cmd: &Commands) -> Result<(), ValidationError> {
    match cmd {
        Commands::LocalGet { key }
        | Commands::LocalRemove { key }
        | Commands::SessionGet { key } => validate_storage_key(key),
        Commands::LocalSet { key, value } | Commands::SessionSet { key, value } => {
            validate_storage_key(key)?;
            validate_value(value)
        }
        Commands::ConsoleLog { r#type } => {
            if matches!(r#type.as_str(), "log" | "warn" | "error" | "info" | "debug") {
                Ok(())
            } else {
                Err(ValidationError::InvalidConsoleType)
            }
        }
        Commands::DioxusClick { target } => validate_non_empty(target, "target"),
        Commands::Eval { js } => {
            validate_non_empty(js, "js")?;
            let dangerous = ["eval(", "Function(", "setTimeout", "setInterval"];
            if dangerous.iter().any(|p| js.contains(p)) {
                Err(ValidationError::DangerousJavascript)
            } else {
                Ok(())
            }
        }
        Commands::InjectCss { css } => validate_non_empty(css, "css"),
        Commands::FillForm { json_data } => {
            validate_non_empty(json_data, "json_data")?;
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(json_data)
                .map_err(ValidationError::InvalidJson)?;
            Ok(())
        }
        Commands::FuzzyClick { text } => validate_text(text),
        Commands::ScrollToText { container, text } => {
            validate_selector(container)?;
            validate_text(text)
        }
        Commands::MockRoute {
            url_pattern,
            response_json,
            ..
        } => {
            validate_non_empty(url_pattern, "url_pattern")?;
            validate_non_empty(response_json, "response_json")
        }
        Commands::DragAndDrop { source, target } => {
            validate_selector(source)?;
            validate_selector(target)
        }
        _ => Ok(()), // Handled by first function or no validation needed
    }
}

#[must_use]
pub fn escape_js_string(s: &str) -> String {
    s.replace('\\', r"\\")
        .replace('\'', r"\'")
        .replace('"', r#"\""#)
        .replace('\n', r"\n")
        .replace('\r', r"\r")
        .replace('\t', r"\t")
}
