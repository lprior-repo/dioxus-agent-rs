import re

with open('tests/calculations.rs', 'r') as f:
    content = f.read()

# The test-reviewer explicitly told me to delete `test_validate_all_commands_accept_valid_inputs`
# It's a phantom test.

# Remove the mega array and test
content = re.sub(r'#\[test\]\nfn test_validate_all_commands_accept_valid_inputs.*?Ok\(\(\)\);\n}', '', content, flags=re.DOTALL)

with open('tests/calculations.rs', 'w') as f:
    f.write(content)
