with open('src/calculations.rs', 'r') as f:
    c = f.read()

c = c.replace('        Commands::Upload { selector, path } => {\n            validate_selector(selector)?;\n            validate_path(path)\n        }\n        Commands::Upload { selector, path } => {\n            validate_selector(selector)?;\n            validate_path(path)\n        }',
              '        Commands::Upload { selector, path } => {\n            validate_selector(selector)?;\n            validate_path(path)\n        }')

with open('src/calculations.rs', 'w') as f:
    f.write(c)

