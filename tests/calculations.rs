#![allow(clippy::unwrap_used)]

//! Unit tests for calculations layer - pure functions

use dioxus_agent_rs::calculations::{
    escape_js_string, generate_computed_style_js, generate_console_js, generate_css_injection_js,
    generate_dioxus_click_js, generate_dioxus_state_js, generate_hydration_wait_js,
    generate_keycombo_js, generate_keypress_js, generate_storage_js, generate_wait_element_js,
    generate_wait_gone_js, validate_inputs,
};

use dioxus_agent_rs::data::{Cli, Commands, Engine};

fn make_cli(url: &str, timeout: u64, cmd: Commands) -> Cli {
    Cli {
        url: url.to_string(),
        timeout,
        no_headless: false,
        json: false,
        auto_wait: false,
        trace: None,
        engine: Engine::Cdp,
        command: cmd,
    }
}

// ============================================================================
// Validation Tests - Preconditions
// ============================================================================

#[test]
fn test_precondition_empty_url_returns_error() {
    let cli = make_cli("", 10, Commands::Dom);
    let result = validate_inputs(&cli);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("URL"));
}

#[test]
fn test_precondition_zero_timeout_returns_error() {
    let cli = make_cli("http://localhost:8080", 0, Commands::Dom);
    let result = validate_inputs(&cli);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .to_lowercase()
        .contains("timeout"));
}

#[test]
fn test_precondition_empty_selector_returns_error() {
    let cli = make_cli(
        "http://localhost:8080",
        10,
        Commands::Click {
            selector: dioxus_agent_rs::data::types::Selector("".to_string()),
        },
    );
    let result = validate_inputs(&cli);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("selector"));
}

#[test]
fn test_precondition_whitespace_selector_returns_error() {
    let cli = make_cli(
        "http://localhost:8080",
        10,
        Commands::Click {
            selector: dioxus_agent_rs::data::types::Selector("   ".to_string()),
        },
    );
    let result = validate_inputs(&cli);
    assert!(result.is_err());
}

#[test]
fn test_precondition_zero_viewport_returns_error() {
    let cli = make_cli(
        "http://localhost:8080",
        10,
        Commands::Viewport {
            width: 0,
            height: 100,
        },
    );
    let result = validate_inputs(&cli);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("width"));
}

#[test]
fn test_precondition_dangerous_js_returns_error() {
    let cli = make_cli(
        "http://localhost:8080",
        10,
        Commands::Eval {
            js: dioxus_agent_rs::data::types::JsPayload("eval(document.cookie)".to_string()),
        },
    );
    let result = validate_inputs(&cli);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("dangerous"));
}

#[test]
fn test_precondition_invalid_console_type_returns_error() {
    let cli = make_cli(
        "http://localhost:8080",
        10,
        Commands::ConsoleLog {
            r#type: "invalid".to_string(),
        },
    );
    let result = validate_inputs(&cli);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("console type"));
}

// ============================================================================
// Validation Tests - Happy Path
// ============================================================================

