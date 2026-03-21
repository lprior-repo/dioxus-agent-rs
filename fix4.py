with open('src/main.rs', 'r') as f:
    c = f.read()

c = c.replace('println!("{}", serde_json::to_string(&output).unwrap());', 
              'println!("{}", serde_json::to_string(&output).unwrap_or_else(|_| r#"{"success":false,"command":"unknown","data":null,"error":"Failed to serialize output","logs":[]}"#.to_string()));')

with open('src/main.rs', 'w') as f:
    f.write(c)

