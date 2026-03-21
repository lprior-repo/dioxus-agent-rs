import re

with open('src/actions.rs', 'r') as f:
    content = f.read()

# Fix ScreenshotAnnotated duplicates and map_err
def replace_eval_screenshot(match):
    return """async fn handle_eval_screenshot(page: &Page, command: &Commands) -> Result<Value> {
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
}"""

content = re.sub(r'async fn handle_eval_screenshot.*?_ => anyhow::bail!\("Invalid eval/screenshot command"\),\n    }\n}', replace_eval_screenshot, content, flags=re.DOTALL)

def replace_viewport(match):
    return """async fn handle_viewport_keyboard(page: &Page, command: &Commands) -> Result<Value> {
    match command {
        Commands::Viewport { width, height } => {
            let params = chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams::builder()
                .width(*width as i64)
                .height(*height as i64)
                .device_scale_factor(1.0)
                .mobile(false)
                .build();
            page.execute(params).await?;
            Ok(serde_json::json!(format!("{width} {height}")))
        }"""

content = re.sub(r'async fn handle_viewport_keyboard.*?Commands::Viewport.*?Ok\(serde_json::json!\(format!\("\{width\} \{height\}"\)\)\)\n        \}', replace_viewport, content, flags=re.DOTALL)

def replace_upload(match):
    return """        Commands::Upload { selector, path } => {
            let el = page.find_element(selector.as_str()).await?;
            let abs_path = std::fs::canonicalize(path)?;
            let params = chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams::builder()
                .files(vec![abs_path.to_string_lossy().to_string()])
                .node_id(el.node_id)
                .build();
            page.execute(params).await?;
            Ok(serde_json::json!(selector))
        }"""

content = re.sub(r'        Commands::Upload \{ selector, path \} => \{.*?Ok\(serde_json::json!\(selector\)\)\n        \}', replace_upload, content, flags=re.DOTALL)

with open('src/actions.rs', 'w') as f:
    f.write(content)
