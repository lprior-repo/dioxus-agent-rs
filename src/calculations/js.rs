
#[must_use]
pub fn escape_js_string(s: &str) -> String {
    s.replace('\\', r"\\")
        .replace('\'', r"\'")
        .replace('"', r#"\""#)
        .replace('\n', r"\n")
        .replace('\r', r"\r")
        .replace('\t', r"\t")
}

#[must_use]
pub fn generate_keypress_js(key: &str) -> String {
    let key_lower = key.to_lowercase();
    match key_lower.as_str() {
        "enter" => "return { key: 'Enter' }".into(),
        "escape" | "esc" => "return { key: 'Escape' }".into(),
        "tab" => "return { key: 'Tab' }".into(),
        "backspace" => "return { key: 'Backspace' }".into(),
        "delete" | "del" => "return { key: 'Delete' }".into(),
        "arrowup" | "up" => "return { key: 'ArrowUp' }".into(),
        "arrowdown" | "down" => "return { key: 'ArrowDown' }".into(),
        "arrowleft" | "left" => "return { key: 'ArrowLeft' }".into(),
        "arrowright" | "right" => "return { key: 'ArrowRight' }".into(),
        "home" => "return { key: 'Home' }".into(),
        "end" => "return { key: 'End' }".into(),
        "pageup" => "return { key: 'PageUp' }".into(),
        "pagedown" => "return { key: 'PageDown' }".into(),
        _ => format!("return {{ key: '{}' }}", escape_js_string(key)),
    }
}

pub fn generate_keycombo_js(combo: &str) -> String {
    let parts: Vec<String> = combo
        .split('+')
        .map(str::trim)
        .map(std::string::ToString::to_string)
        .collect();
    let mut modifiers = Vec::new();
    let mut key: Option<String> = None;

    for part in parts {
        match part.to_lowercase().as_str() {
            "control" | "ctrl" => modifiers.push("ctrlKey"),
            "shift" => modifiers.push("shiftKey"),
            "alt" => modifiers.push("altKey"),
            "meta" | "cmd" | "command" => modifiers.push("metaKey"),
            _ => key = Some(part.clone()),
        }
    }

    let k = escape_js_string(&key.unwrap_or_default());
    if modifiers.is_empty() {
        format!("return {{ key: '{k}', ctrlKey: false, shiftKey: false, altKey: false, metaKey: false }}")
    } else {
        format!("return {{ key: '{k}', {} }}", modifiers.join(", "))
    }
}

#[must_use]
pub fn generate_storage_js(
    storage: &str,
    op: &str,
    key: Option<&str>,
    value: Option<&str>,
) -> String {
    match (storage, op, key, value) {
        ("local", "get", Some(k), _) => {
            format!("return localStorage.getItem('{}');", escape_js_string(k))
        }
        ("local", "set", Some(k), Some(v)) => format!(
            "localStorage.setItem('{}', '{}'); return true;",
            escape_js_string(k),
            escape_js_string(v)
        ),
        ("local", "remove", Some(k), _) => format!(
            "localStorage.removeItem('{}'); return true;",
            escape_js_string(k)
        ),
        ("local", "clear", None, _) => "localStorage.clear(); return true;".into(),
        ("session", "get", Some(k), _) => {
            format!("return sessionStorage.getItem('{}');", escape_js_string(k))
        }
        ("session", "set", Some(k), Some(v)) => format!(
            "sessionStorage.setItem('{}', '{}'); return true;",
            escape_js_string(k),
            escape_js_string(v)
        ),
        ("session", "clear", None, _) => "sessionStorage.clear(); return true;".into(),
        _ => "return null;".into(),
    }
}

#[must_use]
pub fn generate_css_injection_js(css: &str) -> String {
    format!(
        r"const style = document.createElement('style'); style.textContent = '{}'; document.head.appendChild(style); return true;",
        escape_js_string(css)
    )
}

#[must_use]
pub fn generate_dioxus_click_js(target: &str) -> String {
    format!(
        r#"const el = document.querySelector('[data-target="{}"]'); if (el) {{ el.click(); return true; }} return false;"#,
        escape_js_string(target)
    )
}

#[must_use]
pub fn generate_dioxus_state_js() -> String {
    r"if (typeof window.getDioxusState === 'function') { return window.getDioxusState(); }
     if (typeof window.__DX_STATE__ !== 'undefined') { return window.__DX_STATE__; }
     const states = [];
     for (let key in window) {
         if (key.startsWith('__dx') || key.startsWith('__dioxus')) {
             try { states.push({ [key]: window[key] }); } catch(e) {}
         }
     }
     return states.length > 0 ? states : null;"
        .into()
}

#[must_use]
pub fn generate_hydration_wait_js() -> String {
    r#"return new Promise((resolve) => {
        const checkReady = () => {
            const hasSuspense = document.querySelector('[aria-busy="true"]');
            const hasHydrated = document.body.hasAttribute('data-hydrated') || document.querySelector('#main') || document.body.innerHTML.length > 50;
            if (hasHydrated && !hasSuspense) { resolve(true); return true; }
            return false;
        };
        if (checkReady()) return;
        const observer = new MutationObserver(() => {
            if (checkReady()) { observer.disconnect(); resolve(true); }
        });
        observer.observe(document.body, { childList: true, subtree: true, attributes: true });
        setTimeout(() => { observer.disconnect(); resolve(true); }, 10000);
    });"#.into()
}

