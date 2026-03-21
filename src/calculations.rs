#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Calculations layer - pure functions for validation and transformation

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
        _ => format!("return {{ key: '{}' }}", escape_js_string(key)),
    }
}

pub fn generate_keycombo_js(combo: &str) -> String {
    let parts: Vec<String> = combo
        .split('+')
        .map(str::trim)
        .map(std::string::ToString::to_string)
        .collect();
    let mut modifiers = Vec::new();
    let mut key: Option<String> = None;

    for part in parts {
        match part.to_lowercase().as_str() {
            "control" | "ctrl" => modifiers.push("ctrlKey"),
            "shift" => modifiers.push("shiftKey"),
            "alt" => modifiers.push("altKey"),
            "meta" | "cmd" | "command" => modifiers.push("metaKey"),
            _ => key = Some(part.clone()),
        }
    }

    let k = escape_js_string(&key.unwrap_or_default());
    if modifiers.is_empty() {
        format!("return {{ key: '{k}', ctrlKey: false, shiftKey: false, altKey: false, metaKey: false }}")
    } else {
        format!("return {{ key: '{k}', {} }}", modifiers.join(", "))
    }
}

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
        ("local", "set", Some(k), Some(v)) => format!(
            "localStorage.setItem('{}', '{}'); return true;",
            escape_js_string(k),
            escape_js_string(v)
        ),
        ("local", "remove", Some(k), _) => format!(
            "localStorage.removeItem('{}'); return true;",
            escape_js_string(k)
        ),
        ("local", "clear", None, _) => "localStorage.clear(); return true;".into(),
        ("session", "get", Some(k), _) => {
            format!("return sessionStorage.getItem('{}');", escape_js_string(k))
        }
        ("session", "set", Some(k), Some(v)) => format!(
            "sessionStorage.setItem('{}', '{}'); return true;",
            escape_js_string(k),
            escape_js_string(v)
        ),
        ("session", "clear", None, _) => "sessionStorage.clear(); return true;".into(),
        _ => "return null;".into(),
    }
}

#[must_use]
pub fn generate_css_injection_js(css: &str) -> String {
    format!(
        r"const style = document.createElement('style'); style.textContent = '{}'; document.head.appendChild(style); return true;",
        escape_js_string(css)
    )
}

#[must_use]
pub fn generate_dioxus_click_js(target: &str) -> String {
    format!(
        r#"const el = document.querySelector('[data-target="{}"]'); if (el) {{ el.click(); return true; }} return false;"#,
        escape_js_string(target)
    )
}

#[must_use]
pub fn generate_dioxus_state_js() -> String {
    r"if (typeof window.getDioxusState === 'function') { return window.getDioxusState(); }
     if (typeof window.__DX_STATE__ !== 'undefined') { return window.__DX_STATE__; }
     const states = [];
     for (let key in window) {
         if (key.startsWith('__dx') || key.startsWith('__dioxus')) {
             try { states.push({ [key]: window[key] }); } catch(e) {}
         }
     }
     return states.length > 0 ? states : null;"
        .into()
}

#[must_use]
pub fn generate_hydration_wait_js() -> String {
    r#"return new Promise((resolve) => {
        const checkReady = () => {
            const hasSuspense = document.querySelector('[aria-busy="true"]');
            const hasHydrated = document.body.hasAttribute('data-hydrated') || document.querySelector('#main') || document.body.innerHTML.length > 50;
            if (hasHydrated && !hasSuspense) { resolve(true); return true; }
            return false;
        };
        if (checkReady()) return;
        const observer = new MutationObserver(() => {
            if (checkReady()) { observer.disconnect(); resolve(true); }
        });
        observer.observe(document.body, { childList: true, subtree: true, attributes: true });
        setTimeout(() => { observer.disconnect(); resolve(true); }, 10000);
    });"#.into()
}

