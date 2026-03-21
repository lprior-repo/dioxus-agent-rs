import re

with open('src/actions.rs', 'r') as f:
    content = f.read()

# Refactor AssertScreenshot to push math down to calculations.rs
# Black Hat flagged lines 316-332 doing f64 math inside actions.rs

replacement = """        Commands::AssertScreenshot { selector, baseline, failure_path, tolerance } => {
            let buf = if let Some(sel) = selector {
                let el = page.find_element(sel.as_str()).await?;
                el.screenshot(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png).await?
            } else {
                let params = chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotParams::builder().format(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png).build();
                page.screenshot(params).await?
            };
            
            if !std::path::Path::new(baseline).exists() {
                std::fs::write(baseline, &buf)?;
                return Ok(serde_json::json!("Baseline created"));
            }
            
            let baseline_buf = std::fs::read(baseline).context("Failed to load baseline from disk")?;
            let percent_diff = crate::calculations::calculate_pixel_diff(&buf, &baseline_buf)?;
            
            if percent_diff > *tolerance {
                std::fs::write(failure_path, buf)?;
                anyhow::bail!("Visual regression failed: {percent_diff:.2}% diff (tolerance: {tolerance:.2}%)");
            }
            
            Ok(serde_json::json!(true))
        }"""

content = re.sub(r'        Commands::AssertScreenshot.*?Ok\(serde_json::json!\(true\)\)\n        \}', replacement, content, flags=re.DOTALL)

with open('src/actions.rs', 'w') as f:
    f.write(content)
