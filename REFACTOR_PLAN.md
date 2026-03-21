# Should we leverage both?

While `chromiumoxide` gives us God-Tier visibility, `fantoccini` (WebDriver) provides significantly more stable basic interactions across all browsers. 

Could we build a hybrid? Yes. We could use a **Dual-Driver Architecture**.

1. **W3C WebDriver (`fantoccini`)** as the primary driver for `click`, `text`, `submit`, and cross-browser testing.
2. **CDP WebSocket (`chromiumoxide`)** attached to the exact same Chrome session running on port 4444. WebDriver handles the W3C spec interactions, while CDP handles the underlying network interception, file uploads, and specific element screenshots.

This is exactly how Playwright and Puppeteer operate under the hood—they maintain a standard connection for interactions and a CDP WebSocket connection for engine manipulation.

If you want to build this hybrid, we would:
1. Re-add `fantoccini` to `Cargo.toml`.
2. Connect `fantoccini::Client` to `http://localhost:4444`.
3. Extract the Chrome WebSocket debugger URL from the `fantoccini` session.
4. Pass that WebSocket URL into `chromiumoxide::browser::Browser::connect()`.
5. Pass *both* `&mut Client` and `&Page` to the `dispatch_command` loop.
