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

use crate::calculations::generate_hydration_wait_js;
pub mod handlers;
use crate::data::{BrowserMode, Commands, Config, Engine, OutputFormat, WaitStrategy};
use anyhow::{Context, Result};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::page::Page;
use fantoccini::{ClientBuilder, Locator};
use futures::StreamExt;
use serde_json::Value;

/// Executes a command.
///
/// # Errors
///
/// Returns an error if the browser fails to launch, navigate, or if the command execution fails.
pub async fn execute_command(config: Config) -> Result<()> {
    if config.engine == Engine::Dual {
        execute_dual_engine(config).await
    } else {
        execute_cdp_engine(config).await
    }
}

async fn execute_cdp_engine(config: Config) -> Result<()> {
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

    let _ = page.wait_for_navigation().await;

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

    if let Some(trace_dir) = &config.trace
        && let Err(e) = handlers::execute_trace(&page, trace_dir, &config, result.is_ok()).await {
            eprintln!("Warning: Failed to execute trace: {e}");
        }

    let _ = browser.close().await;
    print_output(&config, result)
}

async fn execute_dual_engine(config: Config) -> Result<()> {
    let mut caps = serde_json::Map::new();
    let mut args = vec!["no-sandbox", "disable-dev-shm-usage", "disable-gpu"];
    if config.mode == BrowserMode::Headless {
        args.push("headless");
    }
    caps.insert("goog:chromeOptions".to_string(), serde_json::json!({ "args": args }));

    // Connect WebDriver
    let client = ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:4444")
        .await.context("Failed to connect to ChromeDriver on port 4444")?;

    // Use scopeguard to guarantee we close the session even if we bail early
    let mut client = scopeguard::guard(client, |c| {
        // We must spawn a task to run the async close, because Drop is synchronous
        tokio::spawn(async move {
            let _ = c.close().await;
        });
    });

    // Extract CDP WebSocket URL
    let session_caps = client.capabilities().cloned().unwrap_or_default();
    let ws_url = session_caps.get("goog:chromeOptions")
        .and_then(|opts| opts.get("debuggerAddress"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to find CDP debuggerAddress from WebDriver"))?;

    let req_url = format!("http://{ws_url}/json/version");
    let resp: Value = reqwest::get(&req_url).await?.json().await?;
    let cdp_ws = resp.get("webSocketDebuggerUrl")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract webSocketDebuggerUrl"))?;

    // Connect CDP Shadow Session
    let (browser, mut handler) = chromiumoxide::browser::Browser::connect(cdp_ws).await?;
    
    // Spawn handler loop with an abort handle so we don't leak the task
    let (abort_handle, abort_registration) = futures::future::AbortHandle::new_pair();
    let _task = tokio::task::spawn(futures::future::Abortable::new(async move {
        while let Some(h) = handler.next().await {
            if h.is_err() { break; }
        }
    }, abort_registration));
    
    // scopeguard to clean up the task
    let _task_guard = scopeguard::guard(abort_handle, |a| {
        a.abort();
    });

    // Wait for CDP target discovery to actually fire
    let mut page_opt = None;
    for _ in 0..10 {
        let pages = browser.pages().await?;
        if let Some(p) = pages.first().cloned() {
            page_opt = Some(p);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let page = page_opt.ok_or_else(|| anyhow::anyhow!("No pages found in CDP after waiting"))?;

    // Inject console capture via CDP BEFORE navigating via WebDriver
    // Chromiumoxide supports evaluate_on_new_document to survive navigation
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
    let params = chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams::builder().source(js).build().map_err(|e| anyhow::anyhow!(e))?;
    page.execute(params).await?;

    // Now navigate using WebDriver
    client.goto(config.url.as_str()).await?;

    if config.wait == WaitStrategy::Auto {
        let wait_js = generate_hydration_wait_js();
        let _ = page.evaluate(wait_js).await;
    }

    let result = match tokio::time::timeout(
        config.timeout,
        dispatch_dual(&mut client, &page, &config.command),
    ).await {
        Ok(res) => res,
        Err(_) => Err(anyhow::anyhow!("Command execution timed out after {:?}", config.timeout)),
    };

    if let Some(trace_dir) = &config.trace
        && let Err(e) = handlers::execute_trace(&page, trace_dir, &config, result.is_ok()).await {
            eprintln!("Warning: Failed to execute trace: {e}");
        }

    // Drops happen here, triggering scopeguards
    print_output(&config, result)
}

fn print_output(config: &Config, result: Result<Value>) -> Result<()> {
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

async fn dispatch_dual(client: &mut fantoccini::Client, page: &Page, command: &Commands) -> Result<Value> {
    match command {
        // Core Interactions routed to WebDriver (fantoccini) for W3C stability
        Commands::Click { selector } => {
            client.find(Locator::Css(selector)).await?.click().await?;
            Ok(serde_json::json!(selector))
        }
        Commands::Text { selector, value } => {
            client.find(Locator::Css(selector)).await?.send_keys(value).await?;
            Ok(serde_json::json!(selector))
        }
        Commands::Clear { selector } => {
            client.find(Locator::Css(selector)).await?.clear().await?;
            Ok(serde_json::json!(selector))
        }
        // Everything else falls back to CDP (chromiumoxide)
        _ => dispatch_command(page, command).await,
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
        Commands::Dom | Commands::Title | Commands::Url | Commands::Refresh | Commands::Back | Commands::Forward => handlers::handle_navigation(page, command).await,
        Commands::Click { .. } | Commands::DoubleClick { .. } | Commands::RightClick { .. } | Commands::Hover { .. } | Commands::Text { .. } | Commands::Clear { .. } | Commands::Submit { .. } | Commands::Select { .. } | Commands::Check { .. } | Commands::Uncheck { .. } => handlers::handle_interaction(page, command).await,
        Commands::GetText { .. } | Commands::Attr { .. } | Commands::Classes { .. } | Commands::TagName { .. } | Commands::Visible { .. } | Commands::Enabled { .. } | Commands::Selected { .. } | Commands::Count { .. } | Commands::FindAll { .. } | Commands::Exists { .. } => handlers::handle_queries(page, command).await,
        Commands::Cookies | Commands::SetCookie { .. } | Commands::DeleteCookie { .. } | Commands::LocalGet { .. } | Commands::LocalSet { .. } | Commands::LocalRemove { .. } | Commands::LocalClear | Commands::SessionGet { .. } | Commands::SessionSet { .. } | Commands::SessionClear => handlers::handle_storage(page, command).await,
        Commands::Eval { .. } | Commands::InjectCss { .. } | Commands::Screenshot { .. } | Commands::ElementScreenshot { .. } | Commands::ScreenshotAnnotated { .. } | Commands::AssertScreenshot { .. } => handlers::handle_eval_screenshot(page, command).await,
        Commands::Viewport { .. } | Commands::Scroll { .. } | Commands::ScrollBy { .. } | Commands::Key { .. } | Commands::KeyCombo { .. } => handlers::handle_viewport_keyboard(page, command).await,
        Commands::Console | Commands::ConsoleLog { .. } | Commands::Wait { .. } | Commands::WaitGone { .. } | Commands::WaitNav | Commands::WaitHydration | Commands::WaitStable { .. } => handlers::handle_console_wait(page, command).await,
        Commands::DioxusState | Commands::DioxusClick { .. } | Commands::SemanticTree | Commands::Style { .. } => handlers::handle_dioxus_style(page, command).await,
        Commands::Upload { .. } | Commands::FillForm { .. } | Commands::NetworkLogs | Commands::AssertText { .. } | Commands::AssertVisible { .. } | Commands::AssertExists { .. } | Commands::FuzzyClick { .. } | Commands::WaitNetworkIdle | Commands::ScrollToText { .. } | Commands::ExtractTable { .. } => handlers::handle_ai_extended(page, command).await,
        Commands::MockRoute { .. } | Commands::ShadowClick { .. } | Commands::DragAndDrop { .. } | Commands::ExportState { .. } | Commands::ImportState { .. } => handlers::handle_god_tier(page, command).await,
        Commands::Repl => Ok(Value::Null),
    }
}

