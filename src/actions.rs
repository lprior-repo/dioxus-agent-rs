#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Actions layer - Async `WebDriver` operations at the shell boundary
//! All I/O happens here

#![allow(dead_code)]
#![allow(clippy::too_many_lines)]

use crate::calculations::{
    generate_computed_style_js, generate_console_js, generate_css_injection_js,
    generate_dioxus_click_js, generate_dioxus_state_js, generate_element_screenshot_js,
    generate_hydration_wait_js, generate_keycombo_js, generate_keypress_js, generate_storage_js,
    generate_wait_element_js, generate_wait_gone_js,
};
use crate::data::{Commands, Config};
use anyhow::{Context, Result};
use fantoccini::elements::Element;
use fantoccini::ClientBuilder;
use fantoccini::Locator;
use serde_json::Value;
use std::time::Duration;

/// Execute the command - main entry point for actions
pub async fn execute_command(config: Config) -> Result<()> {
    // Build Chrome capabilities
    let mut caps = serde_json::Map::new();
    let chrome_opts = serde_json::json!({
        "args": ["headless", "no-sandbox", "disable-dev-shm-usage", "disable-gpu"]
    });
    caps.insert("goog:chromeOptions".to_string(), chrome_opts);

    // Connect to ChromeDriver
    let mut client = ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:4444")
        .await
        .context("Failed to connect to ChromeDriver")?;

    // Navigate to URL
    client
        .goto(&config.url)
        .await
        .with_context(|| format!("Failed to navigate to {}", config.url))?;

    // Give Dioxus time to hydrate
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Inject console capture script
    inject_console_capture(&mut client).await?;

    // Execute the command
    let result = execute_command_internal(&mut client, &config).await;

    // Clean up
    let _ = client.close().await;

    result
}

/// Inject console capture script
#[allow(clippy::unnecessary_mut_passed)]
async fn inject_console_capture(client: &mut fantoccini::Client) -> Result<()> {
    let js = "
        window.__captured_logs = [];
        ['log', 'warn', 'error', 'info', 'debug'].forEach(type => {
            window['__captured_' + type] = [];
            const original = console[type];
            console[type] = function(...args) {
                window['__captured_' + type].push(args.map(a => String(a)));
                original.apply(console, args);
            };
        });
    ";
    let _ = client.execute(js, vec![]).await;
    Ok(())
}