#[must_use]
pub fn generate_semantic_tree_js() -> String {
    r#"function getSemanticTree(root) {
        const tree = [];
        const iter = document.createNodeIterator(root, NodeFilter.SHOW_ELEMENT, {
            acceptNode: (node) => {
                const tag = node.tagName.toLowerCase();
                const interactable = ['a', 'button', 'input', 'select', 'textarea'].includes(tag) || node.hasAttribute('role') || node.hasAttribute('tabindex');
                if (!interactable) return NodeFilter.FILTER_SKIP;
                const style = window.getComputedStyle(node);
                if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return NodeFilter.FILTER_REJECT;
                return NodeFilter.FILTER_ACCEPT;
            }
        });
        let node;
        while (node = iter.nextNode()) {
            const tag = node.tagName.toLowerCase();
            let label = node.innerText || node.value || node.getAttribute('aria-label') || node.getAttribute('title') || '';
            label = label.substring(0, 50).replace(/\n/g, ' ').trim();
            let id = node.id ? '#' + node.id : '';
            let cls = node.className ? '.' + node.className.split(' ').join('.') : '';
            let sel = id ? id : (cls ? tag + cls : tag);
            tree.push(`[${tag.toUpperCase()}] ${sel} "${label}"`);
        }
        return tree.join('\n');
    }
    return getSemanticTree(document.body);"#.into()
}

#[must_use]
pub fn generate_screenshot_annotated_js() -> String {
    r"const iter = document.createNodeIterator(document.body, NodeFilter.SHOW_ELEMENT, {
        acceptNode: (node) => {
            const tag = node.tagName.toLowerCase();
            const interactable = ['a', 'button', 'input', 'select', 'textarea'].includes(tag) || node.hasAttribute('role') || node.hasAttribute('tabindex');
            if (!interactable) return NodeFilter.FILTER_SKIP;
            const style = window.getComputedStyle(node);
            if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return NodeFilter.FILTER_REJECT;
            return NodeFilter.FILTER_ACCEPT;
        }
    });
    let node; let counter = 1;
    while (node = iter.nextNode()) {
        const rect = node.getBoundingClientRect();
        if (rect.width === 0 || rect.height === 0) continue;
        const overlay = document.createElement('div');
        overlay.style.position = 'absolute';
        overlay.style.left = `${rect.left + window.scrollX}px`;
        overlay.style.top = `${rect.top + window.scrollY}px`;
        overlay.style.width = `${rect.width}px`;
        overlay.style.height = `${rect.height}px`;
        overlay.style.border = '2px solid red';
        overlay.style.pointerEvents = 'none';
        overlay.style.zIndex = '999999';
        const label = document.createElement('span');
        label.style.position = 'absolute';
        label.style.background = 'red';
        label.style.color = 'white';
        label.style.fontSize = '12px';
        label.style.top = '-14px';
        label.style.left = '-2px';
        label.style.padding = '0 2px';
        label.innerText = counter++;
        overlay.appendChild(label);
        document.body.appendChild(overlay);
    }
    return true;".into()
}

#[must_use]
pub fn generate_computed_style_js(selector: &str, property: &str) -> String {
    format!(
        r"const el = document.querySelector('{}'); if (!el) return null; return window.getComputedStyle(el).getPropertyValue('{}');",
        escape_js_string(selector),
        escape_js_string(property)
    )
}

#[must_use]
pub fn generate_wait_element_js(selector: &str) -> String {
    let escaped = escape_js_string(selector);
    format!(
        r"return new Promise((resolve) => {{
            const el = document.querySelector('{escaped}');
            if (el) {{ resolve(el); return; }}
            const observer = new MutationObserver(() => {{
                const el = document.querySelector('{escaped}');
                if (el) {{ observer.disconnect(); resolve(el); }}
            }});
            observer.observe(document.body, {{ childList: true, subtree: true }});
        }});",
    )
}

#[must_use]
pub fn generate_wait_stable_js(selector: &str) -> String {
    let escaped = escape_js_string(selector);
    format!(
        r"return new Promise((resolve) => {{
            const checkStable = async () => {{
                const el = document.querySelector('{escaped}');
                if (!el) return false;
                const style = window.getComputedStyle(el);
                if (style.display === 'none' || style.visibility === 'hidden') return false;
                const rect1 = el.getBoundingClientRect();
                await new Promise(r => requestAnimationFrame(r));
                await new Promise(r => requestAnimationFrame(r));
                const rect2 = el.getBoundingClientRect();
                return rect1.x === rect2.x && rect1.y === rect2.y && rect1.width === rect2.width && rect1.height === rect2.height;
            }};
            const run = async () => {{
                for (let i = 0; i < 50; i++) {{
                    if (await checkStable()) {{ resolve(true); return; }}
                    await new Promise(r => setTimeout(r, 100));
                }}
                resolve(false);
            }};
            run();
        }});"
    )
}

