# Contract Specification: dioxus-agent-rs

## Context

- **Feature**: Comprehensive WebDriver CLI tool for browser automation, specifically targeting Dioxus applications
- **Domain terms**:
  - `Client` - fantoccini WebDriver client connected to ChromeDriver
  - `Locator` - CSS selector for element identification
  - `Element` - WebElement reference returned by queries
  - `Session` - ChromeDriver session state
- **Assumptions**:
  - ChromeDriver runs on localhost:4444
  - Target browser is Chrome with headless mode
  - All operations are async (tokio runtime)
- **Open questions**:
  - How to handle multiple concurrent sessions?
  - Should element references be cached or queried fresh each time?

---

## Preconditions

| ID | Precondition | Enforcement Level | Type / Pattern |
|----|--------------|-------------------|----------------|
| P1 | WebDriver client must be connected to ChromeDriver | Compile-time | `Option<Client>` → use `Client` only when `Some` |
| P2 | CSS selector must be non-empty string | Runtime constructor | `NonEmptyString::new(selector)?` |
| P3 | URL must be valid URI format | Compile-time | `url::Url::parse()` returns `Result` |
| P4 | Element must exist in DOM before interaction | Runtime | `client.find()` returns `Ok(Element)` or `Err` |
| P5 | Viewport dimensions must be positive integers > 0 | Compile-time | `u32` with bounds check |
| P6 | Timeout duration must be > 0 milliseconds | Runtime | `Duration::from_millis()` with validation |
| P7 | JavaScript must not contain dangerous patterns | Runtime | sanitize `eval()`, `Function()`, `setTimeout` |
| P8 | Cookie name/value must be valid (no null bytes) | Runtime | UTF-8 validation |
| P9 | Storage key must be valid identifier | Runtime | regex `^[a-zA-Z_][a-zA-Z0-9_]*$` |

---

## Postconditions

| ID | Postcondition | Mutation Contract |
|----|---------------|-------------------|
| Q1 | `client.goto(url)` completes → browser at target URL | `Client.session_url` updated |
| Q2 | `client.find(selector)` returns → `Element` is attached to DOM | No mutation |
| Q3 | `element.click()` → element received click event | No mutation |
| Q4 | `element.send_keys(value)` → input value set | `Element.value` property mutated |
| Q5 | `client.execute(js, args)` → JS evaluated, value returned | No mutation |
| Q6 | `client.screenshot()` → PNG bytes written to path | File system mutated |
| Q7 | `client.delete_cookie(name)` → cookie removed from browser | Browser cookie jar mutated |
| Q8 | `client.viewport()` → browser viewport resized | Browser window state mutated |
| Q9 | After command execution → client remains connected | `Client.connected` invariant preserved |

---

## Invariants

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| I1 | Always connected to ChromeDriver session | Check after every operation |
| I2 | URL is valid HTTP/HTTPS format | Validate before navigation |
| I3 | Element references are valid for current page | Re-query after navigation |
| I4 | No zombie ChromeDriver processes on error | Clean shutdown via `client.close()` |
| I5 | Console logs captured without blocking execution | Async log collection |

---

