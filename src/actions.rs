#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Actions layer - Async `WebDriver` operations at the shell boundary
//! All I/O happens here

#![allow(dead_code)]

use crate::calculations::{
    generate_computed_style_js, generate_console_js, generate_css_injection_js,
    generate_dioxus_click_js, generate_dioxus_state_js, generate_element_screenshot_js,
    generate_extract_table_js, generate_fuzzy_click_js, generate_hydration_wait_js,
    generate_keycombo_js, generate_keypress_js, generate_network_idle_js,
    generate_screenshot_annotated_js, generate_scroll_to_text_js, generate_semantic_tree_js,
    generate_storage_js, generate_wait_element_js, generate_wait_gone_js,
};
use crate::data::{BrowserMode, Commands, Config, OutputFormat, WaitStrategy};
use anyhow::{Context, Result};
use fantoccini::{Client, ClientBuilder, Locator};
use serde_json::Value;
use std::time::Duration;

/// Executes a command.
///
/// # Errors
///
/// Returns an error if the webdriver fails to connect, navigate, or execute the command.
pub async fn execute_command(config: Config) -> Result<()> {
    let mut caps = serde_json::Map::new();
    let mut args = vec!["no-sandbox", "disable-dev-shm-usage", "disable-gpu"];
    if config.mode == BrowserMode::Headless {
        args.push("headless");
    }
    caps.insert(
        "goog:chromeOptions".to_string(),
        serde_json::json!({ "args": args }),
    );

    let mut client = ClientBuilder::native()
        .capabilities(caps)
        .connect(config.webdriver_url.as_str())
        .await
        .context("Failed to connect to ChromeDriver")?;

    client
        .goto(config.url.as_str())
        .await
        .with_context(|| format!("Failed to navigate to {}", config.url))?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    inject_console_capture(&mut client).await.context("Failed to inject console capture script")?;

    if config.wait == WaitStrategy::Auto {
        let js = generate_hydration_wait_js();
        let _ = client.execute(&js, vec![]).await;
    }

    let result = if matches!(config.command, Commands::Repl) {
        run_repl(&mut client).await?;
        Ok(serde_json::Value::Null)
    } else {
        match tokio::time::timeout(
            config.timeout,
            dispatch_command(&mut client, &config.command),
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

    let _ = client.close().await;

    if config.output == OutputFormat::Json {
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
        // It's safe to unwrap serde_json::to_string here because the struct is explicitly constructed
        // with known safe primitive types and standard Values.
        println!("{}", serde_json::to_string(&output).unwrap_or_else(|_| r#"{"success":false,"command":"unknown","data":null,"error":"Failed to serialize JSON output","logs":[]}"#.to_string()));
        Ok(())
    } else {
        match result {
            Ok(Value::String(s)) => {
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

async fn run_repl(client: &mut Client) -> Result<()> {
    let current_url = client
        .current_url()
        .await
        .map(|u| u.to_string())
        .unwrap_or_default();
    println!("Dioxus Agent REPL connected to {current_url}");
    println!("Type 'help' for commands, 'exit' to quit.");

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
                            match dispatch_command(client, &cmd).await {
                                Ok(res) => println!("Result: {res}"),
                                Err(e) => println!("Error: {e}"),
                            }
                        }
                        Err(e) => println!("{e}"),
                    }
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}

#[allow(clippy::needless_pass_by_ref_mut)]
async fn inject_console_capture(client: &mut Client) -> Result<()> {
    let js = r"
        window.__captured_logs = [];
        window.__captured_network = [];
        window.__mock_routes = window.__mock_routes || [];
        ['log', 'warn', 'error', 'info', 'debug'].forEach(type => {
            window['__captured_' + type] = [];
            const original = console[type];
            console[type] = function(...args) {
                window['__captured_' + type].push(args.map(a => String(a)));
                original.apply(console, args);
            };
        });
        const originalFetch = window.fetch;
        window.__active_requests = 0;
        window.fetch = async function(...args) {
            const url = typeof args[0] === 'string' ? args[0] : (args[0] && args[0].url) || 'unknown';
            for (const route of window.__mock_routes) {
                if (url.includes(route.pattern)) {
                    return new Response(route.response, {
                        status: route.status,
                        headers: { 'Content-Type': 'application/json' }
                    });
                }
            }
            window.__active_requests++;
            window.__captured_network.push({ type: 'fetch', url: url });
            try {
                const response = await originalFetch.apply(this, args);
                window.__active_requests--;
                return response;
            } catch (error) {
                window.__active_requests--;
                throw error;
            }
        };
        const originalXhrOpen = XMLHttpRequest.prototype.open;
        const originalXhrSend = XMLHttpRequest.prototype.send;
        XMLHttpRequest.prototype.open = function(method, url, ...rest) {
            this._url = url;
            this._method = method;
            return originalXhrOpen.apply(this, [method, url, ...rest]);
        };
        XMLHttpRequest.prototype.send = function(...args) {
            window.__active_requests++;
            window.__captured_network.push({ type: 'xhr', method: this._method, url: this._url });
            this.addEventListener('loadend', () => window.__active_requests--);
            this.addEventListener('error', () => window.__active_requests--);
            this.addEventListener('abort', () => window.__active_requests--);
            return originalXhrSend.apply(this, args);
        };
    ";
    client.execute(js, vec![]).await.map(|_| ())?;
    Ok(())
}

/// Command Dispatcher
async fn dispatch_command(client: &mut Client, command: &Commands) -> Result<Value> {
    match command {
        Commands::Dom | Commands::Title | Commands::Url | Commands::Refresh | Commands::Back | Commands::Forward => handle_navigation(client, command).await,
        Commands::Click { .. } | Commands::DoubleClick { .. } | Commands::RightClick { .. } | Commands::Hover { .. } | Commands::Text { .. } | Commands::Clear { .. } | Commands::Submit { .. } | Commands::Select { .. } | Commands::Check { .. } | Commands::Uncheck { .. } => handle_interaction(client, command).await,
        Commands::GetText { .. } | Commands::Attr { .. } | Commands::Classes { .. } | Commands::TagName { .. } | Commands::Visible { .. } | Commands::Enabled { .. } | Commands::Selected { .. } | Commands::Count { .. } | Commands::FindAll { .. } | Commands::Exists { .. } => handle_queries(client, command).await,
        Commands::Cookies | Commands::SetCookie { .. } | Commands::DeleteCookie { .. } | Commands::LocalGet { .. } | Commands::LocalSet { .. } | Commands::LocalRemove { .. } | Commands::LocalClear | Commands::SessionGet { .. } | Commands::SessionSet { .. } | Commands::SessionClear => handle_storage(client, command).await,
        Commands::Eval { .. } | Commands::InjectCss { .. } | Commands::Screenshot { .. } | Commands::ElementScreenshot { .. } | Commands::ScreenshotAnnotated { .. } => handle_eval_screenshot(client, command).await,
        Commands::Viewport { .. } | Commands::Scroll { .. } | Commands::ScrollBy { .. } | Commands::Key { .. } | Commands::KeyCombo { .. } => handle_viewport_keyboard(client, command).await,
        Commands::Console | Commands::ConsoleLog { .. } | Commands::Wait { .. } | Commands::WaitGone { .. } | Commands::WaitNav | Commands::WaitHydration => handle_console_wait(client, command).await,
        Commands::DioxusState | Commands::DioxusClick { .. } | Commands::SemanticTree | Commands::Style { .. } => handle_dioxus_style(client, command).await,
        Commands::Upload { .. } | Commands::FillForm { .. } | Commands::NetworkLogs | Commands::AssertText { .. } | Commands::AssertVisible { .. } | Commands::AssertExists { .. } | Commands::FuzzyClick { .. } | Commands::WaitNetworkIdle | Commands::ScrollToText { .. } | Commands::ExtractTable { .. } => handle_ai_extended(client, command).await,
        Commands::MockRoute { .. } | Commands::ShadowClick { .. } | Commands::DragAndDrop { .. } | Commands::ExportState { .. } | Commands::ImportState { .. } => handle_god_tier(client, command).await,
        Commands::Repl => Ok(Value::Null),
    }
}

#[allow(clippy::needless_pass_by_ref_mut)]
async fn handle_eval_screenshot(client: &mut Client, command: &Commands) -> Result<Value> {
    match command {
        Commands::Eval { js } => {
            let res = client.execute(js, vec![]).await?;
            Ok(serde_json::json!(res))
        }
        Commands::InjectCss { css } => {
            client.execute(&generate_css_injection_js(css), vec![]).await?;
            Ok(serde_json::json!("CSS injected"))
        }
        Commands::Screenshot { path } => {
            let png = client.screenshot().await?;
            std::fs::write(path, png)?;
            Ok(serde_json::json!(path))
        }
        Commands::ElementScreenshot { selector, path } => {
            let js = generate_element_screenshot_js(selector);
            let bounds: Value = client.execute(&js, vec![]).await?;
            if bounds.is_null() {
                anyhow::bail!("Element not found: {selector}");
            }
            let png = client.screenshot().await?;
            std::fs::write(path, png)?;
            Ok(serde_json::json!(path))
        }
        Commands::ScreenshotAnnotated { path } => {
            client.execute(&generate_screenshot_annotated_js(), vec![]).await?;
            tokio::time::sleep(Duration::from_millis(100)).await;
            let png = client.screenshot().await?;
            std::fs::write(path, png)?;
            Ok(serde_json::json!(path))
        }
        _ => anyhow::bail!("Invalid eval/screenshot command"),
    }
}

#[allow(clippy::needless_pass_by_ref_mut)]
async fn handle_viewport_keyboard(client: &mut Client, command: &Commands) -> Result<Value> {
    match command {
        Commands::Viewport { width, height } => {
            client.set_window_size(*width, *height).await?;
            Ok(serde_json::json!(format!("{width} {height}")))
        }
        Commands::Scroll { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (el) {{ el.scrollIntoView({{ behavior: 'smooth', block: 'center' }}); }}", crate::calculations::escape_js_string(selector));
            client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::ScrollBy { x, y } => {
            client.execute(&format!("window.scrollBy({x}, {y}); return true;"), vec![]).await?;
            Ok(serde_json::json!(format!("{x} {y}")))
        }
        Commands::Key { key } => {
            client.execute(&generate_keypress_js(key), vec![]).await?;
            Ok(serde_json::json!(key))
        }
        Commands::KeyCombo { combo } => {
            client.execute(&generate_keycombo_js(combo), vec![]).await?;
            Ok(serde_json::json!(combo))
        }
        _ => anyhow::bail!("Invalid viewport/keyboard command"),
    }
}

#[allow(clippy::needless_pass_by_ref_mut)]
async fn handle_console_wait(client: &mut Client, command: &Commands) -> Result<Value> {
    match command {
        Commands::Console => {
            let res = client.execute(&generate_console_js(None), vec![]).await?;
            Ok(serde_json::json!(res))
        }
        Commands::ConsoleLog { r#type } => {
            let res = client.execute(&generate_console_js(Some(r#type)), vec![]).await?;
            Ok(serde_json::json!(res))
        }
        Commands::Wait { selector } => {
            client.execute(&generate_wait_element_js(selector), vec![]).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::WaitGone { selector } => {
            client.execute(&generate_wait_gone_js(selector), vec![]).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::WaitNav => {
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok(serde_json::json!("navigation complete"))
        }
        Commands::WaitHydration => {
            client.execute(&generate_hydration_wait_js(), vec![]).await?;
            Ok(serde_json::json!("hydrated"))
        }
        _ => anyhow::bail!("Invalid console/wait command"),
    }
}

#[allow(clippy::needless_pass_by_ref_mut)]
async fn handle_dioxus_style(client: &mut Client, command: &Commands) -> Result<Value> {
    match command {
        Commands::DioxusState => {
            let res = client.execute(&generate_dioxus_state_js(), vec![]).await?;
            Ok(serde_json::json!(res))
        }
        Commands::DioxusClick { target } => {
            let res = client.execute(&generate_dioxus_click_js(target), vec![]).await?;
            if res.as_bool() == Some(true) {
                Ok(serde_json::json!(target))
            } else {
                anyhow::bail!("Target not found: {target}");
            }
        }
        Commands::SemanticTree => {
            let res = client.execute(&generate_semantic_tree_js(), vec![]).await?;
            Ok(serde_json::json!(res))
        }
        Commands::Style { selector, property } => {
            let res = client.execute(&generate_computed_style_js(selector, property), vec![]).await?;
            Ok(if res.is_null() { Value::Null } else { serde_json::json!(res) })
        }
        _ => anyhow::bail!("Invalid dioxus/style command"),
    }
}

#[allow(clippy::needless_pass_by_ref_mut)]
async fn handle_ai_extended(client: &mut Client, command: &Commands) -> Result<Value> {
    match command {
        Commands::Upload { selector, path } => {
            let el = client.find(Locator::Css(selector)).await?;
            let abs_path = std::fs::canonicalize(path)?;
            el.send_keys(abs_path.to_string_lossy().as_ref()).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::FillForm { json_data } => {
            let map: serde_json::Map<String, Value> = serde_json::from_str(json_data)?;
            let mut results = Vec::new();
            for (selector, val) in map {
                if let Some(text_val) = val.as_str() {
                    let el = client.find(Locator::Css(&selector)).await?;
                    el.clear().await?;
                    el.send_keys(text_val).await?;
                    results.push(selector);
                }
            }
            Ok(serde_json::json!(results))
        }
        Commands::NetworkLogs => {
            let res = client.execute("return window.__captured_network || [];", vec![]).await?;
            Ok(serde_json::json!(res))
        }
        Commands::AssertText { selector, expected } => {
            let el = client.find(Locator::Css(selector)).await?;
            let text = el.text().await?;
            if text.contains(expected) {
                Ok(serde_json::json!(true))
            } else {
                anyhow::bail!("Text assertion failed. Expected: '{expected}', found: '{text}'");
            }
        }
        Commands::AssertVisible { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (!el) return false; const style = window.getComputedStyle(el); return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';", crate::calculations::escape_js_string(selector));
            if client.execute(&js, vec![]).await?.as_bool() == Some(true) {
                Ok(serde_json::json!(true))
            } else {
                anyhow::bail!("Visibility assertion failed for: {selector}");
            }
        }
        Commands::AssertExists { selector } => {
            if client.find(Locator::Css(selector)).await.is_ok() {
                Ok(serde_json::json!(true))
            } else {
                anyhow::bail!("Existence assertion failed for: {selector}");
            }
        }
        Commands::FuzzyClick { text } => {
            let res = client.execute(&generate_fuzzy_click_js(text), vec![]).await?;
            if res.is_null() { anyhow::bail!("FuzzyClick failed to find: {text}"); }
            Ok(serde_json::json!(res))
        }
        Commands::WaitNetworkIdle => {
            if client.execute(&generate_network_idle_js(), vec![]).await?.as_bool() == Some(true) {
                Ok(serde_json::json!("Network idle"))
            } else {
                anyhow::bail!("Timeout waiting for network to become idle");
            }
        }
        Commands::ScrollToText { container, text } => {
            let res = client.execute(&generate_scroll_to_text_js(container, text), vec![]).await?;
            if res.as_bool() == Some(true) {
                Ok(serde_json::json!(text))
            } else {
                anyhow::bail!("ScrollToText failed to find: {text}");
            }
        }
        Commands::ExtractTable { selector } => {
            let res = client.execute(&generate_extract_table_js(selector), vec![]).await?;
            if res.is_null() { anyhow::bail!("Table not found: {selector}"); }
            Ok(res)
        }
        _ => anyhow::bail!("Invalid AI extended command"),
    }
}

#[allow(clippy::needless_pass_by_ref_mut)]
async fn handle_god_tier(client: &mut Client, command: &Commands) -> Result<Value> {
    match command {
        Commands::MockRoute { url_pattern, response_json, status } => {
            let js = format!(
                "window.__mock_routes = window.__mock_routes || []; window.__mock_routes.push({{ pattern: '{}', response: '{}', status: {} }}); return true;",
                crate::calculations::escape_js_string(url_pattern),
                crate::calculations::escape_js_string(response_json),
                status
            );
            client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(url_pattern))
        }
        Commands::ShadowClick { selector } => {
            let parts: Vec<&str> = selector.split(">>").map(str::trim).collect();
            let js = format!("
                const selectors = {};
                let current = document;
                for (let i = 0; i < selectors.length; i++) {{
                    if (current.shadowRoot) current = current.shadowRoot;
                    current = current.querySelector(selectors[i]);
                    if (!current) return false;
                }}
                current.click();
                return true;
            ", serde_json::to_string(&parts)?);
            if client.execute(&js, vec![]).await?.as_bool() == Some(true) {
                Ok(serde_json::json!(selector))
            } else {
                anyhow::bail!("Shadow element not found: {selector}");
            }
        }
        Commands::DragAndDrop { source, target } => {
            let js = format!("
                const source = document.querySelector('{}');
                const target = document.querySelector('{}');
                if (!source || !target) return false;
                const dataTransfer = new DataTransfer();
                source.dispatchEvent(new DragEvent('dragstart', {{ dataTransfer, bubbles: true }}));
                target.dispatchEvent(new DragEvent('dragenter', {{ dataTransfer, bubbles: true }}));
                target.dispatchEvent(new DragEvent('dragover',  {{ dataTransfer, bubbles: true }}));
                target.dispatchEvent(new DragEvent('drop',      {{ dataTransfer, bubbles: true }}));
                source.dispatchEvent(new DragEvent('dragend',   {{ dataTransfer, bubbles: true }}));
                return true;
            ", crate::calculations::escape_js_string(source), crate::calculations::escape_js_string(target));
            if client.execute(&js, vec![]).await?.as_bool() == Some(true) {
                Ok(serde_json::json!(true))
            } else {
                anyhow::bail!("DragAndDrop failed");
            }
        }
        Commands::ExportState { path } => {
            let storage = client.execute("return { localStorage: Object.assign({}, window.localStorage), sessionStorage: Object.assign({}, window.sessionStorage) };", vec![]).await?;
            let cookies = client.get_all_cookies().await?;
            let state = serde_json::json!({ "storage": storage, "cookies": cookies.iter().map(|c| serde_json::json!({ "name": c.name(), "value": c.value(), "domain": c.domain(), "path": c.path() })).collect::<Vec<_>>() });
            std::fs::write(path, serde_json::to_string_pretty(&state)?)?;
            Ok(serde_json::json!(path))
        }
        Commands::ImportState { path } => {
            let content = std::fs::read_to_string(path)?;
            let state: Value = serde_json::from_str(&content)?;
            if let Some(storage) = state.get("storage")
                && let Some(ls) = storage.get("localStorage").and_then(Value::as_object) {
                    for (k, v) in ls {
                        if let Some(v_str) = v.as_str() {
                            client.execute(&format!("localStorage.setItem('{}', '{}');", crate::calculations::escape_js_string(k), crate::calculations::escape_js_string(v_str)), vec![]).await?;
                        }
                    }
                }
            Ok(serde_json::json!(path))
        }
        _ => anyhow::bail!("Invalid god tier command"),
    }
}


#[allow(clippy::needless_pass_by_ref_mut)]
async fn handle_navigation(client: &mut Client, command: &Commands) -> Result<Value> {
    match command {
        Commands::Dom => Ok(serde_json::json!(client.source().await?)),
        Commands::Title => Ok(serde_json::json!(client.title().await?)),
        Commands::Url => Ok(serde_json::json!(client.current_url().await?.to_string())),
        Commands::Refresh => { client.refresh().await?; Ok(serde_json::json!("Page refreshed")) }
        Commands::Back => { client.back().await?; Ok(serde_json::json!("Navigated back")) }
        Commands::Forward => { client.forward().await?; Ok(serde_json::json!("Navigated forward")) }
        _ => anyhow::bail!("Invalid navigation command"),
    }
}

#[allow(clippy::needless_pass_by_ref_mut)]
async fn handle_interaction(client: &mut Client, command: &Commands) -> Result<Value> {
    match command {
        Commands::Click { selector } => { client.find(Locator::Css(selector)).await?.click().await?; Ok(serde_json::json!(selector)) }
        Commands::Text { selector, value } => { client.find(Locator::Css(selector)).await?.send_keys(value).await?; Ok(serde_json::json!(selector)) }
        Commands::Clear { selector } => { client.find(Locator::Css(selector)).await?.clear().await?; Ok(serde_json::json!(selector)) }
        Commands::Submit { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (el) {{ el.dispatchEvent(new Event('submit', {{ bubbles: true, cancelable: true }})); }}", crate::calculations::escape_js_string(selector));
            client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::Select { selector, value } => {
            let js = format!("const sel = document.querySelector('{}'); if (sel) {{ for (let opt of sel.options) {{ if (opt.value === '{}') {{ opt.selected = true; break; }} }} sel.dispatchEvent(new Event('change', {{ bubbles: true }})); }}", crate::calculations::escape_js_string(selector), crate::calculations::escape_js_string(value));
            client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::Check { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (el && !el.checked) {{ el.checked = true; el.dispatchEvent(new Event('change', {{ bubbles: true }})); }}", crate::calculations::escape_js_string(selector));
            client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::Uncheck { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (el && el.checked) {{ el.checked = false; el.dispatchEvent(new Event('change', {{ bubbles: true }})); }}", crate::calculations::escape_js_string(selector));
            client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::DoubleClick { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (el) el.dispatchEvent(new MouseEvent('dblclick', {{ bubbles: true }}));", crate::calculations::escape_js_string(selector));
            client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::RightClick { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (el) el.dispatchEvent(new MouseEvent('contextmenu', {{ bubbles: true }}));", crate::calculations::escape_js_string(selector));
            client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::Hover { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (el) el.dispatchEvent(new MouseEvent('mouseover', {{ bubbles: true }}));", crate::calculations::escape_js_string(selector));
            client.execute(&js, vec![]).await?;
            Ok(serde_json::json!(selector))
        }
        _ => anyhow::bail!("Invalid interaction command"),
    }
}

#[allow(clippy::needless_pass_by_ref_mut)]
async fn handle_queries(client: &mut Client, command: &Commands) -> Result<Value> {
    match command {
        Commands::GetText { selector } => Ok(serde_json::json!(client.find(Locator::Css(selector)).await?.text().await?)),
        Commands::Attr { selector, attribute } => {
            let res = client.find(Locator::Css(selector)).await?.attr(attribute).await?;
            Ok(res.map_or(Value::Null, |v| serde_json::json!(v)))
        }
        Commands::Classes { selector } => {
            let res = client.find(Locator::Css(selector)).await?.attr("class").await?;
            Ok(res.map_or(Value::Null, |c| serde_json::json!(c.split_whitespace().collect::<Vec<_>>().join(" "))))
        }
        Commands::TagName { selector } => {
            let js = format!("const el = document.querySelector('{}'); return el ? el.tagName.toLowerCase() : null;", crate::calculations::escape_js_string(selector));
            let res = client.execute(&js, vec![]).await?;
            Ok(if res.is_null() { Value::Null } else { serde_json::json!(res) })
        }
        Commands::Visible { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (!el) return false; const style = window.getComputedStyle(el); return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';", crate::calculations::escape_js_string(selector));
            Ok(serde_json::json!(client.execute(&js, vec![]).await?.as_bool().unwrap_or(false)))
        }
        Commands::Enabled { selector } => {
            let js = format!("const el = document.querySelector('{}'); return el ? !el.disabled : false;", crate::calculations::escape_js_string(selector));
            Ok(serde_json::json!(client.execute(&js, vec![]).await?.as_bool().unwrap_or(false)))
        }
        Commands::Selected { selector } => {
            let js = format!("const el = document.querySelector('{}'); return el ? (el.checked || el.selected) : false;", crate::calculations::escape_js_string(selector));
            Ok(serde_json::json!(client.execute(&js, vec![]).await?.as_bool().unwrap_or(false)))
        }
        Commands::Count { selector } => Ok(serde_json::json!(client.find_all(Locator::Css(selector)).await?.len())),
        Commands::FindAll { selector } => {
            let els = client.find_all(Locator::Css(selector)).await?;
            let mut htmls = Vec::new();
            for el in els { htmls.push(el.html(true).await.unwrap_or_default()); }
            Ok(serde_json::json!(htmls))
        }
        Commands::Exists { selector } => Ok(serde_json::json!(client.find(Locator::Css(selector)).await.is_ok())),
        _ => anyhow::bail!("Invalid query command"),
    }
}

#[allow(clippy::needless_pass_by_ref_mut)]
async fn handle_storage(client: &mut Client, command: &Commands) -> Result<Value> {
    match command {
        Commands::Cookies => {
            let cookies = client.get_all_cookies().await?;
            let res: Vec<String> = cookies.into_iter().map(|c| format!("{}={}; Path={}; Domain={}", c.name(), c.value(), c.path().unwrap_or("/"), c.domain().unwrap_or(""))).collect();
            Ok(serde_json::json!(res))
        }
        Commands::SetCookie { name, value, domain, path } => {
            let cookie_str = format!("{}={}; domain={}; path={}", crate::calculations::escape_js_string(name), crate::calculations::escape_js_string(value), domain.as_deref().unwrap_or(""), path.as_deref().unwrap_or(""));
            client.execute(&format!("document.cookie = '{cookie_str}'; return true;"), vec![]).await?;
            Ok(serde_json::json!(name))
        }
        Commands::DeleteCookie { name } => { client.delete_cookie(name).await?; Ok(serde_json::json!(name)) }
        Commands::LocalGet { key } => Ok(client.execute(&generate_storage_js("local", "get", Some(key), None), vec![]).await?),
        Commands::LocalSet { key, value } => { client.execute(&generate_storage_js("local", "set", Some(key), Some(value)), vec![]).await?; Ok(serde_json::json!(key)) }
        Commands::LocalRemove { key } => { client.execute(&generate_storage_js("local", "remove", Some(key), None), vec![]).await?; Ok(serde_json::json!(key)) }
        Commands::LocalClear => { client.execute(&generate_storage_js("local", "clear", None, None), vec![]).await?; Ok(serde_json::json!("cleared")) }
        Commands::SessionGet { key } => Ok(client.execute(&generate_storage_js("session", "get", Some(key), None), vec![]).await?),
        Commands::SessionSet { key, value } => { client.execute(&generate_storage_js("session", "set", Some(key), Some(value)), vec![]).await?; Ok(serde_json::json!(key)) }
        Commands::SessionClear => { client.execute(&generate_storage_js("session", "clear", None, None), vec![]).await?; Ok(serde_json::json!("cleared")) }
        _ => anyhow::bail!("Invalid storage command"),
    }
}
