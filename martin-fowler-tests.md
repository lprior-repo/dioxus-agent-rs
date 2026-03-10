# Martin Fowler Test Plan: dioxus-agent-rs

## Test Organization

- **Happy Path Tests**: Core functionality working correctly
- **Error Path Tests**: Each failure mode handled gracefully
- **Edge Case Tests**: Boundary conditions and empty inputs
- **Contract Verification Tests**: Preconditions, postconditions, invariants
- **Contract Violation Tests**: Matching each violation example from contract-spec.md
- **End-to-End Scenario**: Full workflow from connection to cleanup

---

## Happy Path Tests

### Navigation Flow

- **test_navigation_dom_returns_html**
  Given: Client connected to ChromeDriver, page loaded with content
  When: Call `get_dom(client)`
  Then: Returns non-empty HTML string containing `<html>`

- **test_navigation_title_returns_page_title**
  Given: Client connected, page with `<title>Test Page</title>` loaded
  When: Call `get_title(client)`
  Then: Returns "Test Page"

- **test_navigation_url_returns_current_url**
  Given: Client connected, navigated to "http://example.com"
  When: Call `get_url(client)`
  Then: Returns "http://www.example.com/" (normalized)

- **test_navigation_refresh_reloads_page**
  Given: Client connected, page with dynamic content loaded
  When: Call `refresh(client)`
  Then: Page reloads, returns Ok

- **test_navigation_back_goes_to_previous_page**
  Given: Client connected, navigated A → B
  When: Call `go_back(client)`
  Then: Current URL is page A

- **test_navigation_forward_goes_to_next_page**
  Given: Client connected, navigated A → B → back to A
  When: Call `go_forward(client)`
  Then: Current URL is page B

### Element Interaction

- **test_click_element_triggers_click_event**
  Given: Page with `<button id="btn">Click</button>`
  When: Call `click_element(client, "#btn")`
  Then: Returns Ok, button received click

- **test_double_click_element_triggers_dblclick**
  Given: Page with `<div id="target"></div>` with dblclick handler
  When: Call `double_click_element(client, "#target")`
  Then: Returns Ok, dblclick event fired

- **test_right_click_opens_context_menu**
  Given: Page with contextmenu handler
  When: Call `right_click_element(client, "#target")`
  Then: Returns Ok, contextmenu event fired

- **test_hover_element_triggers_mouseenter**
  Given: Page with mouseenter handler
  When: Call `hover_element(client, "#target")`
  Then: Returns Ok, mouseenter event fired

- **test_set_text_populates_input_field**
  Given: Page with `<input id="input" type="text">`
  When: Call `set_text(client, "#input", "hello")`
  Then: Input value is "hello"

- **test_clear_input_removes_value**
  Given: Page with `<input id="input" value="existing">`
  When: Call `clear_input(client, "#input")`
  Then: Input value is empty

- **test_submit_form_triggers_submission**
  Given: Page with `<form id="form"><button type="submit">Submit</button></form>`
  When: Call `submit_form(client, "#form")`
  Then: Form submit event fires

- **test_select_option_changes_dropdown**
  Given: Page with `<select id="select"><option value="a">A</option><option value="b">B</option></select>`
  When: Call `select_option(client, "#select", "b")`
  Then: Selected value is "b"

- **test_check_element_checks_checkbox**
  Given: Page with `<input type="checkbox" id="check">`
  When: Call `check_element(client, "#check")`
  Then: Checkbox is checked

- **test_uncheck_element_unchecks_checkbox**
  Given: Page with `<input type="checkbox" id="check" checked>`
  When: Call `uncheck_element(client, "#check")`
  Then: Checkbox is unchecked

### Element Queries

- **test_get_text_returns_element_text_content**
  Given: Page with `<div id="text">Hello World</div>`
  When: Call `get_text(client, "#text")`
  Then: Returns "Hello World"

- **test_get_attribute_returns_attribute_value**
  Given: Page with `<a id="link" href="http://example.com">Link</a>`
  When: Call `get_attribute(client, "#link", "href")`
  Then: Returns Some("http://example.com")

- **test_get_classes_returns_css_classes**
  Given: Page with `<div id="el" class="foo bar baz">`
  When: Call `get_classes(client, "#el")`
  Then: Returns ["foo", "bar", "baz"]

- **test_get_tag_name_returns_element_tag**
  Given: Page with `<span id="el">`
  When: Call `get_tag_name(client, "#el")`
  Then: Returns "span"