## Error Taxonomy

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    // Connection errors
    #[error("ChromeDriver connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Session lost: ChromeDriver disconnected")]
    SessionLost,
    
    // Element errors
    #[error("Element not found: selector '{0}'")]
    ElementNotFound(String),
    
    #[error("Multiple elements found: selector '{0}' (expected 1)")]
    AmbiguousElement(String),
    
    #[error("Element not interactable: {0}")]
    ElementNotInteractable(String),
    
    // Timeout errors
    #[error("Timeout waiting for element '{0}' after {1}ms")]
    Timeout(String, u64),
    
    // Navigation errors
    #[error("Navigation failed: {0}")]
    NavigationFailed(String),
    
    // JavaScript errors
    #[error("JavaScript execution failed: {0}")]
    JavaScriptError(String),
    
    #[error("JavaScript injection blocked: dangerous pattern detected")]
    InjectionBlocked,
    
    // Storage errors
    #[error("Cookie operation failed: {0}")]
    CookieError(String),
    
    #[error("Storage operation failed: {0}")]
    StorageError(String),
    
    // Input validation errors
    #[error("Invalid selector: {0}")]
    InvalidSelector(String),
    
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    // IO errors
    #[error("File error: {0}")]
    IoError(#[from] std::io::Error),
}
```

---

## Contract Signatures

All fallible operations return `Result<T, Error>`:

```rust
// Navigation
fn get_dom(client: &Client) -> Result<String, Error>;
fn get_title(client: &Client) -> Result<String, Error>;
fn get_url(client: &Client) -> Result<String, Error>;
fn refresh(client: &Client) -> Result<(), Error>;
fn go_back(client: &Client) -> Result<(), Error>;
fn go_forward(client: &Client) -> Result<(), Error>;

// Element Interaction
fn click_element(client: &Client, selector: &str) -> Result<(), Error>;
fn double_click_element(client: &Client, selector: &str) -> Result<(), Error>;
fn right_click_element(client: &Client, selector: &str) -> Result<(), Error>;
fn hover_element(client: &Client, selector: &str) -> Result<(), Error>;
fn set_text(client: &Client, selector: &str, value: &str) -> Result<(), Error>;
fn clear_input(client: &Client, selector: &str) -> Result<(), Error>;
fn submit_form(client: &Client, selector: &str) -> Result<(), Error>;
fn select_option(client: &Client, selector: &str, value: &str) -> Result<(), Error>;
fn check_element(client: &Client, selector: &str) -> Result<(), Error>;
fn uncheck_element(client: &Client, selector: &str) -> Result<(), Error>;

// Element Queries
fn get_text(client: &Client, selector: &str) -> Result<String, Error>;
fn get_attribute(client: &Client, selector: &str, attr: &str) -> Result<Option<String>, Error>;
fn get_classes(client: &Client, selector: &str) -> Result<Vec<String>, Error>;
fn get_tag_name(client: &Client, selector: &str) -> Result<String, Error>;
fn is_visible(client: &Client, selector: &str) -> Result<bool, Error>;
fn is_enabled(client: &Client, selector: &str) -> Result<bool, Error>;
fn is_selected(client: &Client, selector: &str) -> Result<bool, Error>;
fn count_elements(client: &Client, selector: &str) -> Result<usize, Error>;
fn find_all_elements(client: &Client, selector: &str) -> Result<Vec<Element>, Error>;
fn element_exists(client: &Client, selector: &str) -> Result<bool, Error>;

// JavaScript & Execution
fn execute_js(client: &Client, js: &str, args: Vec<serde_json::Value>) -> Result<serde_json::Value, Error>;
fn inject_css(client: &Client, css: &str) -> Result<(), Error>;

// Screenshot
fn take_screenshot(client: &Client, path: &Path) -> Result<(), Error>;
fn take_element_screenshot(client: &Client, selector: &str, path: &Path) -> Result<(), Error>;

// Viewport & Scrolling
fn set_viewport(client: &Client, width: u32, height: u32) -> Result<(), Error>;
fn scroll_to_element(client: &Client, selector: &str) -> Result<(), Error>;
fn scroll_by(client: &Client, x: i32, y: i32) -> Result<(), Error>;

// Keyboard
fn press_key(client: &Client, key: &str) -> Result<(), Error>;
fn press_key_combo(client: &Client, combo: &str) -> Result<(), Error>;

