use fantoccini::{ClientBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut caps = serde_json::Map::new();
    let chrome_opts = serde_json::json!({
        "args": ["headless", "no-sandbox", "disable-dev-shm-usage", "disable-gpu"]
    });
    caps.insert("goog:chromeOptions".to_string(), chrome_opts);

    let client = ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:4444")
        .await?;

    let session_caps = client.capabilities().cloned().unwrap_or_default();
    
    if let Some(opts) = session_caps.get("goog:chromeOptions")
        && let Some(debugger_address) = opts.get("debuggerAddress").and_then(|v| v.as_str()) {
            println!("Debugger Address: {}", debugger_address);
            
            let req_url = format!("http://{}/json/version", debugger_address);
            let resp: serde_json::Value = reqwest::get(&req_url).await?.json().await?;
            println!("JSON Version info: {}", serde_json::to_string_pretty(&resp)?);
            
            if let Some(ws_url) = resp.get("webSocketDebuggerUrl").and_then(|v| v.as_str()) {
                println!("Got WS URL: {}", ws_url);
                // Try connecting chromiumoxide
                let (browser, mut handler) = chromiumoxide::browser::Browser::connect(ws_url).await?;
                let _handle = tokio::task::spawn(async move {
                    while let Some(h) = futures::StreamExt::next(&mut handler).await {
                        if h.is_err() { break; }
                    }
                });
                
                let pages = browser.pages().await?;
                println!("Connected to chromiumoxide! Pages count: {}", pages.len());
                if let Some(page) = pages.first() {
                    page.goto("https://example.com").await?;
                    println!("Navigated page via CDP!");
                }
            }
        }

    client.close().await?;
    Ok(())
}

