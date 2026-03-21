with open('src/actions.rs', 'r') as f:
    c = f.read()

c = c.replace('unwrap_or_else(|_| "{"success":false,"command":"unknown","data":null,"error":"Failed to serialize JSON output","logs":[]}".to_string())',
              'unwrap_or_else(|_| r#"{"success":false,"command":"unknown","data":null,"error":"Failed to serialize JSON output","logs":[]}"#.to_string())')

with open('src/actions.rs', 'w') as f:
    f.write(c)

with open('src/calculations.rs', 'r') as f:
    c = f.read()

c = c.replace('r#"const style = document.createElement(\'style\'); style.textContent = \'{}\'; document.head.appendChild(style); return true;"#',
              'r"const style = document.createElement(\'style\'); style.textContent = \'{}\'; document.head.appendChild(style); return true;"')
c = c.replace('r#"const el = document.querySelector(\'{}\'); if (!el) return null; const rect = el.getBoundingClientRect(); return {{ x: rect.x, y: rect.y, width: rect.width, height: rect.height }};"#',
              'r"const el = document.querySelector(\'{}\'); if (!el) return null; const rect = el.getBoundingClientRect(); return {{ x: rect.x, y: rect.y, width: rect.width, height: rect.height }};"')
c = c.replace('r#"if (typeof window.getDioxusState === \'function\') { return window.getDioxusState(); }',
              'r"if (typeof window.getDioxusState === \'function\') { return window.getDioxusState(); }')
c = c.replace('return states.length > 0 ? states : null;"#.into()',
              'return states.length > 0 ? states : null;".into()')
c = c.replace('r#"const iter = document.createNodeIterator(document.body, NodeFilter.SHOW_ELEMENT, {',
              'r"const iter = document.createNodeIterator(document.body, NodeFilter.SHOW_ELEMENT, {')
c = c.replace('return true;"#.into()',
              'return true;".into()')
c = c.replace('r#"const el = document.querySelector(\'{}\'); if (!el) return null; return window.getComputedStyle(el).getPropertyValue(\'{}\');"#',
              'r"const el = document.querySelector(\'{}\'); if (!el) return null; return window.getComputedStyle(el).getPropertyValue(\'{}\');"')
c = c.replace('r#"return new Promise((resolve) => {{', 'r"return new Promise((resolve) => {{')
c = c.replace('}});"#,', '}});",')
c = c.replace('r#"const targetText = \'{escaped}\';', 'r"const targetText = \'{escaped}\';')
c = c.replace('return null;"#', 'return null;"')
c = c.replace('r#"return new Promise((resolve) => {', 'r"return new Promise((resolve) => {')
c = c.replace('});"#.into()', '});".into()')
c = c.replace('r#"return new Promise(async (resolve) => {{', 'r"return new Promise(async (resolve) => {{')
c = c.replace('r#"const table = document.querySelector(\'{}\');', 'r"const table = document.querySelector(\'{}\');')

with open('src/calculations.rs', 'w') as f:
    f.write(c)