/// Internal command execution - handles all 50+ commands
#[allow(clippy::unnecessary_mut_passed)]
async fn execute_command_internal(client: &mut fantoccini::Client, config: &Config) -> Result<()> {
    match &config.command {
        // ============ Navigation ============
        Commands::Dom => {
            let source = client.source().await.context("Failed to get DOM")?;
            println!("{source}");
        }
        Commands::Title => {
            let title = client.title().await.context("Failed to get title")?;
            println!("{title}");
        }
        Commands::Url => {
            let url = client.current_url().await.context("Failed to get URL")?;
            println!("{url}");
        }
        Commands::Refresh => {
            client.refresh().await.context("Failed to refresh")?;
            println!("Page refreshed");
        }
        Commands::Back => {
            client.back().await.context("Failed to go back")?;
            println!("Navigated back");
        }
        Commands::Forward => {
            client.forward().await.context("Failed to go forward")?;
            println!("Navigated forward");
        }

        // ============ Element Interaction ============
        Commands::Click { selector } => {
            let el = client
                .find(Locator::Css(selector))
                .await
                .with_context(|| format!("Element not found: {selector}"))?;
            el.click().await.context("Failed to click element")?;
            println!("{selector}");
        }
        Commands::DoubleClick { selector } => {
            double_click(client, selector).await.context("Failed to double-click element")?;
            println!("{selector}");
        }
        Commands::RightClick { selector } => {
            right_click(client, selector).await.context("Failed to right-click element")?;
            println!("{selector}");
        }
        Commands::Hover { selector } => {
            hover(client, selector).await.context("Failed to hover element")?;
            println!("{selector}");
        }
        Commands::Text { selector, value } => {
            let el = client
                .find(Locator::Css(selector))
                .await
                .with_context(|| format!("Element not found: {selector}"))?;
            el.send_keys(value).await.context("Failed to set text")?;
            println!("{selector} {value}");
        }
        Commands::Clear { selector } => {
            let el = client
                .find(Locator::Css(selector))
                .await
                .with_context(|| format!("Element not found: {selector}"))?;
            let backspace: &str = &"\u{0008}".repeat(100);
            let delete: &str = &"\u{0001}\u{0003}";
            el.send_keys(backspace).await?; // Backspace
            el.send_keys(delete).await?; // Ctrl+A + Delete
            println!("{selector}");
        }
        Commands::Submit { selector } => {
            submit_form(client, selector).await.context("Failed to submit form")?;
            println!("{selector}");
        }
        Commands::Select { selector, value } => {
            select_option(client, selector, value).await.context("Failed to select option")?;
            println!("{selector} {value}");
        }
        Commands::Check { selector } => {
            check_element(client, selector).await.context("Failed to check element")?;
            println!("{selector}");
        }
        Commands::Uncheck { selector } => {
            uncheck_element(client, selector).await.context("Failed to uncheck element")?;
            println!("{selector}");
        }

        // ============ Element Queries ============
        Commands::GetText { selector } => {
            let el = client
                .find(Locator::Css(selector))
                .await
                .with_context(|| format!("Element not found: {selector}"))?;
            let text = el.text().await.context("Failed to get text")?;
            println!("{text}");
        }
        Commands::Attr { selector, attribute } => {
            let el = client
                .find(Locator::Css(selector))
                .await
                .with_context(|| format!("Element not found: {selector}"))?;
            let attr = el.attr(attribute.as_str()).await.context("Failed to get attribute")?;
            match attr {
                Some(v) => println!("{v}"),
                None => println!(),
            }
        }
        Commands::Classes { selector } => {
            let el = client
                .find(Locator::Css(selector))
                .await
                .with_context(|| format!("Element not found: {selector}"))?;
            let classes = el.attr("class").await.context("Failed to get classes")?;
            match classes {
                Some(c) => {
                    let class_list: Vec<&str> = c.split_whitespace().collect();
                    println!("{}", class_list.join(" "));
                }
                None => println!(),
            }
        }
        Commands::TagName { selector } => {
            let js = format!(
                "const el = document.querySelector('{}'); return el ? el.tagName.toLowerCase() : null;",
                selector.replace('\'', "\\'")
            );
            let result: Value = client.execute(&js, vec![]).await.context("Failed to get tag name")?;
            if let Some(name) = result.as_str() {
                println!("{name}");
            }
        }
        Commands::Visible { selector } => {
            let js = format!(
                "const el = document.querySelector('{}'); if (!el) return false; const style = window.getComputedStyle(el); return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';",
                selector.replace('\'', "\\'")
            );
            let result: Value = client.execute(&js, vec![]).await.context("Failed to check visibility")?;
            if let Some(b) = result.as_bool() {
                println!("{b}");
            } else {
                println!("false");
            }
        }
        Commands::Enabled { selector } => {
            let js = format!(
                "const el = document.querySelector('{}'); if (!el) return false; return !el.disabled;",
                selector.replace('\'', "\\'")
            );
            let result: Value = client.execute(&js, vec![]).await.context("Failed to check enabled")?;
            if let Some(b) = result.as_bool() {
                println!("{b}");
            } else {
                println!("false");
            }
        }
        Commands::Selected { selector } => {
            let js = format!(
                "const el = document.querySelector('{}'); if (!el) return false; return el.checked || el.selected;",
                selector.replace('\'', "\\'")
            );
            let result: Value = client.execute(&js, vec![]).await.context("Failed to check selected")?;
            if let Some(b) = result.as_bool() {
                println!("{b}");
            } else {
                println!("false");
            }
        }
        Commands::Count { selector } => {
            let count = client
                .find_all(Locator::Css(selector))
                .await
                .with_context(|| format!("Failed to count: {selector}"))?
                .len();
            println!("{count}");
        }
        Commands::FindAll { selector } => {
            let elements = client
                .find_all(Locator::Css(selector))
                .await
                .with_context(|| format!("Failed to find elements: {selector}"))?;
            for el in elements {
                let html = el.html(true).await.unwrap_or_default();
                println!("{html}");
            }
        }
        Commands::Exists { selector } => {
            let exists = client
                .find(Locator::Css(selector))
                .await
                .is_ok();
            println!("{exists}");
        }

        // ============ JavaScript & Execution ============
        Commands::Eval { js } => {
            let result = client
                .execute(js, vec![])
                .await
                .context("JavaScript execution failed")?;
            println!("{result}");
        }
        Commands::InjectCss { css } => {
            let js = generate_css_injection_js(css);
            client
                .execute(&js, vec![])
                .await
                .context("CSS injection failed")?;
            println!("CSS injected");
        }

        // ============ Screenshot ============
        Commands::Screenshot { path } => {
            let png = client.screenshot().await.context("Screenshot failed")?;
            std::fs::write(path, png).context("Failed to write screenshot")?;
            println!("{path}");
        }
        Commands::ElementScreenshot { selector, path } => {
            // First get element bounds via JS
            let js = generate_element_screenshot_js(selector);
            let bounds: Value = client
                .execute(&js, vec![])
                .await
                .context("Failed to get element bounds")?;
            if bounds.is_null() {
                anyhow::bail!("Element not found: {selector}");
            }
            // For now, take full screenshot - element-specific requires Chrome DevTools Protocol
            let png = client.screenshot().await.context("Screenshot failed")?;
            std::fs::write(path, png).context("Failed to write screenshot")?;
            println!("{path}");
        }

        // ============ Viewport & Scrolling ============
        Commands::Viewport { width, height } => {
            client
                .set_window_size(*width, *height)
                .await
                .context("Failed to set viewport")?;
            println!("{width} {height}");
        }
        Commands::Scroll { selector } => {
            scroll_into_view(client, selector).await.context("Failed to scroll to element")?;
            println!("{selector}");
        }
        Commands::ScrollBy { x, y } => {
            let js = format!("window.scrollBy({x}, {y}); return true;");
            client.execute(&js, vec![]).await.context("Scroll failed")?;
            println!("{x} {y}");
        }

        // ============ Keyboard ============
        Commands::Key { key } => {
            let js = generate_keypress_js(key);
            let _result: Value = client.execute(&js, vec![]).await?;
            println!("{key}");
        }
        Commands::KeyCombo { combo } => {
            let js = generate_keycombo_js(combo);
            let _result: Value = client.execute(&js, vec![]).await?;
            println!("{combo}");
        }

        // ============ Storage ============
        Commands::Cookies => {
            let cookies = client.get_all_cookies().await.context("Failed to get cookies")?;
            for cookie in cookies {
                println!(
                    "{}={}; Path={}; Domain={}",
                    cookie.name(),
                    cookie.value(),
                    cookie.path().unwrap_or("/"),
                    cookie.domain().unwrap_or("")
                );
            }
        }
        Commands::SetCookie {
            name,
            value,
            domain,
            path,
        } => {
            // Use JavaScript to set cookie for simplicity
            let domain_part = domain
                .as_ref()
                .map(|d| format!("; domain={d}"))
                .unwrap_or_default();
            let path_part = path
                .as_ref()
                .map(|p| format!("; path={p}"))
                .unwrap_or_default();
            let cookie_str = format!("{value}{domain_part}{path_part}");
            let js = format!(
                "document.cookie = '{}={}'; return true;",
                name.replace('\'', "\\'"),
                cookie_str.replace('\'', "\\'")
            );
            client.execute(&js, vec![]).await.context("Failed to set cookie")?;
            println!("{name}");
        }
        Commands::DeleteCookie { name } => {
            client.delete_cookie(name).await.context("Failed to delete cookie")?;
            println!("{name}");
        }
        Commands::LocalGet { key } => {
            let js = generate_storage_js("local", "get", Some(key), None);
            let result: Value = client.execute(&js, vec![]).await?;
            println!("{result}");
        }
        Commands::LocalSet { key, value } => {
            let js = generate_storage_js("local", "set", Some(key), Some(value));
            client.execute(&js, vec![]).await.context("Failed to set localStorage")?;
            println!("{key}");
        }
        Commands::LocalRemove { key } => {
            let js = generate_storage_js("local", "remove", Some(key), None);
            client.execute(&js, vec![]).await.context("Failed to remove localStorage")?;
            println!("{key}");
        }
        Commands::LocalClear => {
            let js = generate_storage_js("local", "clear", None, None);
            client.execute(&js, vec![]).await.context("Failed to clear localStorage")?;
            println!("cleared");
        }
        Commands::SessionGet { key } => {
            let js = generate_storage_js("session", "get", Some(key), None);
            let result: Value = client.execute(&js, vec![]).await?;
            println!("{result}");
        }
        Commands::SessionSet { key, value } => {
            let js = generate_storage_js("session", "set", Some(key), Some(value));
            client
                .execute(&js, vec![])
                .await
                .context("Failed to set sessionStorage")?;
            println!("{key}");
        }
        Commands::SessionClear => {
            let js = generate_storage_js("session", "clear", None, None);
            client
                .execute(&js, vec![])
                .await
                .context("Failed to clear sessionStorage")?;
            println!("cleared");
        }

        // ============ Console ============
        Commands::Console => {
            let js = generate_console_js(None);
            let result: Value = client.execute(&js, vec![]).await?;
            if let Some(arr) = result.as_array() {
                for entry in arr {
                    println!("{entry}");
                }
            }
        }
        Commands::ConsoleLog { r#type } => {
            let js = generate_console_js(Some(r#type));
            let result: Value = client.execute(&js, vec![]).await?;
            if let Some(arr) = result.as_array() {
                for entry in arr {
                    println!("{entry}");
                }
            }
        }

        // ============ Waiting ============
        Commands::Wait { selector } => {
            let js = generate_wait_element_js(selector);
            let _: Value = client
                .execute(&js, vec![])
                .await
                .with_context(|| format!("Timeout waiting for: {selector}"))?;
            println!("{selector}");
        }
        Commands::WaitGone { selector } => {
            let js = generate_wait_gone_js(selector);
            let _: Value = client
                .execute(&js, vec![])
                .await
                .with_context(|| format!("Timeout waiting for gone: {selector}"))?;
            println!("{selector}");
        }
        Commands::WaitNav => {
            // Wait for page to load - simple implementation
            tokio::time::sleep(Duration::from_millis(500)).await;
            println!("navigation complete");
        }
        Commands::WaitHydration => {
            let js = generate_hydration_wait_js();
            let _: Value = client.execute(&js, vec![]).await.context("Hydration wait failed")?;
            println!("hydrated");
        }

        // ============ Dioxus-Specific ============
        Commands::DioxusState => {
            let js = generate_dioxus_state_js();
            let result: Value = client.execute(&js, vec![]).await?;
            println!("{result}");
        }
        Commands::DioxusClick { target } => {
            let js = generate_dioxus_click_js(target);
            let result: Value = client
                .execute(&js, vec![])
                .await
                .context("Dioxus click failed")?;
            if result.as_bool() == Some(true) {
                println!("{target}");
            } else {
                anyhow::bail!("Target not found: {target}");
            }
        }

        // ============ Style ============
        Commands::Style { selector, property } => {
            let js = generate_computed_style_js(selector, property);
            let result: Value = client.execute(&js, vec![]).await?;
            if result.is_null() {
                println!();
            } else {
                println!("{result}");
            }
        }
    }

    Ok(())
}

// ============ Helper functions for WebDriver operations ============

/// Double-click using JavaScript fallback
async fn double_click(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); \
         if (el) {{ el.dispatchEvent(new MouseEvent('dblclick', {{ bubbles: true, cancelable: true, view: window }})); }}",
        selector.replace('\'', "\\'")
    );
    client.execute(&js, vec![]).await.map_err(anyhow::Error::from).map(|_| ())
}

