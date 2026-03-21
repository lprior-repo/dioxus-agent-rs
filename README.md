# dioxus-agent-rs

A God-Tier Rust automation CLI perfectly optimized for AI Agents driving Dioxus and WebAssembly front-ends.

## The Philosophical Divide: WebDriver (`fantoccini`) vs CDP (`chromiumoxide`)

This tool has been built using both architectures. Here is the bitter truth about both:

### `fantoccini` (W3C WebDriver)
**The Good:**
- Standardized across Chrome, Firefox, Safari, and Edge.
- Extremely stable, predictable, and mature API surface.
- Native `.click()`, `.send_keys()`, and `.clear()` methods work flawlessly 99% of the time.

**The Bad:**
- Requires a separate binary (`chromedriver`, `geckodriver`) running on a specific port.
- No low-level control. File uploads (`<input type="file">`) are notoriously broken or finicky.
- Lacks real network interception (Mocking fetch/XHR requires massive JS injection hacks).
- Cannot take screenshots of specific elements natively without external image cropping.

### `chromiumoxide` (Chrome DevTools Protocol - CDP)
**The Good:**
- **Zero-setup execution.** It natively launches Chrome itself. No `chromedriver` background process required.
- **God-Tier Capabilities.** It talks directly to the V8 engine. You can mock network routes natively, override device metrics (true mobile simulation), and intercept low-level DOM events.
- **Native Element Screenshots & File Uploads.** You can tell the browser to screenshot a specific node ID, or synthesize an OS-level file drag-and-drop.

**The Bad:**
- **Chrome Only.** You sacrifice Firefox and Safari entirely.
- **Brittle API Surface.** The `chromiumoxide::cdp::browser_protocol` modules are auto-generated from Chrome's internal spec. It requires monstrous builder patterns just to take a screenshot.
- **Flaky Basic Interactions.** Ironically, standard things like `element.type_str("hello")` or `element.click()` can sometimes be less reliable than WebDriver because CDP operates at a lower level than the W3C spec meant for standard user emulation.

### The Verdict: Which should we use?

For an **AI Agent driving a web UI**, the answer is decisively **CDP (`chromiumoxide`)**. 

AI Agents are bottlenecked by setup friction and visibility. Forcing an AI to figure out how to start a background `chromedriver` on port 4444 before it can even run a test is an immediate failure point. 

Furthermore, AI Agents need "God-Mode" features to survive:
1. They need to extract the DOM cleanly.
2. They need to mock network requests so they don't hallucinate API errors.
3. They need to inject files directly into memory.

`chromiumoxide` provides this natively.

## Current State

The current implementation in the `master` branch is entirely built on **`chromiumoxide`**. It is structured using strict **Functional Rust** (Data -> Calc -> Actions) and enforces **Zero Unwraps / Panics** at the boundary.
