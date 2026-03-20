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
    generate_hydration_wait_js, generate_keycombo_js, generate_keypress_js,
    generate_screenshot_annotated_js, generate_semantic_tree_js, generate_storage_js,
    generate_wait_element_js, generate_wait_gone_js,
};
use crate::data::{Commands, Config};
use anyhow::{Context, Result};
use fantoccini::ClientBuilder;
use fantoccini::Locator;
use fantoccini::elements::Element;
use serde_json::Value;
use std::time::Duration;

/// Execute the command - main entry point for actions
pub async fn execute_command(config: Config) -> Result<()> {
    // Build Chrome capabilities
    let mut caps = serde_json::Map::new();
    let mut args = vec!["no-sandbox", "disable-dev-shm-usage", "disable-gpu"];
    if !config.no_headless {
        args.push("headless");
    }
    let chrome_opts = serde_json::json!({
        "args": args
    });
    caps.insert("goog:chromeOptions".to_string(), chrome_opts);

    // Connect to ChromeDriver
    let mut client = ClientBuilder::native()
        .capabilities(caps)
        .connect(&config.webdriver_url)
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

    if config.auto_wait {
        let js = generate_hydration_wait_js();
        let _ = client.execute(&js, vec![]).await;
    }

    // Execute the command
    let result = if matches!(config.command, Commands::Repl) {
        run_repl(&mut client).await?;
        Ok(serde_json::Value::Null)
    } else {
        match tokio::time::timeout(
            config.timeout,
            execute_command_internal(&mut client, &config.command),
        )
        .await
        {
            Ok(res) => res,
            Err(_) => Err(anyhow::anyhow!(
                "Command execution timed out after {:?}",
                config.timeout
            )),
        }
    };

    // Clean up
    let _ = client.close().await;

    if config.json {
        let (success, data, error) = match result {
            Ok(v) => (true, v, None),
            Err(e) => (false, serde_json::Value::Null, Some(e.to_string())),
        };
        let cmd_str = format!("{:?}", config.command)
            .split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_string();
        let output = crate::data::CommandOutput {
            success,
            command: cmd_str,
            target: None,
            data,
            error,
            logs: vec![],
        };
        println!("{}", serde_json::to_string(&output).unwrap());
        Ok(())
    } else {
        match result {
            Ok(serde_json::Value::String(s)) => {
                println!("{s}");
                Ok(())
            }
            Ok(v) if v.is_null() => Ok(()),
            Ok(v) => {
                println!("{v}");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

async fn run_repl(client: &mut fantoccini::Client) -> Result<()> {
    let current_url = client
        .current_url()
        .await
        .map(|u| u.to_string())
        .unwrap_or_default();
    println!("Dioxus Agent REPL connected to {current_url}");
    println!("Type 'help' for commands, 'exit' to quit.");

    // Using rustyline for REPL inside tokio spawn_blocking
    let mut rl = rustyline::DefaultEditor::new()?;

    loop {
        let readline = tokio::task::block_in_place(|| rl.readline("dioxus> "));
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                if input == "exit" || input == "quit" {
                    break;
                }
                let _ = rl.add_history_entry(input);

                if let Some(mut args) = shlex::split(input) {
                    args.insert(0, "dioxus-agent-rs".to_string());

                    match clap::Parser::try_parse_from(args) {
                        Ok(crate::data::Cli { command: cmd, .. }) => {
                            if matches!(cmd, Commands::Repl) {
                                println!("Already in REPL mode.");
                                continue;
                            }
                            if let Err(e) = execute_command_internal(client, &cmd).await {
                                println!("Error: {e}");
                            }
                        }
                        Err(e) => {
                            println!("{e}");
                        }
                    }
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {err:?}");
                break;
            }
        }
    }
    Ok(())
}

/// Inject console capture script
#[allow(clippy::unnecessary_mut_passed)]
async fn inject_console_capture(client: &mut fantoccini::Client) -> Result<()> {
    let js = "
        window.__captured_logs = [];
        window.__captured_network = [];
        ['log', 'warn', 'error', 'info', 'debug'].forEach(type => {
            window['__captured_' + type] = [];
            const original = console[type];
            console[type] = function(...args) {
                window['__captured_' + type].push(args.map(a => String(a)));
                original.apply(console, args);
            };
        });
        const originalFetch = window.fetch;
        window.fetch = async function(...args) {
            const url = typeof args[0] === 'string' ? args[0] : (args[0] && args[0].url) || 'unknown';
            window.__captured_network.push({ type: 'fetch', url: url });
            return originalFetch.apply(this, args);
        };
        const originalXhrOpen = XMLHttpRequest.prototype.open;
        XMLHttpRequest.prototype.open = function(method, url, ...rest) {
            window.__captured_network.push({ type: 'xhr', method, url });
            return originalXhrOpen.apply(this, [method, url, ...rest]);
        };
    ";
    let _ = client.execute(js, vec![]).await;
    Ok(())
}

/// Internal command execution - handles all 50+ commands
#[allow(clippy::unnecessary_mut_passed)]
async fn execute_command_internal(
    client: &mut fantoccini::Client,
    command: &Commands,
) -> Result<Value> {
    match command {
        // ============ Navigation ============
        Commands::Dom => {
            let source = client.source().await.context("Failed to get DOM")?;
            Ok(serde_json::json!(source))
        }
        Commands::Title => {
            let title = client.title().await.context("Failed to get title")?;
            Ok(serde_json::json!(title))
        }
        Commands::Url => {
            let url = client.current_url().await.context("Failed to get URL")?;
            Ok(serde_json::json!(url.to_string()))
        }
        Commands::Refresh => {
            client.refresh().await.context("Failed to refresh")?;
            Ok(serde_json::json!("Page refreshed"))
        }
        Commands::Back => {
            client.back().await.context("Failed to go back")?;
            Ok(serde_json::json!("Navigated back"))
        }
        Commands::Forward => {
            client.forward().await.context("Failed to go forward")?;
            Ok(serde_json::json!("Navigated forward"))
        }

        // ============ Element Interaction ============
        Commands::Click { selector } => {
            let el = client
                .find(Locator::Css(selector))
                .await
                .with_context(|| format!("Element not found: {selector}"))?;
            el.click().await.context("Failed to click element")?;
            Ok(serde_json::json!(selector))
        }
        Commands::DoubleClick { selector } => {
            double_click(client, selector)
                .await
                .context("Failed to double-click element")?;
            Ok(serde_json::json!(selector))
        }
        Commands::RightClick { selector } => {
            right_click(client, selector)
                .await
                .context("Failed to right-click element")?;
            Ok(serde_json::json!(selector))
        }
        Commands::Hover { selector } => {
            hover(client, selector)
                .await
                .context("Failed to hover element")?;
            Ok(serde_json::json!(selector))
        }
        Commands::Text { selector, value } => {
            let el = client
                .find(Locator::Css(selector))
                .await
                .with_context(|| format!("Element not found: {selector}"))?;
            el.send_keys(value).await.context("Failed to set text")?;
            Ok(serde_json::json!(format!("{selector} {value}")))
        }
        Commands::Clear { selector } => {
            let el = client
                .find(Locator::Css(selector))
                .await
                .with_context(|| format!("Element not found: {selector}"))?;
            let backspace: &str = &"\u{0008}".repeat(100);
            let delete: &str = "\u{0001}\u{0003}";
            el.send_keys(backspace).await?; // Backspace
            el.send_keys(delete).await?; // Ctrl+A + Delete
            Ok(serde_json::json!(selector))
        }
        Commands::Submit { selector } => {
            submit_form(client, selector)
                .await
                .context("Failed to submit form")?;
            Ok(serde_json::json!(selector))
        }
        Commands::Select { selector, value } => {
            select_option(client, selector, value)
                .await
                .context("Failed to select option")?;
            Ok(serde_json::json!(format!("{selector} {value}")))
        }
        Commands::Check { selector } => {
            check_element(client, selector)
                .await
                .context("Failed to check element")?;
            Ok(serde_json::json!(selector))
        }
        Commands::Uncheck { selector } => {
            uncheck_element(client, selector)
                .await
                .context("Failed to uncheck element")?;
            Ok(serde_json::json!(selector))
        }

        // ============ Element Queries ============
        Commands::GetText { selector } => {
            let el = client
                .find(Locator::Css(selector))
                .await
                .with_context(|| format!("Element not found: {selector}"))?;
            let text = el.text().await.context("Failed to get text")?;
            Ok(serde_json::json!(text))
        }
        Commands::Attr {
            selector,
            attribute,
        } => {
            let el = client
                .find(Locator::Css(selector))
                .await
                .with_context(|| format!("Element not found: {selector}"))?;
            let attr = el
                .attr(attribute.as_str())
                .await
                .context("Failed to get attribute")?;
            match attr {
                Some(v) => Ok(serde_json::json!(v)),
                None => Ok(serde_json::Value::Null),
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
                    Ok(serde_json::json!(format!("{}", class_list.join(" "))))
                }
                None => Ok(serde_json::Value::Null),
            }
        }
        Commands::TagName { selector } => {
            let js = format!(
                "const el = document.querySelector('{}'); return el ? el.tagName.toLowerCase() : null;",
                selector.replace('\'', "\\'")
            );
            let result: Value = client
                .execute(&js, vec![])
                .await
                .context("Failed to get tag name")?;
            if let Some(name) = result.as_str() {
                Ok(serde_json::json!(name))
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        Commands::Visible { selector } => {
            let js = format!(
                "const el = document.querySelector('{}'); if (!el) return false; const style = window.getComputedStyle(el); return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';",
                selector.replace('\'', "\\'")
            );
            let result: Value = client
                .execute(&js, vec![])
                .await
                .context("Failed to check visibility")?;
            if let Some(b) = result.as_bool() {
                Ok(serde_json::json!(b))
            } else {
                Ok(serde_json::json!("false"))
            }
        }
        Commands::Enabled { selector } => {
            let js = format!(
                "const el = document.querySelector('{}'); if (!el) return false; return !el.disabled;",
                selector.replace('\'', "\\'")
            );
            let result: Value = client
                .execute(&js, vec![])
                .await
                .context("Failed to check enabled")?;
            if let Some(b) = result.as_bool() {
                Ok(serde_json::json!(b))
            } else {
                Ok(serde_json::json!("false"))
            }
        }
        Commands::Selected { selector } => {
            let js = format!(
                "const el = document.querySelector('{}'); if (!el) return false; return el.checked || el.selected;",
                selector.replace('\'', "\\'")
            );
            let result: Value = client
                .execute(&js, vec![])
                .await
                .context("Failed to check selected")?;
            if let Some(b) = result.as_bool() {
                Ok(serde_json::json!(b))
            } else {
                Ok(serde_json::json!("false"))
            }
        }
        Commands::Count { selector } => {
            let count = client
                .find_all(Locator::Css(selector))
                .await
                .with_context(|| format!("Failed to count: {selector}"))?
                .len();
            Ok(serde_json::json!(count))
        }
        Commands::FindAll { selector } => {
            let elements = client
                .find_all(Locator::Css(selector))
                .await
                .with_context(|| format!("Failed to find elements: {selector}"))?;
            let mut results = Vec::new();
            for el in elements {
                let html = el.html(true).await.unwrap_or_default();
                results.push(html);
            }
            Ok(serde_json::json!(results))
        }
        Commands::Exists { selector } => {
            let exists = client.find(Locator::Css(selector)).await.is_ok();
            Ok(serde_json::json!(exists))
        }

        // ============ JavaScript & Execution ============
        Commands::Eval { js } => {
            let result = client
                .execute(js, vec![])
                .await
                .context("JavaScript execution failed")?;
            Ok(serde_json::json!(result))
        }
        Commands::InjectCss { css } => {
            let js = generate_css_injection_js(css);
            client
                .execute(&js, vec![])
                .await
                .context("CSS injection failed")?;
            Ok(serde_json::json!("CSS injected"))
        }

        // ============ Screenshot ============
        Commands::Screenshot { path } => {
            let png = client.screenshot().await.context("Screenshot failed")?;
            std::fs::write(path, png).context("Failed to write screenshot")?;
            Ok(serde_json::json!(path))
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
            Ok(serde_json::json!(path))
        }

        // ============ Viewport & Scrolling ============
        Commands::Viewport { width, height } => {
            client
                .set_window_size(*width, *height)
                .await
                .context("Failed to set viewport")?;
            Ok(serde_json::json!(format!("{width} {height}")))
        }
        Commands::Scroll { selector } => {
            scroll_into_view(client, selector)
                .await
                .context("Failed to scroll to element")?;
            Ok(serde_json::json!(selector))
        }
        Commands::ScrollBy { x, y } => {
            let js = format!("window.scrollBy({x}, {y}); return true;");
            client.execute(&js, vec![]).await.context("Scroll failed")?;
            Ok(serde_json::json!(format!("{x} {y}")))
        }

        // ============ Keyboard ============
        Commands::Key { key } => {
            let js = generate_keypress_js(key);
            let _result: Value = client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(key))
        }
        Commands::KeyCombo { combo } => {
            let js = generate_keycombo_js(combo);
            let _result: Value = client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(combo))
        }

        // ============ Storage ============
        Commands::Cookies => {
            let cookies = client
                .get_all_cookies()
                .await
                .context("Failed to get cookies")?;
            let mut results = Vec::new();
            for cookie in cookies {
                results.push(format!(
                    "{}={}; Path={}; Domain={}",
                    cookie.name(),
                    cookie.value(),
                    cookie.path().unwrap_or("/"),
                    cookie.domain().unwrap_or("")
                ));
            }
            Ok(serde_json::json!(results))
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
            client
                .execute(&js, vec![])
                .await
                .context("Failed to set cookie")?;
            Ok(serde_json::json!(name))
        }
        Commands::DeleteCookie { name } => {
            client
                .delete_cookie(name)
                .await
                .context("Failed to delete cookie")?;
            Ok(serde_json::json!(name))
        }
        Commands::LocalGet { key } => {
            let js = generate_storage_js("local", "get", Some(key), None);
            let result: Value = client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(result))
        }
        Commands::LocalSet { key, value } => {
            let js = generate_storage_js("local", "set", Some(key), Some(value));
            client
                .execute(&js, vec![])
                .await
                .context("Failed to set localStorage")?;
            Ok(serde_json::json!(key))
        }
        Commands::LocalRemove { key } => {
            let js = generate_storage_js("local", "remove", Some(key), None);
            client
                .execute(&js, vec![])
                .await
                .context("Failed to remove localStorage")?;
            Ok(serde_json::json!(key))
        }
        Commands::LocalClear => {
            let js = generate_storage_js("local", "clear", None, None);
            client
                .execute(&js, vec![])
                .await
                .context("Failed to clear localStorage")?;
            Ok(serde_json::json!("cleared"))
        }
        Commands::SessionGet { key } => {
            let js = generate_storage_js("session", "get", Some(key), None);
            let result: Value = client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(result))
        }
        Commands::SessionSet { key, value } => {
            let js = generate_storage_js("session", "set", Some(key), Some(value));
            client
                .execute(&js, vec![])
                .await
                .context("Failed to set sessionStorage")?;
            Ok(serde_json::json!(key))
        }
        Commands::SessionClear => {
            let js = generate_storage_js("session", "clear", None, None);
            client
                .execute(&js, vec![])
                .await
                .context("Failed to clear sessionStorage")?;
            Ok(serde_json::json!("cleared"))
        }

        // ============ Console ============
        Commands::Console => {
            let js = generate_console_js(None);
            let result: Value = client.execute(&js, vec![]).await?;
            let mut results = Vec::new();
            if let Some(arr) = result.as_array() {
                for entry in arr {
                    results.push(entry.clone());
                }
            }
            Ok(serde_json::json!(results))
        }
        Commands::ConsoleLog { r#type } => {
            let js = generate_console_js(Some(r#type));
            let result: Value = client.execute(&js, vec![]).await?;
            let mut results = Vec::new();
            if let Some(arr) = result.as_array() {
                for entry in arr {
                    results.push(entry.clone());
                }
            }
            Ok(serde_json::json!(results))
        }

        // ============ Waiting ============
        Commands::Wait { selector } => {
            let js = generate_wait_element_js(selector);
            let _: Value = client
                .execute(&js, vec![])
                .await
                .with_context(|| format!("Timeout waiting for: {selector}"))?;
            Ok(serde_json::json!(selector))
        }
        Commands::WaitGone { selector } => {
            let js = generate_wait_gone_js(selector);
            let _: Value = client
                .execute(&js, vec![])
                .await
                .with_context(|| format!("Timeout waiting for gone: {selector}"))?;
            Ok(serde_json::json!(selector))
        }
        Commands::WaitNav => {
            // Wait for page to load - simple implementation
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok(serde_json::json!("navigation complete"))
        }
        Commands::WaitHydration => {
            let js = generate_hydration_wait_js();
            let _: Value = client
                .execute(&js, vec![])
                .await
                .context("Hydration wait failed")?;
            Ok(serde_json::json!("hydrated"))
        }

        // ============ Dioxus-Specific ============
        Commands::DioxusState => {
            let js = generate_dioxus_state_js();
            let result: Value = client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(result))
        }
        Commands::DioxusClick { target } => {
            let js = generate_dioxus_click_js(target);
            let result: Value = client
                .execute(&js, vec![])
                .await
                .context("Dioxus click failed")?;
            if result.as_bool() == Some(true) {
                Ok(serde_json::json!(target))
            } else {
                anyhow::bail!("Target not found: {target}");
            }
        }

        // ============ AI Agent Extended ============
        Commands::Upload { selector, path } => {
            let el = client
                .find(Locator::Css(selector))
                .await
                .with_context(|| format!("Element not found: {selector}"))?;

            // Resolve to absolute path if needed
            let abs_path = std::fs::canonicalize(path).context("Invalid path")?;
            el.send_keys(abs_path.to_str().unwrap_or(""))
                .await
                .context("Failed to upload file")?;
            Ok(serde_json::json!(selector))
        }
        Commands::FillForm { json_data } => {
            let map: serde_json::Map<String, Value> = serde_json::from_str(json_data)?;
            let mut results = Vec::new();
            for (selector, val) in map {
                let text_val = match val {
                    Value::String(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                let el = client
                    .find(Locator::Css(&selector))
                    .await
                    .with_context(|| format!("Element not found: {selector}"))?;

                // Clear first
                let backspace: &str = &"\u{0008}".repeat(100);
                let delete: &str = "\u{0001}\u{0003}";
                el.send_keys(backspace).await?;
                el.send_keys(delete).await?;

                // Send new text
                el.send_keys(&text_val)
                    .await
                    .context("Failed to set text")?;
                results.push(selector);
            }
            Ok(serde_json::json!(results))
        }
        Commands::NetworkLogs => {
            let result: Value = client
                .execute("return window.__captured_network || [];", vec![])
                .await?;
            let mut results = Vec::new();
            if let Some(arr) = result.as_array() {
                for entry in arr {
                    results.push(entry.clone());
                }
            }
            Ok(serde_json::json!(results))
        }
        Commands::AssertText { selector, expected } => {
            let el = client
                .find(Locator::Css(selector))
                .await
                .with_context(|| format!("Element not found: {selector}"))?;
            let text = el.text().await.context("Failed to get text")?;
            if text.contains(expected) {
                Ok(serde_json::json!(true))
            } else {
                anyhow::bail!(
                    "Text assertion failed. Expected to contain: '{expected}', found: '{text}'"
                );
            }
        }
        Commands::AssertVisible { selector } => {
            let js = format!(
                "const el = document.querySelector('{}'); if (!el) return false; const style = window.getComputedStyle(el); return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';",
                selector.replace('\'', "\\'")
            );
            let result: Value = client
                .execute(&js, vec![])
                .await
                .context("Failed to check visibility")?;
            if result.as_bool() == Some(true) {
                Ok(serde_json::json!(true))
            } else {
                anyhow::bail!("Visibility assertion failed for: {selector}");
            }
        }
        Commands::AssertExists { selector } => {
            let exists = client.find(Locator::Css(selector)).await.is_ok();
            if exists {
                Ok(serde_json::json!(true))
            } else {
                anyhow::bail!("Existence assertion failed for: {selector}");
            }
        }

        // ============ Style ============
        Commands::Style { selector, property } => {
            let js = generate_computed_style_js(selector, property);
            let result: Value = client.execute(&js, vec![]).await?;
            if result.is_null() {
                Ok(serde_json::Value::Null)
            } else {
                Ok(serde_json::json!(result))
            }
        }
        Commands::Repl => {
            // Handled externally
            Ok(serde_json::Value::Null)
        }
        Commands::SemanticTree => {
            let js = generate_semantic_tree_js();
            let result: Value = client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(result))
        }
        Commands::ScreenshotAnnotated { path } => {
            let js = generate_screenshot_annotated_js();
            client
                .execute(&js, vec![])
                .await
                .context("Failed to inject annotations")?;
            tokio::time::sleep(Duration::from_millis(100)).await;
            let png = client.screenshot().await.context("Screenshot failed")?;
            std::fs::write(path, png).context("Failed to write screenshot")?;
            Ok(serde_json::json!(path))
        }
    }
}

// ============ Helper functions for WebDriver operations ============

/// Double-click using JavaScript fallback
async fn double_click(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); \
         if (el) {{ el.dispatchEvent(new MouseEvent('dblclick', {{ bubbles: true, cancelable: true, view: window }})); }}",
        selector.replace('\'', "\\'")
    );
    client
        .execute(&js, vec![])
        .await
        .map_err(anyhow::Error::from)
        .map(|_| ())
}