- **test_visible_returns_true_for_visible_element**
  Given: Page with visible `<div id="visible">`
  When: Call `is_visible(client, "#visible")`
  Then: Returns true

- **test_enabled_returns_true_for_enabled_input**
  Given: Page with enabled `<input id="enabled">`
  When: Call `is_enabled(client, "#enabled")`
  Then: Returns true

- **test_selected_returns_true_for_selected_option**
  Given: Page with `<option selected id="sel">`
  When: Call `is_selected(client, "#sel")`
  Then: Returns true

- **test_count_returns_number_of_matching_elements**
  Given: Page with three `<div class="item">`
  When: Call `count_elements(client, ".item")`
  Then: Returns 3

- **test_find_all_returns_all_element_html**
  Given: Page with multiple elements
  When: Call `find_all_elements(client, ".item")`
  Then: Returns Vec<Element> with all matches

- **test_exists_returns_true_when_element_present**
  Given: Page with `<div id="present">`
  When: Call `element_exists(client, "#present")`
  Then: Returns true

- **test_exists_returns_false_when_element_absent**
  Given: Page without `#absent`
  When: Call `element_exists(client, "#absent")`
  Then: Returns false

### Storage Operations

- **test_cookies_returns_all_cookies**
  Given: Browser with cookies set
  When: Call `get_cookies(client)`
  Then: Returns Vec<Cookie> with all cookies

- **test_set_cookie_creates_cookie**
  Given: Clean cookie jar
  When: Call `set_cookie(client, "test", "value", None, None)`
  Then: Cookie "test=value" exists

- **test_delete_cookie_removes_cookie**
  Given: Cookie "todelete" exists
  When: Call `delete_cookie(client, "todelete")`
  Then: Cookie is removed

- **test_local_storage_get_retrieves_value**
  Given: localStorage.setItem("key", "value")
  When: Call `local_storage_get(client, "key")`
  Then: Returns Some("value")

- **test_local_storage_set_stores_value**
  Given: Empty localStorage
  When: Call `local_storage_set(client, "newkey", "newvalue")`
  Then: localStorage.getItem("newkey") returns "newvalue"

- **test_local_storage_remove_deletes_item**
  Given: localStorage has item
  When: Call `local_storage_remove(client, "key")`
  Then: Item removed from localStorage

- **test_local_storage_clear_removes_all**
  Given: localStorage has multiple items
  When: Call `local_storage_clear(client)`
  Then: localStorage is empty

- **test_session_storage_get_retrieves_value**
  Given: sessionStorage.setItem("key", "value")
  When: Call `session_storage_get(client, "key")`
  Then: Returns Some("value")

- **test_session_storage_set_stores_value**
  Given: Empty sessionStorage
  When: Call `session_storage_set(client, "key", "value")`
  Then: Value stored in sessionStorage

- **test_session_storage_clear_removes_all**
  Given: sessionStorage has items
  When: Call `session_storage_clear(client)`
  Then: sessionStorage is empty

### Dioxus-Specific

- **test_wait_hydration_waits_for_dioxus_ready**
  Given: Dioxus app loading
  When: Call `wait_for_hydration(client)`
  Then: Returns Ok after hydration complete

- **test_dioxus_state_returns_app_state**
  Given: Dioxus app with state
  When: Call `get_dioxus_state(client)`
  Then: Returns JSON with Dioxus state

- **test_dioxus_click_clicks_by_target**
  Given: Dioxus app with `<div id="app"><button data-target="btn-1">`
  When: Call `dioxus_click(client, "btn-1")`
  Then: Button clicked

### JavaScript & Screenshot

- **test_eval_executes_javascript**
  When: Call `execute_js(client, "return 1 + 1", [])`
  Then: Returns 2

- **test_inject_css_applies_styles**
  Given: Page with element
  When: Call `inject_css(client, "#target { color: red }")`
  Then: Element computed style includes color: red

- **test_screenshot_captures_page**
  Given: Page loaded
  When: Call `take_screenshot(client, "/tmp/test.png")`
  Then: File exists with valid PNG

- **test_element_screenshot_captures_element**
  Given: Element visible
  When: Call `take_element_screenshot(client, "#el", "/tmp/el.png")`
  Then: File exists with element image

### Viewport & Keyboard

- **test_viewport_sets_dimensions**
  When: Call `set_viewport(client, 1920, 1080)`
  Then: Browser viewport is 1920x1080