#[must_use]
pub fn generate_semantic_tree_js() -> String {
    r#"function getSemanticTree(root) {
        const tree = [];
        const iter = document.createNodeIterator(root, NodeFilter.SHOW_ELEMENT, {
            acceptNode: (node) => {
                const tag = node.tagName.toLowerCase();
                const interactable = ['a', 'button', 'input', 'select', 'textarea'].includes(tag) || node.hasAttribute('role') || node.hasAttribute('tabindex');
                if (!interactable) return NodeFilter.FILTER_SKIP;
                const style = window.getComputedStyle(node);
                if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return NodeFilter.FILTER_REJECT;
                return NodeFilter.FILTER_ACCEPT;
            }
        });
        let node;
        while (node = iter.nextNode()) {
            const tag = node.tagName.toLowerCase();
            let label = node.innerText || node.value || node.getAttribute('aria-label') || node.getAttribute('title') || '';
            label = label.substring(0, 50).replace(/\n/g, ' ').trim();
            let id = node.id ? '#' + node.id : '';
            let cls = node.className ? '.' + node.className.split(' ').join('.') : '';
            let sel = id ? id : (cls ? tag + cls : tag);
            tree.push(`[${tag.toUpperCase()}] ${sel} "${label}"`);
        }
        return tree.join('\n');
    }
    return getSemanticTree(document.body);"#.into()
}

#[must_use]
pub fn generate_screenshot_annotated_js() -> String {
    r"const iter = document.createNodeIterator(document.body, NodeFilter.SHOW_ELEMENT, {
        acceptNode: (node) => {
            const tag = node.tagName.toLowerCase();
            const interactable = ['a', 'button', 'input', 'select', 'textarea'].includes(tag) || node.hasAttribute('role') || node.hasAttribute('tabindex');
            if (!interactable) return NodeFilter.FILTER_SKIP;
            const style = window.getComputedStyle(node);
            if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return NodeFilter.FILTER_REJECT;
            return NodeFilter.FILTER_ACCEPT;
        }
    });
    let node; let counter = 1;
    while (node = iter.nextNode()) {
        const rect = node.getBoundingClientRect();
        if (rect.width === 0 || rect.height === 0) continue;
        const overlay = document.createElement('div');
        overlay.style.position = 'absolute';
        overlay.style.left = `${rect.left + window.scrollX}px`;
        overlay.style.top = `${rect.top + window.scrollY}px`;
        overlay.style.width = `${rect.width}px`;
        overlay.style.height = `${rect.height}px`;
        overlay.style.border = '2px solid red';
        overlay.style.pointerEvents = 'none';
        overlay.style.zIndex = '999999';
        const label = document.createElement('span');
        label.style.position = 'absolute';
        label.style.background = 'red';
        label.style.color = 'white';
        label.style.fontSize = '12px';
        label.style.top = '-14px';
        label.style.left = '-2px';
        label.style.padding = '0 2px';
        label.innerText = counter++;
        overlay.appendChild(label);
        document.body.appendChild(overlay);
    }
    return true;".into()
}

#[must_use]
pub fn generate_computed_style_js(selector: &str, property: &str) -> String {
    format!(
        r"const el = document.querySelector('{}'); if (!el) return null; return window.getComputedStyle(el).getPropertyValue('{}');",
        escape_js_string(selector),
        escape_js_string(property)
    )
}

#[must_use]
pub fn generate_wait_element_js(selector: &str) -> String {
    let escaped = escape_js_string(selector);
    format!(
        r"return new Promise((resolve) => {{
            const el = document.querySelector('{escaped}');
            if (el) {{ resolve(el); return; }}
            const observer = new MutationObserver(() => {{
                const el = document.querySelector('{escaped}');
                if (el) {{ observer.disconnect(); resolve(el); }}
            }});
            observer.observe(document.body, {{ childList: true, subtree: true }});
        }});",
    )
}

#[must_use]
pub fn generate_wait_stable_js(selector: &str) -> String {
    let escaped = escape_js_string(selector);
    format!(
        r"return new Promise((resolve) => {{
            const checkStable = async () => {{
                const el = document.querySelector('{escaped}');
                if (!el) return false;
                const style = window.getComputedStyle(el);
                if (style.display === 'none' || style.visibility === 'hidden') return false;
                const rect1 = el.getBoundingClientRect();
                await new Promise(r => requestAnimationFrame(r));
                await new Promise(r => requestAnimationFrame(r));
                const rect2 = el.getBoundingClientRect();
                return rect1.x === rect2.x && rect1.y === rect2.y && rect1.width === rect2.width && rect1.height === rect2.height;
            }};
            const run = async () => {{
                for (let i = 0; i < 50; i++) {{
                    if (await checkStable()) {{ resolve(true); return; }}
                    await new Promise(r => setTimeout(r, 100));
                }}
                resolve(false);
            }};
            run();
        }});"
    )
}