/// Right-click (context menu) using JavaScript fallback
async fn right_click(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); \
         if (el) {{ el.dispatchEvent(new MouseEvent('contextmenu', {{ bubbles: true, cancelable: true, view: window }})); }}",
        selector.replace('\'', "\\'")
    );
    client.execute(&js, vec![]).await.map_err(anyhow::Error::from).map(|_| ())
}

/// Hover using JavaScript fallback
async fn hover(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); \
         if (el) {{ el.dispatchEvent(new MouseEvent('mouseover', {{ bubbles: true, cancelable: true, view: window }})); }}",
        selector.replace('\'', "\\'")
    );
    client.execute(&js, vec![]).await.map_err(anyhow::Error::from).map(|_| ())
}

/// Scroll element into view using JavaScript
async fn scroll_into_view(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); if (el) {{ el.scrollIntoView({{ behavior: 'smooth', block: 'center' }}); }}",
        selector.replace('\'', "\\'")
    );
    client.execute(&js, vec![]).await.map_err(anyhow::Error::from).map(|_| ())
}

/// Submit form using JavaScript
async fn submit_form(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); if (el) {{ el.dispatchEvent(new Event('submit', {{ bubbles: true, cancelable: true }})); }}",
        selector.replace('\'', "\\'")
    );
    client.execute(&js, vec![]).await.map_err(anyhow::Error::from).map(|_| ())
}

