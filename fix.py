with open('src/actions.rs', 'r') as f:
    c = f.read()

c = c.replace('#[allow(clippy::unwrap_used)]\n        println!("{}", serde_json::to_string(&output).unwrap());',
              'println!("{}", serde_json::to_string(&output).unwrap_or_else(|_| "{\"success\":false,\"command\":\"unknown\",\"data\":null,\"error\":\"Failed to serialize JSON output\",\"logs\":[]}".to_string()));')

with open('src/actions.rs', 'w') as f:
    f.write(c)

