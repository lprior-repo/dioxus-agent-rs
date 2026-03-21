with open('tests/calculations.rs', 'r') as f:
    c = f.read()

c = c.replace('result.unwrap_err().to_string().contains("timeout")', 'result.unwrap_err().to_string().to_lowercase().contains("timeout")')

with open('tests/calculations.rs', 'w') as f:
    f.write(c)