/// Right-click (context menu) using JavaScript fallback
async fn right_click(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); \
         if (el) {{ el.dispatchEvent(new MouseEvent('contextmenu', {{ bubbles: true, cancelable: true, view: window }})); }}",
        selector.replace('\'', "\\'")
    );
    client
        .execute(&js, vec![])
        .await
        .map_err(anyhow::Error::from)
        .map(|_| ())
}

/// Hover using JavaScript fallback
async fn hover(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); \
         if (el) {{ el.dispatchEvent(new MouseEvent('mouseover', {{ bubbles: true, cancelable: true, view: window }})); }}",
        selector.replace('\'', "\\'")
    );
    client
        .execute(&js, vec![])
        .await
        .map_err(anyhow::Error::from)
        .map(|_| ())
}

/// Scroll element into view using JavaScript
async fn scroll_into_view(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); if (el) {{ el.scrollIntoView({{ behavior: 'smooth', block: 'center' }}); }}",
        selector.replace('\'', "\\'")
    );
    client
        .execute(&js, vec![])
        .await
        .map_err(anyhow::Error::from)
        .map(|_| ())
}

/// Submit form using JavaScript
async fn submit_form(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); if (el) {{ el.dispatchEvent(new Event('submit', {{ bubbles: true, cancelable: true }})); }}",
        selector.replace('\'', "\\'")
    );
    client
        .execute(&js, vec![])
        .await
        .map_err(anyhow::Error::from)
        .map(|_| ())
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
    client
        .execute(&js, vec![])
        .await
        .map_err(anyhow::Error::from)
        .map(|_| ())
}

/// Check checkbox/radio using JavaScript
async fn check_element(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); if (el && !el.checked) {{ el.checked = true; el.dispatchEvent(new Event('change', {{ bubbles: true }})); }}",
        selector.replace('\'', "\\'")
    );
    client
        .execute(&js, vec![])
        .await
        .map_err(anyhow::Error::from)
        .map(|_| ())
}

/// Uncheck checkbox using JavaScript
async fn uncheck_element(client: &fantoccini::Client, selector: &str) -> Result<()> {
    let js = format!(
        "const el = document.querySelector('{}'); if (el && el.checked) {{ el.checked = false; el.dispatchEvent(new Event('change', {{ bubbles: true }})); }}",
        selector.replace('\'', "\\'")
    );
    client
        .execute(&js, vec![])
        .await
        .map_err(anyhow::Error::from)
        .map(|_| ())
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