/// Select option in dropdown using JavaScript
async fn select_option(client: &fantoccini::Client, selector: &str, value: &str) -> Result<()> {
    let escaped_value = value.replace('\'', "\\'");
    let js = format!(
        "const sel = document.querySelector('{}'); \
         if (sel) {{ \
             for (let opt of sel.options) {{ \
                 if (opt.value === '{}') {{ opt.selected = true; break; }} \
             }} \
             sel.dispatchEvent(new Event('change', {{ bubbles: true }})); \
         }}",
        selector.replace('\'', "\\'"),
        escaped_value
    );
    client.execute(&js, vec![]).await.map_err(anyhow::Error::from).map(|_| ())
}

/// Check checkbox/radio using JavaScript
async fn check_element(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); if (el && !el.checked) {{ el.checked = true; el.dispatchEvent(new Event('change', {{ bubbles: true }})); }}",
        selector.replace('\'', "\\'")
    );
    client.execute(&js, vec![]).await.map_err(anyhow::Error::from).map(|_| ())
}

/// Uncheck checkbox using JavaScript
async fn uncheck_element(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); if (el && el.checked) {{ el.checked = false; el.dispatchEvent(new Event('change', {{ bubbles: true }})); }}",
        selector.replace('\'', "\\'")
    );
    client.execute(&js, vec![]).await.map_err(anyhow::Error::from).map(|_| ())
}

/// Convert element to JSON for JS execution
trait ElementExt {
    fn to_json(&self) -> Value;
}

impl ElementExt for Element {
    fn to_json(&self) -> Value {
        serde_json::json!({ "ELEMENT": format!("{:?}", self) })
    }
}