- **test_scroll_to_element_scrolls_view**
  Given: Long page with element at bottom
  When: Call `scroll_to_element(client, "#bottom")`
  Then: Element is in viewport

- **test_scroll_by_scrolls_by_pixels**
  When: Call `scroll_by(client, 0, 100)`
  Then: Page scrolled down 100px

- **test_key_presses_key**
  Given: Input focused
  When: Call `press_key(client, "Enter")`
  Then: Enter key pressed

- **test_key_combo_presses_combo**
  Given: Input focused
  When: Call `press_key_combo(client, "Control+a")`
  Then: All text selected

---

## Error Path Tests

- **test_element_not_found_returns_error**
  Given: Page loaded without element
  When: Call `click_element(client, "#does-not-exist")`
  Then: Returns `Err(Error::ElementNotFound("#does-not-exist".into()))`

- **test_connection_failed_returns_error**
  Given: ChromeDriver not running
  When: Attempt to connect
  Then: Returns `Err(Error::ConnectionFailed(...))`

- **test_session_lost_returns_error**
  Given: Connected session, ChromeDriver crashes
  When: Call any operation
  Then: Returns `Err(Error::SessionLost)`

- **test_timeout_waiting_for_element_returns_error**
  Given: Element never appears
  When: Call `wait_for_element(client, "#never", Duration::from_millis(100))`
  Then: Returns `Err(Error::Timeout("#never".into(), 100))`

- **test_navigation_failed_returns_error**
  Given: Invalid URL
  When: Call `goto(client, "http://invalid..")`
  Then: Returns `Err(Error::NavigationFailed(...))`

- **test_javascript_error_returns_error**
  Given: JS throws error
  When: Call `execute_js(client, "throw new Error('oops')", [])`
  Then: Returns `Err(Error::JavaScriptError("oops".into()))`

- **test_invalid_selector_returns_error**
  Given: Empty selector
  When: Call `click_element(client, "")`
  Then: Returns `Err(Error::InvalidSelector(...))`

- **test_invalid_url_returns_error**
  Given: Malformed URL
  When: Call `goto(client, "not-a-url")`
  Then: Returns `Err(Error::InvalidUrl(...))`

---

## Edge Case Tests

- **test_handles_empty_text_content**
  Given: `<div id="empty"></div>`
  When: Call `get_text(client, "#empty")`
  Then: Returns empty string (not error)

- **test_handles_missing_attribute**
  Given: `<div id="noattr"></div>` (no class)
  When: Call `get_attribute(client, "#noattr", "class")`
  Then: Returns None

- **test_handles_zero_elements_count**
  Given: No elements matching `.nonexistent`
  When: Call `count_elements(client, ".nonexistent")`
  Then: Returns 0

- **test_handles_very_long_text**
  Given: Input with 10KB text
  When: Call `set_text(client, "#input", very_long_string)`
  Then: Sets full value without truncation

- **test_handles_unicode_text**
  Given: Input field
  When: Call `set_text(client, "#input", "Hello 世界 🌍")`
  Then: Unicode preserved correctly

- **test_handles_special_chars_in_selector**
  Given: Element with special chars in ID
  When: Call `click_element(client, "#id-with-dash")`
  Then: Selector parsed correctly

- **test_handles_viewport_minimum_size**
  When: Call `set_viewport(client, 320, 240)`
  Then: Returns Ok (minimum supported)

- **test_handles_negative_scroll_offset**
  When: Call `scroll_by(client, 0, -50)`
  Then: Scrolls up 50px (clamped to 0)

---

## Contract Verification Tests

### Precondition Verification

- **test_precondition_client_connected**
  Given: Client not connected
  When: Call any operation
  Then: Returns Error::ConnectionFailed or Error::SessionLost

- **test_precondition_selector_not_empty**
  Given: Empty string selector
  When: Call `click_element(client, "")`
  Then: Returns Error::InvalidSelector

- **test_precondition_url_valid_format**
  Given: Invalid URL string
  When: Call `goto(client, "ftp://invalid")`
  Then: Returns Error::InvalidUrl

- **test_precondition_element_exists**
  Given: Selector for non-existent element
  When: Call `click_element(client, "#nope")`
  Then: Returns Error::ElementNotFound

- **test_precondition_viewport_positive_dimensions**
  Given: Zero width
  When: Call `set_viewport(client, 0, 100)`
  Then: Returns Error::InvalidInput

- **test_precondition_timeout_positive**
  Given: Zero timeout
  When: Call `wait_for_element(client, "body", Duration::ZERO)`
  Then: Returns Error::InvalidInput

