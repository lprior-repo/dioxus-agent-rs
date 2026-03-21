with open('src/calculations.rs', 'r') as f:
    c = f.read()

c = c.replace('r"return new Promise((resolve) => {\n        const checkReady = () => {\n            const hasSuspense = document.querySelector(\'[aria-busy="true"]\');', 
              'r#"return new Promise((resolve) => {\n        const checkReady = () => {\n            const hasSuspense = document.querySelector(\'[aria-busy="true"]\');')
c = c.replace('setTimeout(() => { observer.disconnect(); resolve(true); }, 10000);\n    });".into()',
              'setTimeout(() => { observer.disconnect(); resolve(true); }, 10000);\n    });"#.into()')

with open('src/calculations.rs', 'w') as f:
    f.write(c)

with open('src/actions.rs', 'r') as f:
    c = f.read()

c = c.replace('let js = r#"\\n        window.__captured_logs', 'let js = r"\\n        window.__captured_logs')
c = c.replace('return originalXhrSend.apply(this, args);\\n        };\\n    "#;', 'return originalXhrSend.apply(this, args);\\n        };\\n    ";')

with open('src/actions.rs', 'w') as f:
    f.write(c)

