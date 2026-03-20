#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Calculations layer - pure functions for validation and transformation
//! No side effects, no I/O, no mutable state

use crate::data::{Cli, Commands, Config};
use std::time::Duration;
use url::Url;

/// Validate CLI inputs and produce Config
/// Pure function - no side effects
pub fn validate_inputs(cli: &Cli) -> Result<Config, String> {
    // Validate URL format using url::Url::parse()
    match Url::parse(&cli.url) {
        Ok(_) => {}
        Err(e) => return Err(format!("Invalid URL: {e}")),
    }

    // Validate timeout
    if cli.timeout == 0 {
        return Err("timeout must be > 0".into());
    }

    // Validate command-specific inputs
    validate_command(&cli.command)?;

    Ok(Config {
        url: cli.url.clone(),
        timeout: Duration::from_secs(cli.timeout),
        webdriver_url: cli.webdriver_url.clone(),
        no_headless: cli.no_headless,
        json: cli.json,
        auto_wait: cli.auto_wait,
        command: cli.command.clone(),
    })
}

/// Validate command-specific arguments
/// Returns Ok(()) if valid, Err(message) if invalid
fn validate_command(cmd: &Commands) -> Result<(), String> {
    match cmd {
        // Commands with selector validation (single field)
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
        | Commands::ElementScreenshot { selector, .. } => validate_non_empty_selector(selector),

        // Commands with selector + value
        Commands::Text { selector, value } | Commands::Select { selector, value } => {
            validate_non_empty_selector(selector)?;
            validate_non_empty(value, "value")
        }

        // Commands with selector + path
        Commands::Upload { selector, path } => {
            validate_non_empty_selector(selector)?;
            validate_non_empty(path, "path")
        }

        // Commands with selector + expected
        Commands::AssertText { selector, expected } => {
            validate_non_empty_selector(selector)?;
            validate_non_empty(expected, "expected")
        }

        // Commands with selector + attribute
        Commands::Attr {
            selector,
            attribute,
        } => {
            validate_non_empty_selector(selector)?;
            validate_non_empty(attribute, "attribute")
        }

        // Commands with selector + property
        Commands::Style { selector, property } => {
            validate_non_empty_selector(selector)?;
            validate_non_empty(property, "property")
        }

        // Commands with selector + path
        Commands::Screenshot { path } => validate_non_empty(path, "path"),

        // Commands with viewport
        Commands::Viewport { width, height } => {
            if *width == 0 {
                return Err("width must be > 0".into());
            }
            if *height == 0 {
                return Err("height must be > 0".into());
            }
            Ok(())
        }

        // Commands with key validation
        Commands::Key { key } => validate_non_empty(key, "key"),
        Commands::KeyCombo { combo } => validate_non_empty(combo, "key"),

        // Commands with cookie validation
        Commands::SetCookie { name, value, .. } => {
            validate_non_empty(name, "name")?;
            validate_non_empty(value, "value")?;
            if name.contains('\0') {
                return Err("cookie name cannot contain null bytes".into());
            }
            Ok(())
        }

        Commands::DeleteCookie { name } => validate_non_empty(name, "name"),

        // Storage commands
        Commands::LocalGet { key }
        | Commands::LocalRemove { key }
        | Commands::SessionGet { key } => validate_storage_key(key),

        Commands::LocalSet { key, value } | Commands::SessionSet { key, value } => {
            validate_storage_key(key)?;
            validate_non_empty(value, "value")
        }

        // Console type validation
        Commands::ConsoleLog { r#type } => {
            let valid = matches!(r#type.as_str(), "log" | "warn" | "error" | "info" | "debug");
            if valid {
                Ok(())
            } else {
                Err(format!("invalid console type: {type}"))
            }
        }

        // Dioxus commands
        Commands::DioxusClick { target } => validate_non_empty(target, "target"),

        // Commands with JS/CSS validation
        Commands::Eval { js } => {
            if js.is_empty() {
                return Err("JavaScript cannot be empty".into());
            }
            // Check for dangerous patterns
            let dangerous = ["eval(", "Function(", "setTimeout", "setInterval"];
            let has_dangerous = dangerous.iter().any(|p| js.contains(p));
            if has_dangerous {
                return Err("JavaScript contains potentially dangerous patterns".into());
            }
            Ok(())
        }

        Commands::InjectCss { css } => {
            if css.is_empty() {
                return Err("CSS cannot be empty".into());
            }
            Ok(())
        }

        // All other commands are valid (unit variants)
        Commands::Dom
        | Commands::Title
        | Commands::Url
        | Commands::Refresh
        | Commands::Back
        | Commands::Forward
        | Commands::Check { .. }
        | Commands::Uncheck { .. }
        | Commands::Cookies
        | Commands::LocalClear
        | Commands::SessionClear
        | Commands::Console
        | Commands::NetworkLogs
        | Commands::Wait { .. }
        | Commands::WaitGone { .. }
        | Commands::WaitNav
        | Commands::WaitHydration
        | Commands::DioxusState
        | Commands::SemanticTree
        | Commands::Repl
        | Commands::ScrollBy { .. } => Ok(()),

        Commands::ScreenshotAnnotated { path } => validate_non_empty(path, "path"),
    }
}

/// Validate that selector is not empty or whitespace-only
fn validate_non_empty_selector(selector: &str) -> Result<(), String> {
    validate_non_empty(selector, "selector")
}

/// Validate that a string is not empty or whitespace-only
fn validate_non_empty(s: &str, field: &str) -> Result<(), String> {
    if s.trim().is_empty() {
        return Err(format!("{field} cannot be empty or whitespace"));
    }
    Ok(())
}

/// Validate storage key (alphanumeric + underscore)
fn validate_storage_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("key cannot be empty".into());
    }
    let first_char = key.chars().next().unwrap_or(' ');
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err("key must start with letter or underscore".into());
    }
    let valid = key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if valid {
        Ok(())
    } else {
        Err("key contains invalid characters".into())
    }
}

