#[derive(serde::Serialize)]
pub struct TracePayload<'a> {
    pub command: String,
    pub url: &'a str,
    pub timestamp: u128,
    pub success: bool,
    pub screenshot: String,
    pub semantic_tree: String,
}

pub fn generate_trace_payload(
    command_name: &str,
    url: &str,
    timestamp: u128,
    success: bool,
    screenshot_path: &str,
    tree_path: &str,
) -> Result<String, serde_json::Error> {
    let payload = TracePayload {
        command: command_name.to_string(),
        url,
        timestamp,
        success,
        screenshot: screenshot_path.to_string(),
        semantic_tree: tree_path.to_string(),
    };
    serde_json::to_string_pretty(&payload)
}