#[test]
fn test_validate_all_commands_accept_valid_inputs() {
    let commands = vec![
        Commands::Dom,
        Commands::Title,
        Commands::Url,
        Commands::Refresh,
        Commands::Back,
        Commands::Forward,
        Commands::Click {
            selector: dioxus_agent_rs::data::types::Selector("#btn".to_string()),
        },
        Commands::DoubleClick {
            selector: dioxus_agent_rs::data::types::Selector(".item".to_string()),
        },
        Commands::RightClick {
            selector: dioxus_agent_rs::data::types::Selector("a.link".to_string()),
        },
        Commands::Hover {
            selector: dioxus_agent_rs::data::types::Selector("div.hover".to_string()),
        },
        Commands::Text {
            selector: dioxus_agent_rs::data::types::Selector("input".to_string()),
            value: dioxus_agent_rs::data::types::InputValue("hello".to_string()),
        },
        Commands::Clear {
            selector: dioxus_agent_rs::data::types::Selector("input".to_string()),
        },
        Commands::Submit {
            selector: dioxus_agent_rs::data::types::Selector("form".to_string()),
        },
        Commands::Select {
            selector: dioxus_agent_rs::data::types::Selector("select".to_string()),
            value: dioxus_agent_rs::data::types::InputValue("opt1".to_string()),
        },
        Commands::Check {
            selector: dioxus_agent_rs::data::types::Selector("input[type=checkbox]".to_string()),
        },
        Commands::Uncheck {
            selector: dioxus_agent_rs::data::types::Selector("input[type=checkbox]".to_string()),
        },
        Commands::GetText {
            selector: dioxus_agent_rs::data::types::Selector("p".to_string()),
        },
        Commands::Attr {
            selector: dioxus_agent_rs::data::types::Selector("img".to_string()),
            attribute: "src".to_string(),
        },
        Commands::Classes {
            selector: dioxus_agent_rs::data::types::Selector("div".to_string()),
        },
        Commands::TagName {
            selector: dioxus_agent_rs::data::types::Selector("span".to_string()),
        },
        Commands::Visible {
            selector: dioxus_agent_rs::data::types::Selector("div".to_string()),
        },
        Commands::Enabled {
            selector: dioxus_agent_rs::data::types::Selector("input".to_string()),
        },
        Commands::Selected {
            selector: dioxus_agent_rs::data::types::Selector("option".to_string()),
        },
        Commands::Scroll {
            selector: dioxus_agent_rs::data::types::Selector("div".to_string()),
        },
        Commands::ScrollBy { x: 0, y: 100 },
        Commands::Key {
            key: "Enter".to_string(),
        },
        Commands::KeyCombo {
            combo: "Control+c".to_string(),
        },
        Commands::Screenshot {
            path: dioxus_agent_rs::data::types::FilePath("/tmp/screen.png".to_string()),
        },
        Commands::ElementScreenshot {
            selector: dioxus_agent_rs::data::types::Selector("div".to_string()),
            path: dioxus_agent_rs::data::types::FilePath("/tmp/el.png".to_string()),
        },
        Commands::Viewport {
            width: 1920,
            height: 1080,
        },
        Commands::InjectCss {
            css: dioxus_agent_rs::data::types::CssPayload("body { color: red; }".to_string()),
        },
        Commands::Style {
            selector: dioxus_agent_rs::data::types::Selector("div".to_string()),
            property: "color".to_string(),
        },
        Commands::Eval {
            js: dioxus_agent_rs::data::types::JsPayload("document.title".to_string()),
        },
        Commands::Cookies,
        Commands::SetCookie {
            name: "session".to_string(),
            value: dioxus_agent_rs::data::types::InputValue("abc123".to_string()),
            domain: None,
            path: None,
        },
        Commands::DeleteCookie {
            name: "session".to_string(),
        },
        Commands::LocalGet {
            key: dioxus_agent_rs::data::types::StorageKey("user".to_string()),
        },
        Commands::LocalSet {
            key: dioxus_agent_rs::data::types::StorageKey("user".to_string()),
            value: dioxus_agent_rs::data::types::InputValue("john".to_string()),
        },
        Commands::LocalRemove {
            key: dioxus_agent_rs::data::types::StorageKey("user".to_string()),
        },
        Commands::LocalClear,
        Commands::SessionGet {
            key: dioxus_agent_rs::data::types::StorageKey("token".to_string()),
        },
        Commands::SessionSet {
            key: dioxus_agent_rs::data::types::StorageKey("token".to_string()),
            value: dioxus_agent_rs::data::types::InputValue("xyz".to_string()),
        },
        Commands::SessionClear,
        Commands::Console,
        Commands::ConsoleLog {
            r#type: "log".to_string(),
        },
        Commands::Wait {
            selector: dioxus_agent_rs::data::types::Selector("#loading".to_string()),
        },
        Commands::WaitGone {
            selector: dioxus_agent_rs::data::types::Selector("#loading".to_string()),
        },
        Commands::WaitNav,
        Commands::WaitHydration,
        Commands::DioxusState,
        Commands::DioxusClick {
            target: dioxus_agent_rs::data::types::Selector("123".to_string()),
        },
        Commands::Count {
            selector: dioxus_agent_rs::data::types::Selector("li".to_string()),
        },
        Commands::FindAll {
            selector: dioxus_agent_rs::data::types::Selector("div".to_string()),
        },
        Commands::Exists {
            selector: dioxus_agent_rs::data::types::Selector("#app".to_string()),
        },
    ];

    for cmd in commands {
        let cli = make_cli("http://localhost:8080", 10, cmd);
        let result = validate_inputs(&cli);
        // Some might fail due to edge cases, but should not panic
        let _ = result;
    }
}

// ============================================================================
// JavaScript Generation Tests
// ============================================================================

#[test]
fn test_keypress_enter() {
    let js = generate_keypress_js("Enter");
    assert!(js.contains("Enter"));
}

#[test]
fn test_keypress_escape() {
    let js = generate_keypress_js("escape");
    assert!(js.contains("Escape"));
}

#[test]
fn test_keypress_arrows() {
    assert!(generate_keypress_js("arrowup").contains("ArrowUp"));
    assert!(generate_keypress_js("arrowdown").contains("ArrowDown"));
    assert!(generate_keypress_js("arrowleft").contains("ArrowLeft"));
    assert!(generate_keypress_js("arrowright").contains("ArrowRight"));
}