/// Generate JavaScript for keyboard actions
/// Pure function - no side effects
#[must_use]
pub fn generate_keypress_js(key: &str) -> String {
    let key_lower = key.to_lowercase();
    match key_lower.as_str() {
        "enter" => "return { key: 'Enter' }".into(),
        "escape" | "esc" => "return { key: 'Escape' }".into(),
        "tab" => "return { key: 'Tab' }".into(),
        "backspace" => "return { key: 'Backspace' }".into(),
        "delete" | "del" => "return { key: 'Delete' }".into(),
        "arrowup" | "up" => "return { key: 'ArrowUp' }".into(),
        "arrowdown" | "down" => "return { key: 'ArrowDown' }".into(),
        "arrowleft" | "left" => "return { key: 'ArrowLeft' }".into(),
        "arrowright" | "right" => "return { key: 'ArrowRight' }".into(),
        "home" => "return { key: 'Home' }".into(),
        "end" => "return { key: 'End' }".into(),
        "pageup" => "return { key: 'PageUp' }".into(),
        "pagedown" => "return { key: 'PageDown' }".into(),
        _ => format!("return {{ key: '{}' }}", key.replace('\'', "\\'")),
    }
}

/// Generate JavaScript for key combination
pub fn generate_keycombo_js(combo: &str) -> String {
    let parts: Vec<String> = combo
        .split('+')
        .map(str::trim)
        .map(std::string::ToString::to_string)
        .collect();

    let mut modifiers = Vec::new();
    let mut key: Option<String> = None;

    for part in parts {
        let lower = part.to_lowercase();
        match lower.as_str() {
            "control" | "ctrl" => modifiers.push("ctrlKey"),
            "shift" => modifiers.push("shiftKey"),
            "alt" => modifiers.push("altKey"),
            "meta" | "cmd" | "command" => modifiers.push("metaKey"),
            _ => key = Some(part.clone()), // Keep original case for special keys
        }
    }

    let key = key.unwrap_or_default();

    if modifiers.is_empty() {
        format!(
            "return {{ key: '{}', ctrlKey: false, shiftKey: false, altKey: false, metaKey: false }}",
            key.replace('\'', "\\'")
        )
    } else {
        let modifier_expr = modifiers.join(", ");
        format!(
            "return {{ key: '{}', {} }}",
            key.replace('\'', "\\'"),
            modifier_expr
        )
    }
}

