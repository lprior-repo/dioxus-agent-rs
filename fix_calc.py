import re

with open('src/calculations.rs', 'r') as f:
    content = f.read()

content = content.replace('    #[error("Invalid WebDriver URL: {0}")]\n    InvalidWebDriverUrl(url::ParseError),\n', '')
content = content.replace('    let webdriver_url =\n        Url::parse(&cli.webdriver_url).map_err(ValidationError::InvalidWebDriverUrl)?;\n\n', '')
content = content.replace('        timeout: Duration::from_secs(cli.timeout),\n        webdriver_url,\n', '        timeout: Duration::from_secs(cli.timeout),\n')

# Also remove generate_element_screenshot_js
content = re.sub(r'#\[must_use\]\npub fn generate_element_screenshot_js.*?\n}', '', content, flags=re.DOTALL)

with open('src/calculations.rs', 'w') as f:
    f.write(content)

with open('tests/calculations.rs', 'r') as f:
    content = f.read()
    
content = content.replace('        webdriver_url: "http://localhost:4444".to_string(),\n', '')

with open('tests/calculations.rs', 'w') as f:
    f.write(content)
