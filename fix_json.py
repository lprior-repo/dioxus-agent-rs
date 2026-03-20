import re

with open('src/actions.rs', 'r') as f:
    content = f.read()

# Fix literal string insertions
replacements = {
    'Ok(serde_json::json!("{source}"))': 'Ok(serde_json::json!(source))',
    'Ok(serde_json::json!("{title}"))': 'Ok(serde_json::json!(title))',
    'Ok(serde_json::json!("{url}"))': 'Ok(serde_json::json!(url))',
    'Ok(serde_json::json!("{selector}"))': 'Ok(serde_json::json!(selector))',
    'Ok(serde_json::json!("{selector} {value}"))': 'Ok(serde_json::json!(format!("{selector} {value}")))',
    'Ok(serde_json::json!("{text}"))': 'Ok(serde_json::json!(text))',
    'Ok(serde_json::json!("{v}"))': 'Ok(serde_json::json!(v))',
    'Ok(serde_json::json!("{name}"))': 'Ok(serde_json::json!(name))',
    'Ok(serde_json::json!("{b}"))': 'Ok(serde_json::json!(b))',
    'Ok(serde_json::json!("{count}"))': 'Ok(serde_json::json!(count))',
    'Ok(serde_json::json!("{exists}"))': 'Ok(serde_json::json!(exists))',
    'Ok(serde_json::json!("{result}"))': 'Ok(serde_json::json!(result))',
    'Ok(serde_json::json!("{path}"))': 'Ok(serde_json::json!(path))',
    'Ok(serde_json::json!("{width} {height}"))': 'Ok(serde_json::json!(format!("{width} {height}")))',
    'Ok(serde_json::json!("{x} {y}"))': 'Ok(serde_json::json!(format!("{x} {y}")))',
    'Ok(serde_json::json!("{key}"))': 'Ok(serde_json::json!(key))',
    'Ok(serde_json::json!("{combo}"))': 'Ok(serde_json::json!(combo))',
    'Ok(serde_json::json!("{target}"))': 'Ok(serde_json::json!(target))',
    'Ok(serde_json::json!("{entry}"))': 'Ok(serde_json::json!(entry))'
}

for old, new in replacements.items():
    content = content.replace(old, new)

with open('src/actions.rs', 'w') as f:
    f.write(content)