/// Generate storage JS
#[must_use]
pub fn generate_storage_js(
    storage: &str,
    op: &str,
    key: Option<&str>,
    value: Option<&str>,
) -> String {
    match (storage, op, key, value) {
        ("local", "get", Some(k), _) => {
            format!("return localStorage.getItem('{}');", escape_js_string(k))
        }
        ("local", "set", Some(k), Some(v)) => {
            format!(
                "localStorage.setItem('{}', '{}'); return true;",
                escape_js_string(k),
                escape_js_string(v)
            )
        }
        ("local", "remove", Some(k), _) => {
            format!(
                "localStorage.removeItem('{}'); return true;",
                escape_js_string(k)
            )
        }
        ("local", "clear", None, _) => "localStorage.clear(); return true;".into(),
        ("session", "get", Some(k), _) => {
            format!("return sessionStorage.getItem('{}');", escape_js_string(k))
        }
        ("session", "set", Some(k), Some(v)) => {
            format!(
                "sessionStorage.setItem('{}', '{}'); return true;",
                escape_js_string(k),
                escape_js_string(v)
            )
        }
        ("session", "clear", None, _) => "sessionStorage.clear(); return true;".into(),
        _ => "return null;".into(),
    }
}

/// Escape string for JavaScript
#[must_use]
pub fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Generate CSS injection JS
#[must_use]
pub fn generate_css_injection_js(css: &str) -> String {
    let escaped = css.replace('\'', "\\'").replace('\n', " ");
    format!(
        "const style = document.createElement('style'); \
        style.textContent = '{escaped}'; \
        document.head.appendChild(style); \
        return true;"
    )
}

/// Generate element screenshot JS
#[must_use]
pub fn generate_element_screenshot_js(selector: &str) -> String {
    format!(
        "const el = document.querySelector('{}'); \
        if (!el) return null; \
        const rect = el.getBoundingClientRect(); \
        return {{ x: rect.x, y: rect.y, width: rect.width, height: rect.height }};",
        selector.replace('\'', "\\'")
    )
}

/// Generate Dioxus click JS
#[must_use]
pub fn generate_dioxus_click_js(target: &str) -> String {
    format!(
        "const el = document.querySelector('[data-target=\"{}\"]'); \
        if (el) {{ el.click(); return true; }} \
        return false;",
        target.replace('\'', "\\'")
    )
}

/// Generate Dioxus state JS
#[must_use]
pub fn generate_dioxus_state_js() -> String {
    "if (typeof window.getDioxusState === 'function') { return window.getDioxusState(); } \
     if (typeof window.__DX_STATE__ !== 'undefined') { return window.__DX_STATE__; } \
     const states = []; \
     for (let key in window) { \
         if (key.startsWith('__dx') || key.startsWith('__dioxus')) { \
             try { states.push({ [key]: window[key] }); } catch(e) {} \
         } \
     } \
     return states.length > 0 ? states : null;"
        .into()
}

/// Generate hydration wait JS
#[must_use]
pub fn generate_hydration_wait_js() -> String {
    "return new Promise((resolve) => { \
        const checkReady = () => { \
            const hasSuspense = document.querySelector('[aria-busy=\"true\"]'); \
            const hasHydrated = document.body.hasAttribute('data-hydrated') || document.querySelector('#main') || document.body.innerHTML.length > 50; \
            if (hasHydrated && !hasSuspense) { resolve(true); return true; } \
            return false; \
        }; \
        if (checkReady()) return; \
        const observer = new MutationObserver(() => { \
            if (checkReady()) { observer.disconnect(); resolve(true); } \
        }); \
        observer.observe(document.body, { childList: true, subtree: true, attributes: true }); \
        setTimeout(() => { observer.disconnect(); resolve(true); }, 10000); \
    });"
    .into()
}

/// Generate Semantic Tree JS
#[must_use]
pub fn generate_semantic_tree_js() -> String {
    "function getSemanticTree(root) { \
        const tree = []; \
        const iter = document.createNodeIterator(root, NodeFilter.SHOW_ELEMENT, { \
            acceptNode: (node) => { \
                const tag = node.tagName.toLowerCase(); \
                const interactable = ['a', 'button', 'input', 'select', 'textarea'].includes(tag) || node.hasAttribute('role') || node.hasAttribute('tabindex'); \
                if (!interactable) return NodeFilter.FILTER_SKIP; \
                const style = window.getComputedStyle(node); \
                if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return NodeFilter.FILTER_REJECT; \
                return NodeFilter.FILTER_ACCEPT; \
            } \
        }); \
        let node; \
        while (node = iter.nextNode()) { \
            const tag = node.tagName.toLowerCase(); \
            let label = node.innerText || node.value || node.getAttribute('aria-label') || node.getAttribute('title') || ''; \
            label = label.substring(0, 50).replace(/\\n/g, ' ').trim(); \
            let id = node.id ? '#' + node.id : ''; \
            let cls = node.className ? '.' + node.className.split(' ').join('.') : ''; \
            let sel = id ? id : (cls ? tag + cls : tag); \
            tree.push(`[${tag.toUpperCase()}] ${sel} \"${label}\"`); \
        } \
        return tree.join('\\n'); \
    } \
    return getSemanticTree(document.body);"
        .into()
}

