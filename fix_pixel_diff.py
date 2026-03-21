import re

with open('src/calculations.rs', 'r') as f:
    content = f.read()

# I need to add image processing logic to calculations.rs
new_calc = """
#[derive(thiserror::Error, Debug)]
pub enum PixelDiffError {
    #[error("Failed to load screenshot: {0}")]
    LoadImage(#[from] image::ImageError),
    #[error("Dimensions mismatch. Baseline: {0}x{1}, New: {2}x{3}")]
    DimensionsMismatch(u32, u32, u32, u32),
    #[error("Visual regression failed: {0:.2}% diff (tolerance: {1:.2}%)")]
    ToleranceExceeded(f64, f64),
}

/// Computes the pixel difference percentage between two images.
/// 
/// # Errors
/// 
/// Returns `PixelDiffError` if the images cannot be parsed or if dimensions do not match.
pub fn calculate_pixel_diff(img1_buf: &[u8], img2_buf: &[u8]) -> Result<f64, PixelDiffError> {
    let img1 = image::load_from_memory(img1_buf)?;
    let img2 = image::load_from_memory(img2_buf)?;

    if img1.width() != img2.width() || img1.height() != img2.height() {
        return Err(PixelDiffError::DimensionsMismatch(
            img2.width(),
            img2.height(),
            img1.width(),
            img1.height(),
        ));
    }

    let img1_rgb = img1.to_rgb8();
    let img2_rgb = img2.to_rgb8();

    let mut diff_pixels = 0;
    let total_pixels = img1.width() * img1.height();

    for (p1, p2) in img1_rgb.pixels().zip(img2_rgb.pixels()) {
        let r_diff = i32::from(p1[0]) - i32::from(p2[0]);
        let g_diff = i32::from(p1[1]) - i32::from(p2[1]);
        let b_diff = i32::from(p1[2]) - i32::from(p2[2]);
        if r_diff.abs() + g_diff.abs() + b_diff.abs() > 10 {
            diff_pixels += 1;
        }
    }

    Ok((f64::from(diff_pixels) / f64::from(total_pixels)) * 100.0)
}
"""

content = content + "\n" + new_calc

# Remove escape_js_string entirely and replace it with serde_json serialization
content = re.sub(r'#\[must_use\]\npub fn escape_js_string\(.*?\n}', '', content, flags=re.DOTALL)
content = re.sub(r'escape_js_string\(&([^)]+)\)', r'serde_json::to_string(&\1).unwrap_or_else(|_| "\"\"".to_string())', content)
content = re.sub(r'escape_js_string\(([^)]+)\)', r'serde_json::to_string(&\1).unwrap_or_else(|_| "\"\"".to_string())', content)
content = content.replace("format!(\"return {{ key: '{}' }}\", serde_json::to_string(&key).unwrap_or_else(|_| \"\\\"\\\"\".to_string()))", "format!(\"return {{ key: {} }}\", serde_json::to_string(&key).unwrap_or_else(|_| \"\\\"\\\"\".to_string()))")
content = content.replace("format!(\"return {{ key: '{}', ctrlKey: false, shiftKey: false, altKey: false, metaKey: false }}\"", "format!(\"return {{ key: {}, ctrlKey: false, shiftKey: false, altKey: false, metaKey: false }}\"")
content = content.replace("format!(\"return {{ key: '{}', {} }}\"", "format!(\"return {{ key: {}, {} }}\"")

# We will need a proper manual fix because serde_json::to_string includes the double quotes.
# So format!("return getItem('{}')", x) becomes format!("return getItem({})", x)

with open('src/calculations.rs', 'w') as f:
    f.write(content)
