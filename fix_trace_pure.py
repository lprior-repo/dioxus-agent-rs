import re

with open('src/actions.rs', 'r') as f:
    content = f.read()

trace_logic = """
    if let Some(trace_dir) = &config.trace {
        let _ = std::fs::create_dir_all(trace_dir);
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
        
        let trace_file = format!("{trace_dir}/{timestamp}-trace.json");
        let png_file = format!("{trace_dir}/{timestamp}-screenshot.png");
        let tree_file = format!("{trace_dir}/{timestamp}-semantic.txt");
        
        let params = chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotParams::builder().format(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png).build();
        if let Ok(png) = page.screenshot(params).await {
            let _ = std::fs::write(&png_file, png);
        }
        
        if let Ok(tree) = page.evaluate(generate_semantic_tree_js().as_str()).await
            && let Ok(tree_str) = tree.into_value::<String>() {
                let _ = std::fs::write(&tree_file, tree_str);
            }
        
        let trace_data = serde_json::json!({
            "command": format!("{:?}", config.command),
            "url": config.url.as_str(),
            "timestamp": timestamp,
            "success": result.is_ok(),
            "screenshot": png_file,
            "semantic_tree": tree_file,
        });
        let _ = std::fs::write(trace_file, serde_json::to_string_pretty(&trace_data).unwrap_or_default());
    }
"""

replacement = """
    if let Some(trace_dir) = &config.trace {
        if let Err(e) = execute_trace(&page, trace_dir, &config, result.is_ok()).await {
            eprintln!("Warning: Failed to execute trace: {e}");
        }
    }
"""

content = content.replace(trace_logic, replacement)

new_func = """
async fn execute_trace(page: &Page, trace_dir: &str, config: &Config, success: bool) -> Result<()> {
    std::fs::create_dir_all(trace_dir)?;
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
    let trace_file = format!("{trace_dir}/{timestamp}-trace.json");
    let png_file = format!("{trace_dir}/{timestamp}-screenshot.png");
    let tree_file = format!("{trace_dir}/{timestamp}-semantic.txt");
    let params = chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotParams::builder().format(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png).build();
    let png = page.screenshot(params).await?;
    std::fs::write(&png_file, png)?;
    let tree = page.evaluate(generate_semantic_tree_js().as_str()).await?;
    let tree_str = tree.into_value::<String>()?;
    std::fs::write(&tree_file, tree_str)?;
    let payload = crate::calculations::generate_trace_payload(&format!("{:?}", config.command), config.url.as_str(), timestamp, success, &png_file, &tree_file)?;
    std::fs::write(trace_file, payload)?;
    Ok(())
}
"""

content = content + "\n" + new_func

with open('src/actions.rs', 'w') as f:
    f.write(content)
