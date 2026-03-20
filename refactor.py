import re

with open('src/actions.rs', 'r') as f:
    content = f.read()

# Change signature
content = content.replace(
    "async fn execute_command_internal(client: &mut fantoccini::Client, command: &Commands) -> Result<()> {",
    "async fn execute_command_internal(client: &mut fantoccini::Client, command: &Commands) -> Result<Value> {"
)

# Repl command at bottom
content = content.replace(
    "Commands::Repl => {\n            // Handled externally\n        }",
    "Commands::Repl => {\n            // Handled externally\n            Ok(serde_json::Value::Null)\n        }"
)

# At the end of execute_command_internal
content = content.replace(
    "    }\n\n    Ok(())\n}",
    "    }\n}"
)

# Replace simple println! with Ok(json!())
def replace_println(match):
    inner = match.group(1)
    if inner.startswith('"') and inner.endswith('"'):
        # String literal
        return f"Ok(serde_json::json!({inner}))"
    elif "{" in inner and "}" in inner:
        # Interpolated string like "{selector} {value}"
        # We'll just wrap it in format!
        return f"Ok(serde_json::json!(format!({inner})))"
    else:
        # Just a variable
        return f"Ok(serde_json::json!({inner}))"

# We only want to replace println! inside the match command block.
# Let's just do a regex replace for println!(...)
# Wait, some println! are empty: println!()
content = re.sub(r'println!\(\);', r'Ok(serde_json::Value::Null)', content)

# Replace println!("{var}") and similar
# We need to make sure we don't hit run_repl printlns.
# Let's just replace them and manually fix run_repl.
content = re.sub(r'println!\((.*?)\);', replace_println, content)

# Fix run_repl manually since we replaced its printlns
content = content.replace(
    'Ok(serde_json::json!(format!("Dioxus Agent REPL connected to {current_url}")))',
    'println!("Dioxus Agent REPL connected to {current_url}");'
)
content = content.replace(
    'Ok(serde_json::json!("Type \'help\' for commands, \'exit\' to quit."))',
    'println!("Type \'help\' for commands, \'exit\' to quit.");'
)
content = content.replace('Ok(serde_json::json!("Already in REPL mode."))', 'println!("Already in REPL mode.");')
content = content.replace('Ok(serde_json::json!(format!("Error: {e}")))', 'println!("Error: {e}");')
content = content.replace('Ok(serde_json::json!("{e}"))', 'println!("{e}");')
content = content.replace('Ok(serde_json::json!("CTRL-C"))', 'println!("CTRL-C");')
content = content.replace('Ok(serde_json::json!("CTRL-D"))', 'println!("CTRL-D");')
content = content.replace('Ok(serde_json::json!(format!("Error: {:?}", err)))', 'println!("Error: {:?}", err);')

with open('src/actions.rs', 'w') as f:
    f.write(content)