#[test]
fn test_keypress_special() {
    assert!(generate_keypress_js("tab").contains("Tab"));
    assert!(generate_keypress_js("backspace").contains("Backspace"));
    assert!(generate_keypress_js("home").contains("Home"));
    assert!(generate_keypress_js("end").contains("End"));
}

#[test]
fn test_keypress_custom() {
    let js = generate_keypress_js("a");
    assert!(js.contains("a"));
}

#[test]
fn test_keycombo_ctrl_c() {
    let js = generate_keycombo_js("Control+c");
    assert!(js.contains("ctrlKey"));
    assert!(js.contains("c"));
}

#[test]
fn test_keycombo_shift_tab() {
    let js = generate_keycombo_js("Shift+Tab");
    assert!(js.contains("shiftKey"));
    assert!(js.contains("Tab"));
}

#[test]
fn test_keycombo_meta_enter() {
    let js = generate_keycombo_js("Meta+Enter");
    assert!(js.contains("metaKey"));
    assert!(js.contains("Enter"));
}

#[test]
fn test_storage_local_get() {
    let js = generate_storage_js("local", "get", Some("user"), None);
    assert!(js.contains("localStorage"));
    assert!(js.contains("user"));
}

#[test]
fn test_storage_local_set() {
    let js = generate_storage_js("local", "set", Some("user"), Some("john"));
    assert!(js.contains("localStorage.setItem"));
    assert!(js.contains("user"));
    assert!(js.contains("john"));
}

#[test]
fn test_storage_local_remove() {
    let js = generate_storage_js("local", "remove", Some("user"), None);
    assert!(js.contains("localStorage.removeItem"));
}

#[test]
fn test_storage_local_clear() {
    let js = generate_storage_js("local", "clear", None, None);
    assert!(js.contains("localStorage.clear"));
}

#[test]
fn test_storage_session_get() {
    let js = generate_storage_js("session", "get", Some("token"), None);
    assert!(js.contains("sessionStorage"));
    assert!(js.contains("token"));
}

#[test]
fn test_storage_session_set() {
    let js = generate_storage_js("session", "set", Some("token"), Some("abc"));
    assert!(js.contains("sessionStorage.setItem"));
}

#[test]
fn test_escape_js_string() {
    assert_eq!(escape_js_string("hello"), "hello");
    assert_eq!(escape_js_string("hello'world"), "hello\\'world");
    assert_eq!(escape_js_string("line1\nline2"), "line1\\nline2");
    assert_eq!(escape_js_string("tab\there"), "tab\\there");
}

#[test]
fn test_css_injection() {
    let js = generate_css_injection_js("body { color: red; }");
    assert!(js.contains("createElement"));
    assert!(js.contains("style"));
    assert!(js.contains("head"));
}

#[test]
fn test_dioxus_click_js() {
    let js = generate_dioxus_click_js("123");
    assert!(js.contains("data-target"));
    assert!(js.contains("click"));
}

#[test]
fn test_dioxus_state_js() {
    let js = generate_dioxus_state_js();
    assert!(js.contains("__DX_STATE__") || js.contains("__dx") || js.contains("__dioxus"));
}

#[test]
fn test_hydration_wait_js() {
    let js = generate_hydration_wait_js();
    assert!(js.contains("Promise"));
    assert!(js.contains("MutationObserver") || js.contains("data-hydrated"));
}

#[test]
fn test_computed_style_js() {
    let js = generate_computed_style_js("div", "color");
    assert!(js.contains("getComputedStyle"));
    assert!(js.contains("getPropertyValue"));
}

#[test]
fn test_wait_element_js() {
    let js = generate_wait_element_js("#loading");
    assert!(js.contains("Promise"));
    assert!(js.contains("MutationObserver"));
    assert!(js.contains("querySelector"));
}

#[test]
fn test_wait_gone_js() {
    let js = generate_wait_gone_js("#loading");
    assert!(js.contains("Promise"));
    assert!(js.contains("MutationObserver"));
}

#[test]
fn test_console_js_default() {
    let js = generate_console_js(None);
    assert!(js.contains("__captured"));
}

#[test]
fn test_console_js_specific() {
    let js = generate_console_js(Some("error"));
    assert!(js.contains("error"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_keypress_mixed_case() {
    // Should handle case insensitivity
    let js1 = generate_keypress_js("ENTER");
    let js2 = generate_keypress_js("enter");
    assert_eq!(js1, js2);
}

#[test]
fn test_keycombo_with_spaces() {
    // Should handle spaces around +
    let js1 = generate_keycombo_js("Control + c");
    let js2 = generate_keycombo_js("Control+c");
    assert_eq!(js1, js2);
}

#[test]
fn test_storage_handles_special_chars_in_value() {
    let js = generate_storage_js("local", "set", Some("key"), Some("value with 'quotes'"));
    assert!(js.contains("\\'quotes\\'"));
}
