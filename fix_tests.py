with open('tests/calculations.rs', 'r') as f:
    c = f.read()

c = c.replace('result.unwrap_err().contains("URL")', 'result.unwrap_err().to_string().contains("URL")')
c = c.replace('result.unwrap_err().contains("timeout")', 'result.unwrap_err().to_string().contains("timeout")')
c = c.replace('result.unwrap_err().contains("selector")', 'result.unwrap_err().to_string().contains("selector")')
c = c.replace('result.unwrap_err().contains("width")', 'result.unwrap_err().to_string().contains("width")')
c = c.replace('result.unwrap_err().contains("dangerous")', 'result.unwrap_err().to_string().contains("dangerous")')
c = c.replace('result.unwrap_err().contains("console type")', 'result.unwrap_err().to_string().contains("console type")')

with open('tests/calculations.rs', 'w') as f:
    f.write(c)