- **test_precondition_js_no_dangerous_patterns**
  Given: JS with eval()
  When: Call `execute_js(client, "eval('1+1')", [])`
  Then: Returns Error::InjectionBlocked

### Postcondition Verification

- **test_postcondition_navigation_changes_url**
  Given: At URL A
  When: Call `goto(client, "http://B")` then `get_url(client)`
  Then: Returns URL B (not A)

- **test_postcondition_click_fires_event**
  Given: Button with click counter
  When: Click the button twice
  Then: Counter is 2

- **test_postcondition_screenshot_creates_file**
  Given: Valid path
  When: Call `take_screenshot(client, path)`
  Then: File exists at path with PNG content

- **test_postcondition_client_remains_connected**
  Given: Connected client
  When: Execute any command
  Then: Client still connected (no SessionLost)

### Invariant Verification

- **test_invariant_connected_after_operations**
  Given: Initially connected
  When: Execute 10 different commands
  Then: All return Ok or expected errors, client still connected

- **test_inventor_url_valid_after_navigation**
  Given: Valid initial URL
  When: Navigate to another valid URL
  Then: get_url() always returns valid URL

- **test_invariant_element_valid_after_navigation**
  Given: Element found on page A
  When: Navigate to page B
  Then: Original element reference is invalid (need re-query)

---

## Contract Violation Tests

(One test per violation example in contract-spec.md)

- **test_violation_p2_empty_selector_returns_invalid_selector_error**
  Given: Empty string selector
  When: Call `click_element(&client, "")`
  Then: Returns `Err(Error::InvalidSelector("selector cannot be empty".into()))`

- **test_violation_p2_whitespace_only_selector_returns_invalid_selector_error**
  Given: Whitespace-only selector
  When: Call `click_element(&client, "   ")`
  Then: Returns `Err(Error::InvalidSelector("selector cannot be whitespace only".into()))`

- **test_violation_p3_invalid_url_returns_invalid_url_error**
  Given: Invalid URL string
  When: Call `client.goto("not-a-url")`
  Then: Returns `Err(Error::InvalidUrl("invalid uri".into()))`

- **test_violation_p4_nonexistent_element_returns_element_not_found_error**
  Given: Page without element
  When: Call `click_element(&client, "#nonexistent")`
  Then: Returns `Err(Error::ElementNotFound("#nonexistent".into()))`

- **test_violation_p5_zero_width_returns_invalid_input_error**
  Given: Zero width dimension
  When: Call `set_viewport(&client, 0, 800)`
  Then: Returns `Err(Error::InvalidInput("width must be > 0".into()))`

- **test_violation_p6_zero_timeout_returns_invalid_input_error**
  Given: Zero duration
  When: Call `wait_for_element(&client, "body", Duration::from_millis(0))`
  Then: Returns `Err(Error::InvalidInput("timeout must be > 0".into()))`

- **test_violation_p7_dangerous_js_returns_injection_blocked_error**
  Given: JavaScript with eval
  When: Call `execute_js(&client, "eval(document.cookie)", [])`
  Then: Returns `Err(Error::InjectionBlocked)`

- **test_violation_p8_invalid_cookie_name_returns_cookie_error**
  Given: Cookie name with null byte
  When: Call `set_cookie(&client, "bad\x00name", "value", None, None)`
  Then: Returns `Err(Error::CookieError("invalid name".into()))`

- **test_violation_q1_navigation_updates_url**
  Given: At "http://example.com"
  When: Call `goto("http://other.com")` then `get_url()`
  Then: Returns "http://other.com" (not original)

- **test_violation_q6_screenshot_creates_valid_file**
  Given: Valid path
  When: Call `screenshot("path.png")`
  Then: File exists with PNG header (bytes[0:8] == 0x89 0x50 0x4E 0x47...)

---

## End-to-End Scenario

### Scenario: Complete User Flow

**Given**: ChromeDriver running, empty browser profile
**When**: 
1. Connect client to ChromeDriver
2. Navigate to "http://example.com"
3. Find login form elements
4. Enter username "testuser"
5. Enter password "secret123"
6. Click submit button
7. Wait for dashboard to load
8. Verify URL contains "/dashboard"
9. Get page title
10. Take screenshot of dashboard
11. Get cookies
12. Close browser

**Then**:
- All operations return Ok
- Final URL is "/dashboard"
- Screenshot file exists
- At least one cookie present
- Client cleanly disconnected