#[must_use]
pub fn generate_wait_gone_js(selector: &str) -> String {
    let escaped = escape_js_string(selector);
    format!(
        r"return new Promise((resolve) => {{
            const el = document.querySelector('{escaped}');
            if (!el) {{ resolve(true); return; }}
            const observer = new MutationObserver(() => {{
                const el = document.querySelector('{escaped}');
                if (!el) {{ observer.disconnect(); resolve(true); }}
            }});
            observer.observe(document.body, {{ childList: true, subtree: true }});
        }});",
    )
}

#[must_use]
pub fn generate_console_js(console_type: Option<&str>) -> String {
    console_type.map_or_else(
        || "return window.__captured_logs || [];".into(),
        |t| format!("return window.__captured_{} || [];", escape_js_string(t)),
    )
}

#[must_use]
pub fn generate_fuzzy_click_js(text: &str) -> String {
    let escaped = escape_js_string(&text.to_lowercase());
    format!(
        r"const targetText = '{escaped}';
        const iter = document.createNodeIterator(document.body, NodeFilter.SHOW_ELEMENT, {{
            acceptNode: (node) => {{
                const tag = node.tagName.toLowerCase();
                const interactable = ['a', 'button', 'input', 'select', 'textarea'].includes(tag) || node.hasAttribute('role') || node.hasAttribute('tabindex');
                if (!interactable) return NodeFilter.FILTER_SKIP;
                const style = window.getComputedStyle(node);
                if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return NodeFilter.FILTER_REJECT;
                return NodeFilter.FILTER_ACCEPT;
            }}
        }});
        let node;
        let bestMatch = null;
        let exactMatch = false;
        while (node = iter.nextNode()) {{
            const nodeText = (node.innerText || node.value || node.getAttribute('aria-label') || node.getAttribute('title') || '').toLowerCase();
            if (nodeText === targetText) {{
                bestMatch = node;
                exactMatch = true;
                break;
            }} else if (!exactMatch && nodeText.includes(targetText)) {{
                bestMatch = node;
            }}
        }}
        if (bestMatch) {{
            bestMatch.click();
            const id = bestMatch.id ? '#' + bestMatch.id : '';
            const cls = bestMatch.className ? '.' + bestMatch.className.split(' ').join('.') : '';
            return id ? id : (cls ? bestMatch.tagName.toLowerCase() + cls : bestMatch.tagName.toLowerCase());
        }}
        return null;"
    )
}

#[must_use]
pub fn generate_network_idle_js() -> String {
    r"return new Promise((resolve) => {
        let idleCycles = 0;
        const check = setInterval(() => {
            if (window.__active_requests === 0 || typeof window.__active_requests === 'undefined') {
                idleCycles++;
                if (idleCycles >= 5) {
                    clearInterval(check);
                    resolve(true);
                }
            } else {
                idleCycles = 0;
            }
        }, 100);
        setTimeout(() => { clearInterval(check); resolve(false); }, 15000);
    });"
    .into()
}

#[must_use]
pub fn generate_scroll_to_text_js(container: &str, text: &str) -> String {
    format!(
        r"return new Promise(async (resolve) => {{
            const container = document.querySelector('{}');
            if (!container) {{ resolve(null); return; }}
            const targetText = '{}'.toLowerCase();
            let attempts = 0;
            while (attempts < 20) {{
                const found = Array.from(container.querySelectorAll('*')).some(el =>
                    el.children.length === 0 && el.textContent.toLowerCase().includes(targetText)
                );
                if (found) {{ resolve(true); return; }}
                const oldScroll = container.scrollTop;
                container.scrollTop += container.clientHeight;
                if (container.scrollTop === oldScroll) {{ resolve(false); return; }}
                await new Promise(r => setTimeout(r, 200));
                attempts++;
            }}
            resolve(false);
        }});",
        escape_js_string(container),
        escape_js_string(text)
    )
}

#[must_use]
pub fn generate_extract_table_js(selector: &str) -> String {
    format!(
        r"const table = document.querySelector('{}');
        if (!table) return null;
        const headers = Array.from(table.querySelectorAll('th')).map(th => th.innerText.trim());
        if (headers.length === 0) return null;
        const rows = Array.from(table.querySelectorAll('tbody tr, tr:not(:first-child)'));
        return rows.map(row => {{
            const cells = Array.from(row.querySelectorAll('td'));
            const obj = {{}};
            headers.forEach((h, i) => {{
                obj[h] = cells[i] ? cells[i].innerText.trim() : '';
            }});
            return obj;
        }});",
        escape_js_string(selector)
    )
}