#[must_use]
pub fn generate_wait_gone_js(selector: &str) -> String {
    let escaped = escape_js_string(selector);
    format!(
        r"return new Promise((resolve) => {{
            const el = document.querySelector('{escaped}');
            if (!el) {{ resolve(true); return; }}
            const observer = new MutationObserver(() => {{
                const el = document.querySelector('{escaped}');
                if (!el) {{ observer.disconnect(); resolve(true); }}
            }});
            observer.observe(document.body, {{ childList: true, subtree: true }});
        }});",
    )
}

#[must_use]
pub fn generate_console_js(console_type: Option<&str>) -> String {
    console_type.map_or_else(
        || "return window.__captured_logs || [];".into(),
        |t| format!("return window.__captured_{} || [];", escape_js_string(t)),
    )
}

#[must_use]
pub fn generate_fuzzy_click_js(text: &str) -> String {
    let escaped = escape_js_string(&text.to_lowercase());
    format!(
        r"const targetText = '{escaped}';
        const iter = document.createNodeIterator(document.body, NodeFilter.SHOW_ELEMENT, {{
            acceptNode: (node) => {{
                const tag = node.tagName.toLowerCase();
                const interactable = ['a', 'button', 'input', 'select', 'textarea'].includes(tag) || node.hasAttribute('role') || node.hasAttribute('tabindex');
                if (!interactable) return NodeFilter.FILTER_SKIP;
                const style = window.getComputedStyle(node);
                if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return NodeFilter.FILTER_REJECT;
                return NodeFilter.FILTER_ACCEPT;
            }}
        }});
        let node;
        let bestMatch = null;
        let exactMatch = false;
        while (node = iter.nextNode()) {{
            const nodeText = (node.innerText || node.value || node.getAttribute('aria-label') || node.getAttribute('title') || '').toLowerCase();
            if (nodeText === targetText) {{
                bestMatch = node;
                exactMatch = true;
                break;
            }} else if (!exactMatch && nodeText.includes(targetText)) {{
                bestMatch = node;
            }}
        }}
        if (bestMatch) {{
            bestMatch.click();
            const id = bestMatch.id ? '#' + bestMatch.id : '';
            const cls = bestMatch.className ? '.' + bestMatch.className.split(' ').join('.') : '';
            return id ? id : (cls ? bestMatch.tagName.toLowerCase() + cls : bestMatch.tagName.toLowerCase());
        }}
        return null;"
    )
}

#[must_use]
pub fn generate_network_idle_js() -> String {
    r"return new Promise((resolve) => {
        let idleCycles = 0;
        const check = setInterval(() => {
            if (window.__active_requests === 0 || typeof window.__active_requests === 'undefined') {
                idleCycles++;
                if (idleCycles >= 5) {
                    clearInterval(check);
                    resolve(true);
                }
            } else {
                idleCycles = 0;
            }
        }, 100);
        setTimeout(() => { clearInterval(check); resolve(false); }, 15000);
    });"
    .into()
}

#[must_use]
pub fn generate_scroll_to_text_js(container: &str, text: &str) -> String {
    format!(
        r"return new Promise(async (resolve) => {{
            const container = document.querySelector('{}');
            if (!container) {{ resolve(null); return; }}
            const targetText = '{}'.toLowerCase();
            let attempts = 0;
            while (attempts < 20) {{
                const found = Array.from(container.querySelectorAll('*')).some(el =>
                    el.children.length === 0 && el.textContent.toLowerCase().includes(targetText)
                );
                if (found) {{ resolve(true); return; }}
                const oldScroll = container.scrollTop;
                container.scrollTop += container.clientHeight;
                if (container.scrollTop === oldScroll) {{ resolve(false); return; }}
                await new Promise(r => setTimeout(r, 200));
                attempts++;
            }}
            resolve(false);
        }});",
        escape_js_string(container),
        escape_js_string(text)
    )
}

#[must_use]
pub fn generate_extract_table_js(selector: &str) -> String {
    format!(
        r"const table = document.querySelector('{}');
        if (!table) return null;
        const headers = Array.from(table.querySelectorAll('th')).map(th => th.innerText.trim());
        if (headers.length === 0) return null;
        const rows = Array.from(table.querySelectorAll('tbody tr, tr:not(:first-child)'));
        return rows.map(row => {{
            const cells = Array.from(row.querySelectorAll('td'));
            const obj = {{}};
            headers.forEach((h, i) => {{
                obj[h] = cells[i] ? cells[i].innerText.trim() : '';
            }});
            return obj;
        }});",
        escape_js_string(selector)
    )
}