// Storage
fn get_cookies(client: &Client) -> Result<Vec<Cookie>, Error>;
fn set_cookie(client: &Client, name: &str, value: &str, domain: Option<&str>, path: Option<&str>) -> Result<(), Error>;
fn delete_cookie(client: &Client, name: &str) -> Result<(), Error>;
fn local_storage_get(client: &Client, key: &str) -> Result<Option<String>, Error>;
fn local_storage_set(client: &Client, key: &str, value: &str) -> Result<(), Error>;
fn local_storage_remove(client: &Client, key: &str) -> Result<(), Error>;
fn local_storage_clear(client: &Client) -> Result<(), Error>;
fn session_storage_get(client: &Client, key: &str) -> Result<Option<String>, Error>;
fn session_storage_set(client: &Client, key: &str, value: &str) -> Result<(), Error>;
fn session_storage_clear(client: &Client) -> Result<(), Error>;

// Waiting
fn wait_for_element(client: &Client, selector: &str, timeout: Duration) -> Result<Element, Error>;
fn wait_for_element_gone(client: &Client, selector: &str, timeout: Duration) -> Result<(), Error>;
fn wait_for_navigation(client: &Client, timeout: Duration) -> Result<(), Error>;
fn wait_for_hydration(client: &Client) -> Result<(), Error>;

// Dioxus-Specific
fn get_dioxus_state(client: &Client) -> Result<serde_json::Value, Error>;
fn dioxus_click(client: &Client, target: &str) -> Result<(), Error>;

// Style
fn get_computed_style(client: &Client, selector: &str, property: &str) -> Result<String, Error>;
```

---

## Violation Examples

### Precondition Violations

- **VIOLATES P2**: `click_element(&client, "")` → should produce `Err(Error::InvalidSelector("selector cannot be empty".into()))`
- **VIOLATES P2**: `click_element(&client, "   ")` → should produce `Err(Error::InvalidSelector("selector cannot be whitespace only".into()))`
- **VIOLATES P3**: `client.goto("not-a-url")` → should produce `Err(Error::InvalidUrl("invalid uri".into()))`
- **VIOLATES P4**: `click_element(&client, "#nonexistent")` after page load → should produce `Err(Error::ElementNotFound("#nonexistent".into()))`
- **VIOLATES P5**: `set_viewport(&client, 0, 800)` → should produce `Err(Error::InvalidInput("width must be > 0".into()))`
- **VIOLATES P6**: `wait_for_element(&client, "body", Duration::from_millis(0))` → should produce `Err(Error::InvalidInput("timeout must be > 0".into()))`
- **VIOLATES P7**: `execute_js(&client, "eval(document.cookie)", [])` → should produce `Err(Error::InjectionBlocked)`
- **VIOLATES P8**: `set_cookie(&client, "bad\x00name", "value", None, None)` → should produce `Err(Error::CookieError("invalid name".into()))`

### Postcondition Violations

- **VIOLATES Q1**: After `goto("http://example.com")` → calling `get_url()` should return "http://example.com/" (not the original URL)
- **VIOLATES Q6**: After `screenshot("path.png")` → file should exist at path with valid PNG header

---

## Ownership Contracts

| Parameter Type | Function | Mutation Contract |
|---------------|----------|-------------------|
| `&Client` | Navigation, queries | No mutation - read-only |
| `&mut Client` | Close, internal state | `Client.connected` → false after close |
| `Element` ownership | Element actions | Ownership transferred to caller |
| `&Element` | Queries on element | No mutation - read-only |
| `&mut Element` | Text input, clear | `Element.value` property mutated |

### Clone Policy Decisions

- `Element` - Does NOT impl Clone (represents WebDriver reference, not safe to duplicate)
- `Client` - Does NOT impl Clone (single session ownership)
- `Cookie` - impl Clone (plain data transfer object)
- `serde_json::Value` - impl Clone (owned JSON data)

---

## Non-goals

- [ ] Multi-tab support (not in current spec)
- [ ] Proxy configuration (not in current spec)
- [ ] Browser extensions (not in current spec)
- [ ] File download handling (not in current spec)
- [ ] WebRTC/media controls (not in current spec)