/// Generate Screenshot Annotated JS
#[must_use]
pub fn generate_screenshot_annotated_js() -> String {
    "const iter = document.createNodeIterator(document.body, NodeFilter.SHOW_ELEMENT, { \
        acceptNode: (node) => { \
            const tag = node.tagName.toLowerCase(); \
            const interactable = ['a', 'button', 'input', 'select', 'textarea'].includes(tag) || node.hasAttribute('role') || node.hasAttribute('tabindex'); \
            if (!interactable) return NodeFilter.FILTER_SKIP; \
            const style = window.getComputedStyle(node); \
            if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return NodeFilter.FILTER_REJECT; \
            return NodeFilter.FILTER_ACCEPT; \
        } \
    }); \
    let node; let counter = 1; \
    while (node = iter.nextNode()) { \
        const rect = node.getBoundingClientRect(); \
        if (rect.width === 0 || rect.height === 0) continue; \
        const overlay = document.createElement('div'); \
        overlay.style.position = 'absolute'; \
        overlay.style.left = `${rect.left + window.scrollX}px`; \
        overlay.style.top = `${rect.top + window.scrollY}px`; \
        overlay.style.width = `${rect.width}px`; \
        overlay.style.height = `${rect.height}px`; \
        overlay.style.border = '2px solid red'; \
        overlay.style.pointerEvents = 'none'; \
        overlay.style.zIndex = '999999'; \
        const label = document.createElement('span'); \
        label.style.position = 'absolute'; \
        label.style.background = 'red'; \
        label.style.color = 'white'; \
        label.style.fontSize = '12px'; \
        label.style.top = '-14px'; \
        label.style.left = '-2px'; \
        label.style.padding = '0 2px'; \
        label.innerText = counter++; \
        overlay.appendChild(label); \
        document.body.appendChild(overlay); \
    } \
    return true;"
        .into()
}

/// Generate computed style JS
#[must_use]
pub fn generate_computed_style_js(selector: &str, property: &str) -> String {
    format!(
        "const el = document.querySelector('{}'); \
        if (!el) return null; \
        return window.getComputedStyle(el).getPropertyValue('{}');",
        selector.replace('\'', "\\'"),
        property.replace('\'', "\\'")
    )
}

/// Generate wait for element JS
#[must_use]
pub fn generate_wait_element_js(selector: &str) -> String {
    format!(
        "return new Promise((resolve) => {{ \
            const el = document.querySelector('{}'); \
            if (el) {{ resolve(el); return; }} \
            const observer = new MutationObserver(() => {{ \
                const el = document.querySelector('{}'); \
                if (el) {{ observer.disconnect(); resolve(el); }} \
            }}); \
            observer.observe(document.body, {{ childList: true, subtree: true }}); \
        }});",
        selector.replace('\'', "\\'"),
        selector.replace('\'', "\\'")
    )
}

/// Generate wait for element gone JS
#[must_use]
pub fn generate_wait_gone_js(selector: &str) -> String {
    format!(
        "return new Promise((resolve) => {{ \
            const el = document.querySelector('{}'); \
            if (!el) {{ resolve(true); return; }} \
            const observer = new MutationObserver(() => {{ \
                const el = document.querySelector('{}'); \
                if (!el) {{ observer.disconnect(); resolve(true); }} \
            }}); \
            observer.observe(document.body, {{ childList: true, subtree: true }}); \
        }});",
        selector.replace('\'', "\\'"),
        selector.replace('\'', "\\'")
    )
}

/// Generate console capture JS
#[must_use]
pub fn generate_console_js(console_type: Option<&str>) -> String {
    match console_type {
        Some(t) => format!("return window.__captured_{t} || [];"),
        None => "return window.__captured_logs || [];".into(),
    }
}
