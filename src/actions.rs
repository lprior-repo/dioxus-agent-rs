#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Actions layer - Async `WebDriver` operations via CDP (`chromiumoxide`)
//! All I/O happens here

#![allow(dead_code)]
#![allow(clippy::needless_pass_by_ref_mut)]

use crate::calculations::{
    generate_computed_style_js, generate_console_js, generate_css_injection_js,
    generate_dioxus_click_js, generate_dioxus_state_js,
    generate_extract_table_js, generate_fuzzy_click_js, generate_hydration_wait_js,
    generate_keycombo_js, generate_keypress_js, generate_network_idle_js,
    generate_screenshot_annotated_js, generate_scroll_to_text_js, generate_semantic_tree_js,
    generate_storage_js, generate_wait_element_js, generate_wait_gone_js,
};
use crate::data::{BrowserMode, Commands, Config, OutputFormat, WaitStrategy};
use anyhow::{Context, Result};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::page::Page;
use futures::StreamExt;
use serde_json::Value;
use std::time::Duration;

/// Executes a command.
///
/// # Errors
///
/// Returns an error if the browser fails to launch, navigate, or if the command execution fails.
pub async fn execute_command(config: Config) -> Result<()> {
    let mut builder = BrowserConfig::builder();
    if config.mode == BrowserMode::Headed {
        builder = builder.with_head();
    }
    
    // Launch browser
    let (mut browser, mut handler) = Browser::launch(
        builder.build().map_err(|e| anyhow::anyhow!(e))?
    ).await.context("Failed to launch Chrome")?;
    
    let _handle = tokio::task::spawn(async move {
        while let Some(h) = handler.next().await {
            if h.is_err() {
                break;
            }
        }
    });

    let page = browser.new_page(config.url.as_str()).await.context("Failed to navigate")?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    inject_console_capture(&page).await.context("Failed to inject console capture script")?;

    if config.wait == WaitStrategy::Auto {
        let js = generate_hydration_wait_js();
        let _ = page.evaluate(js).await;
    }

    let result = if matches!(config.command, Commands::Repl) {
        run_repl(&page).await?;
        Ok(serde_json::Value::Null)
    } else {
        match tokio::time::timeout(
            config.timeout,
            dispatch_command(&page, &config.command),
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

    let _ = browser.close().await;

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

async fn run_repl(page: &Page) -> Result<()> {
    let current_url = page.evaluate("window.location.href").await?.into_value::<String>()?;
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
                            match dispatch_command(page, &cmd).await {
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

async fn inject_console_capture(page: &Page) -> Result<()> {
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
    page.evaluate(js).await?;
    Ok(())
}

/// Command Dispatcher
async fn dispatch_command(page: &Page, command: &Commands) -> Result<Value> {
    match command {
        Commands::Dom | Commands::Title | Commands::Url | Commands::Refresh | Commands::Back | Commands::Forward => handle_navigation(page, command).await,
        Commands::Click { .. } | Commands::DoubleClick { .. } | Commands::RightClick { .. } | Commands::Hover { .. } | Commands::Text { .. } | Commands::Clear { .. } | Commands::Submit { .. } | Commands::Select { .. } | Commands::Check { .. } | Commands::Uncheck { .. } => handle_interaction(page, command).await,
        Commands::GetText { .. } | Commands::Attr { .. } | Commands::Classes { .. } | Commands::TagName { .. } | Commands::Visible { .. } | Commands::Enabled { .. } | Commands::Selected { .. } | Commands::Count { .. } | Commands::FindAll { .. } | Commands::Exists { .. } => handle_queries(page, command).await,
        Commands::Cookies | Commands::SetCookie { .. } | Commands::DeleteCookie { .. } | Commands::LocalGet { .. } | Commands::LocalSet { .. } | Commands::LocalRemove { .. } | Commands::LocalClear | Commands::SessionGet { .. } | Commands::SessionSet { .. } | Commands::SessionClear => handle_storage(page, command).await,
        Commands::Eval { .. } | Commands::InjectCss { .. } | Commands::Screenshot { .. } | Commands::ElementScreenshot { .. } | Commands::ScreenshotAnnotated { .. } => handle_eval_screenshot(page, command).await,
        Commands::Viewport { .. } | Commands::Scroll { .. } | Commands::ScrollBy { .. } | Commands::Key { .. } | Commands::KeyCombo { .. } => handle_viewport_keyboard(page, command).await,
        Commands::Console | Commands::ConsoleLog { .. } | Commands::Wait { .. } | Commands::WaitGone { .. } | Commands::WaitNav | Commands::WaitHydration => handle_console_wait(page, command).await,
        Commands::DioxusState | Commands::DioxusClick { .. } | Commands::SemanticTree | Commands::Style { .. } => handle_dioxus_style(page, command).await,
        Commands::Upload { .. } | Commands::FillForm { .. } | Commands::NetworkLogs | Commands::AssertText { .. } | Commands::AssertVisible { .. } | Commands::AssertExists { .. } | Commands::FuzzyClick { .. } | Commands::WaitNetworkIdle | Commands::ScrollToText { .. } | Commands::ExtractTable { .. } => handle_ai_extended(page, command).await,
        Commands::MockRoute { .. } | Commands::ShadowClick { .. } | Commands::DragAndDrop { .. } | Commands::ExportState { .. } | Commands::ImportState { .. } => handle_god_tier(page, command).await,
        Commands::Repl => Ok(Value::Null),
    }
}

async fn handle_eval_screenshot(page: &Page, command: &Commands) -> Result<Value> {
    match command {
        Commands::Eval { js } => {
            let res = page.evaluate(js.as_str()).await?.into_value::<Value>()?;
            Ok(res)
        }
        Commands::InjectCss { css } => {
            page.evaluate(generate_css_injection_js(css).as_str()).await?;
            Ok(serde_json::json!("CSS injected"))
        }
        Commands::Screenshot { path } => {
            let params = chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotParams::builder().format(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png).build();
            let png = page.screenshot(params).await?;
            std::fs::write(path, png)?;
            Ok(serde_json::json!(path))
        }
        Commands::ElementScreenshot { selector, path } => {
            let el = page.find_element(selector.as_str()).await?;
            let buf = el.screenshot(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png).await?;
            std::fs::write(path, buf)?;
            Ok(serde_json::json!(path))
        }
        Commands::ScreenshotAnnotated { path } => {
            page.evaluate(generate_screenshot_annotated_js().as_str()).await?;
            tokio::time::sleep(Duration::from_millis(100)).await;
            let params = chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotParams::builder().format(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png).build();
            let png = page.screenshot(params).await?;
            std::fs::write(path.clone(), png)?;
            Ok(serde_json::json!(path))
        }
        _ => anyhow::bail!("Invalid eval/screenshot command"),
    }
}

async fn handle_viewport_keyboard(page: &Page, command: &Commands) -> Result<Value> {
    match command {
        Commands::Viewport { width, height } => {
            let params = chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams::builder()
                .width(i64::from(*width))
                .height(i64::from(*height))
                .device_scale_factor(1.0)
                .mobile(false)
                .build()
                .map_err(|e| anyhow::anyhow!(e))?;
            page.execute(params).await?;
            Ok(serde_json::json!(format!("{width} {height}")))
        }
        Commands::Scroll { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (el) {{ el.scrollIntoView({{ behavior: 'smooth', block: 'center' }}); }}", crate::calculations::escape_js_string(selector));
            page.evaluate(js.as_str()).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::ScrollBy { x, y } => {
            page.evaluate(format!("window.scrollBy({x}, {y}); return true;").as_str()).await?;
            Ok(serde_json::json!(format!("{x} {y}")))
        }
        Commands::Key { key } => {
            page.evaluate(generate_keypress_js(key).as_str()).await?;
            Ok(serde_json::json!(key))
        }
        Commands::KeyCombo { combo } => {
            page.evaluate(generate_keycombo_js(combo).as_str()).await?;
            Ok(serde_json::json!(combo))
        }
        _ => anyhow::bail!("Invalid viewport/keyboard command"),
    }
}

async fn handle_console_wait(page: &Page, command: &Commands) -> Result<Value> {
    match command {
        Commands::Console => {
            let res = page.evaluate(generate_console_js(None).as_str()).await?.into_value::<Value>()?;
            Ok(res)
        }
        Commands::ConsoleLog { r#type } => {
            let res = page.evaluate(generate_console_js(Some(r#type)).as_str()).await?.into_value::<Value>()?;
            Ok(res)
        }
        Commands::Wait { selector } => {
            page.evaluate(generate_wait_element_js(selector).as_str()).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::WaitGone { selector } => {
            page.evaluate(generate_wait_gone_js(selector).as_str()).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::WaitNav => {
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok(serde_json::json!("navigation complete"))
        }
        Commands::WaitHydration => {
            page.evaluate(generate_hydration_wait_js().as_str()).await?;
            Ok(serde_json::json!("hydrated"))
        }
        _ => anyhow::bail!("Invalid console/wait command"),
    }
}

async fn handle_dioxus_style(page: &Page, command: &Commands) -> Result<Value> {
    match command {
        Commands::DioxusState => {
            let res = page.evaluate(generate_dioxus_state_js().as_str()).await?.into_value::<Value>()?;
            Ok(res)
        }
        Commands::DioxusClick { target } => {
            let res = page.evaluate(generate_dioxus_click_js(target).as_str()).await?.into_value::<Value>()?;
            if res.as_bool() == Some(true) {
                Ok(serde_json::json!(target))
            } else {
                anyhow::bail!("Target not found: {target}");
            }
        }
        Commands::SemanticTree => {
            let res = page.evaluate(generate_semantic_tree_js().as_str()).await?.into_value::<Value>()?;
            Ok(res)
        }
        Commands::Style { selector, property } => {
            let res = page.evaluate(generate_computed_style_js(selector, property).as_str()).await?.into_value::<Value>()?;
            Ok(if res.is_null() { Value::Null } else { serde_json::json!(res) })
        }
        _ => anyhow::bail!("Invalid dioxus/style command"),
    }
}

async fn handle_ai_extended(page: &Page, command: &Commands) -> Result<Value> {
    match command {
        Commands::Upload { selector, path } => {
            let el = page.find_element(selector.as_str()).await?;
            let abs_path = std::fs::canonicalize(path)?;
            let params = chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams::builder()
                .files(vec![abs_path.to_string_lossy().to_string()])
                .node_id(el.node_id)
                .build()
                .map_err(|e| anyhow::anyhow!(e))?;
            page.execute(params).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::FillForm { json_data } => {
            let map: serde_json::Map<String, Value> = serde_json::from_str(json_data)?;
            let mut results = Vec::new();
            for (selector, val) in map {
                if let Some(text_val) = val.as_str() {
                    let el = page.find_element(selector.as_str()).await?;
                    page.evaluate(format!("document.querySelector('{}').value = '';", crate::calculations::escape_js_string(&selector)).as_str()).await?;
                    el.type_str(text_val).await?;
                    results.push(selector);
                }
            }
            Ok(serde_json::json!(results))
        }
        Commands::NetworkLogs => {
            let res = page.evaluate("return window.__captured_network || [];").await?.into_value::<Value>()?;
            Ok(res)
        }
        Commands::AssertText { selector, expected } => {
            let el = page.find_element(selector.as_str()).await?;
            let text = el.inner_text().await?.unwrap_or_default();
            if text.contains(expected) {
                Ok(serde_json::json!(true))
            } else {
                anyhow::bail!("Text assertion failed. Expected: '{expected}', found: '{text}'");
            }
        }
        Commands::AssertVisible { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (!el) return false; const style = window.getComputedStyle(el); return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';", crate::calculations::escape_js_string(selector));
            if page.evaluate(js.as_str()).await?.into_value::<Value>()?.as_bool() == Some(true) {
                Ok(serde_json::json!(true))
            } else {
                anyhow::bail!("Visibility assertion failed for: {selector}");
            }
        }
        Commands::AssertExists { selector } => {
            if page.find_element(selector.as_str()).await.is_ok() {
                Ok(serde_json::json!(true))
            } else {
                anyhow::bail!("Existence assertion failed for: {selector}");
            }
        }
        Commands::FuzzyClick { text } => {
            let res = page.evaluate(generate_fuzzy_click_js(text).as_str()).await?.into_value::<Value>()?;
            if res.is_null() { anyhow::bail!("FuzzyClick failed to find: {text}"); }
            Ok(res)
        }
        Commands::WaitNetworkIdle => {
            if page.evaluate(generate_network_idle_js().as_str()).await?.into_value::<Value>()?.as_bool() == Some(true) {
                Ok(serde_json::json!("Network idle"))
            } else {
                anyhow::bail!("Timeout waiting for network to become idle");
            }
        }
        Commands::ScrollToText { container, text } => {
            let res = page.evaluate(generate_scroll_to_text_js(container, text).as_str()).await?.into_value::<Value>()?;
            if res.as_bool() == Some(true) {
                Ok(serde_json::json!(text))
            } else {
                anyhow::bail!("ScrollToText failed to find: {text}");
            }
        }
        Commands::ExtractTable { selector } => {
            let res = page.evaluate(generate_extract_table_js(selector).as_str()).await?.into_value::<Value>()?;
            if res.is_null() { anyhow::bail!("Table not found: {selector}"); }
            Ok(res)
        }
        _ => anyhow::bail!("Invalid AI extended command"),
    }
}

async fn handle_god_tier(page: &Page, command: &Commands) -> Result<Value> {
    match command {
        Commands::MockRoute { url_pattern, response_json, status } => {
            let js = format!(
                "window.__mock_routes = window.__mock_routes || []; window.__mock_routes.push({{ pattern: '{}', response: '{}', status: {} }}); return true;",
                crate::calculations::escape_js_string(url_pattern),
                crate::calculations::escape_js_string(response_json),
                status
            );
            page.evaluate(js.as_str()).await?;
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
            if page.evaluate(js.as_str()).await?.into_value::<Value>()?.as_bool() == Some(true) {
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
            if page.evaluate(js.as_str()).await?.into_value::<Value>()?.as_bool() == Some(true) {
                Ok(serde_json::json!(true))
            } else {
                anyhow::bail!("DragAndDrop failed");
            }
        }
        Commands::ExportState { path } => {
            let storage = page.evaluate("return { localStorage: Object.assign({}, window.localStorage), sessionStorage: Object.assign({}, window.sessionStorage) };").await?.into_value::<Value>()?;
            
            // For cookies, chromiumoxide uses Network domain
            let cookies = page.get_cookies().await?;
            
            // Map chromiumoxide's Cookie to JSON easily
            let state = serde_json::json!({ "storage": storage, "cookies": cookies });
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
                            page.evaluate(format!("localStorage.setItem('{}', '{}');", crate::calculations::escape_js_string(k), crate::calculations::escape_js_string(v_str)).as_str()).await?;
                        }
                    }
                }
            Ok(serde_json::json!(path))
        }
        _ => anyhow::bail!("Invalid god tier command"),
    }
}

async fn handle_navigation(page: &Page, command: &Commands) -> Result<Value> {
    match command {
        Commands::Dom => {
            let html = page.evaluate("document.documentElement.outerHTML").await?.into_value::<String>()?;
            Ok(serde_json::json!(html))
        }
        Commands::Title => {
            let title = page.evaluate("document.title").await?.into_value::<String>()?;
            Ok(serde_json::json!(title))
        }
        Commands::Url => {
            let url = page.evaluate("window.location.href").await?.into_value::<String>()?;
            Ok(serde_json::json!(url))
        }
        Commands::Refresh => { page.evaluate("location.reload()").await?; Ok(serde_json::json!("Page refreshed")) }
        Commands::Back => { page.evaluate("history.back()").await?; Ok(serde_json::json!("Navigated back")) }
        Commands::Forward => { page.evaluate("history.forward()").await?; Ok(serde_json::json!("Navigated forward")) }
        _ => anyhow::bail!("Invalid navigation command"),
    }
}

async fn handle_interaction(page: &Page, command: &Commands) -> Result<Value> {
    match command {
        Commands::Click { selector } => { page.find_element(selector.as_str()).await?.click().await?; Ok(serde_json::json!(selector)) }
        Commands::Text { selector, value } => { page.find_element(selector.as_str()).await?.type_str(value).await?; Ok(serde_json::json!(selector)) }
        Commands::Clear { selector } => { 
            page.evaluate(format!("document.querySelector('{}').value = '';", crate::calculations::escape_js_string(selector)).as_str()).await?; 
            Ok(serde_json::json!(selector)) 
        }
        Commands::Submit { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (el) {{ el.dispatchEvent(new Event('submit', {{ bubbles: true, cancelable: true }})); }}", crate::calculations::escape_js_string(selector));
            page.evaluate(js.as_str()).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::Select { selector, value } => {
            let js = format!("const sel = document.querySelector('{}'); if (sel) {{ for (let opt of sel.options) {{ if (opt.value === '{}') {{ opt.selected = true; break; }} }} sel.dispatchEvent(new Event('change', {{ bubbles: true }})); }}", crate::calculations::escape_js_string(selector), crate::calculations::escape_js_string(value));
            page.evaluate(js.as_str()).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::Check { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (el && !el.checked) {{ el.checked = true; el.dispatchEvent(new Event('change', {{ bubbles: true }})); }}", crate::calculations::escape_js_string(selector));
            page.evaluate(js.as_str()).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::Uncheck { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (el && el.checked) {{ el.checked = false; el.dispatchEvent(new Event('change', {{ bubbles: true }})); }}", crate::calculations::escape_js_string(selector));
            page.evaluate(js.as_str()).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::DoubleClick { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (el) el.dispatchEvent(new MouseEvent('dblclick', {{ bubbles: true }}));", crate::calculations::escape_js_string(selector));
            page.evaluate(js.as_str()).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::RightClick { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (el) el.dispatchEvent(new MouseEvent('contextmenu', {{ bubbles: true }}));", crate::calculations::escape_js_string(selector));
            page.evaluate(js.as_str()).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::Hover { selector } => {
            page.find_element(selector.as_str()).await?.hover().await?;
            Ok(serde_json::json!(selector))
        }
        _ => anyhow::bail!("Invalid interaction command"),
    }
}

async fn handle_queries(page: &Page, command: &Commands) -> Result<Value> {
    match command {
        Commands::GetText { selector } => Ok(serde_json::json!(page.find_element(selector.as_str()).await?.inner_text().await?.unwrap_or_default())),
        Commands::Attr { selector, attribute } => {
            let res = page.find_element(selector.as_str()).await?.attribute(attribute).await?;
            Ok(res.map_or(Value::Null, |v| serde_json::json!(v)))
        }
        Commands::Classes { selector } => {
            let res = page.find_element(selector.as_str()).await?.attribute("class").await?;
            Ok(res.map_or(Value::Null, |c| serde_json::json!(c.split_whitespace().collect::<Vec<_>>().join(" "))))
        }
        Commands::TagName { selector } => {
            let js = format!("const el = document.querySelector('{}'); return el ? el.tagName.toLowerCase() : null;", crate::calculations::escape_js_string(selector));
            let res = page.evaluate(js.as_str()).await?.into_value::<Value>()?;
            Ok(if res.is_null() { Value::Null } else { serde_json::json!(res) })
        }
        Commands::Visible { selector } => {
            let js = format!("const el = document.querySelector('{}'); if (!el) return false; const style = window.getComputedStyle(el); return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';", crate::calculations::escape_js_string(selector));
            Ok(serde_json::json!(page.evaluate(js.as_str()).await?.into_value::<Value>()?.as_bool().unwrap_or(false)))
        }
        Commands::Enabled { selector } => {
            let js = format!("const el = document.querySelector('{}'); return el ? !el.disabled : false;", crate::calculations::escape_js_string(selector));
            Ok(serde_json::json!(page.evaluate(js.as_str()).await?.into_value::<Value>()?.as_bool().unwrap_or(false)))
        }
        Commands::Selected { selector } => {
            let js = format!("const el = document.querySelector('{}'); return el ? (el.checked || el.selected) : false;", crate::calculations::escape_js_string(selector));
            Ok(serde_json::json!(page.evaluate(js.as_str()).await?.into_value::<Value>()?.as_bool().unwrap_or(false)))
        }
        Commands::Count { selector } => {
            let els = page.find_elements(selector.as_str()).await?;
            Ok(serde_json::json!(els.len()))
        }
        Commands::FindAll { selector } => {
            let js = format!("Array.from(document.querySelectorAll('{}')).map(el => el.outerHTML)", crate::calculations::escape_js_string(selector));
            let htmls = page.evaluate(js.as_str()).await?.into_value::<Value>()?;
            Ok(htmls)
        }
        Commands::Exists { selector } => Ok(serde_json::json!(page.find_element(selector.as_str()).await.is_ok())),
        _ => anyhow::bail!("Invalid query command"),
    }
}

async fn handle_storage(page: &Page, command: &Commands) -> Result<Value> {
    match command {
        Commands::Cookies => {
            let cookies = page.get_cookies().await?;
            let res: Vec<String> = cookies.into_iter().map(|c| format!("{}={}; Path={}; Domain={}", c.name, c.value, c.path, c.domain)).collect();
            Ok(serde_json::json!(res))
        }
        Commands::SetCookie { name, value, domain, path } => {
            let cookie_str = format!("{}={}; domain={}; path={}", crate::calculations::escape_js_string(name), crate::calculations::escape_js_string(value), domain.as_deref().unwrap_or(""), path.as_deref().unwrap_or(""));
            page.evaluate(format!("document.cookie = '{cookie_str}'; return true;").as_str()).await?;
            Ok(serde_json::json!(name))
        }
        Commands::DeleteCookie { name } => {
            page.evaluate(format!("document.cookie = '{}={}; Max-Age=0';", crate::calculations::escape_js_string(name), crate::calculations::escape_js_string(name)).as_str()).await?;
            Ok(serde_json::json!(name))
        }
        Commands::LocalGet { key } => Ok(page.evaluate(generate_storage_js("local", "get", Some(key), None).as_str()).await?.into_value::<Value>()?),
        Commands::LocalSet { key, value } => { page.evaluate(generate_storage_js("local", "set", Some(key), Some(value)).as_str()).await?; Ok(serde_json::json!(key)) }
        Commands::LocalRemove { key } => { page.evaluate(generate_storage_js("local", "remove", Some(key), None).as_str()).await?; Ok(serde_json::json!(key)) }
        Commands::LocalClear => { page.evaluate(generate_storage_js("local", "clear", None, None).as_str()).await?; Ok(serde_json::json!("cleared")) }
        Commands::SessionGet { key } => Ok(page.evaluate(generate_storage_js("session", "get", Some(key), None).as_str()).await?.into_value::<Value>()?),
        Commands::SessionSet { key, value } => { page.evaluate(generate_storage_js("session", "set", Some(key), Some(value)).as_str()).await?; Ok(serde_json::json!(key)) }
        Commands::SessionClear => { page.evaluate(generate_storage_js("session", "clear", None, None).as_str()).await?; Ok(serde_json::json!("cleared")) }
        _ => anyhow::bail!("Invalid storage command"),
    }
}
