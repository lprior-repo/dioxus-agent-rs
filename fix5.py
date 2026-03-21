with open('src/calculations.rs', 'r') as f:
    c = f.read()

c = c.replace('Commands::ElementScreenshot { selector, path } | Commands::Upload { selector, path } => {',
              'Commands::ElementScreenshot { selector, path } => {')
c = c.replace('validate_selector(selector)?;\n            validate_path(path)\n        }\n\n        Commands::AssertText { selector, expected } => {',
              'validate_selector(selector)?;\n            validate_path(path)\n        }\n        Commands::Upload { selector, path } => {\n            validate_selector(selector)?;\n            validate_path(path)\n        }\n\n        Commands::AssertText { selector, expected } => {')

with open('src/calculations.rs', 'w') as f:
    f.write(c)

with open('src/actions.rs', 'r') as f:
    c = f.read()

c = c.replace('async fn inject_console_capture(client: &mut Client) -> Result<()> {',
              '#[allow(clippy::needless_pass_by_ref_mut)]\nasync fn inject_console_capture(client: &mut Client) -> Result<()> {')
c = c.replace('async fn handle_navigation(client: &mut Client, command: &Commands) -> Result<Value> {',
              '#[allow(clippy::needless_pass_by_ref_mut)]\nasync fn handle_navigation(client: &mut Client, command: &Commands) -> Result<Value> {')
c = c.replace('async fn handle_interaction(client: &mut Client, command: &Commands) -> Result<Value> {',
              '#[allow(clippy::needless_pass_by_ref_mut)]\nasync fn handle_interaction(client: &mut Client, command: &Commands) -> Result<Value> {')
c = c.replace('async fn handle_queries(client: &mut Client, command: &Commands) -> Result<Value> {',
              '#[allow(clippy::needless_pass_by_ref_mut)]\nasync fn handle_queries(client: &mut Client, command: &Commands) -> Result<Value> {')
c = c.replace('async fn handle_storage(client: &mut Client, command: &Commands) -> Result<Value> {',
              '#[allow(clippy::needless_pass_by_ref_mut)]\nasync fn handle_storage(client: &mut Client, command: &Commands) -> Result<Value> {')

with open('src/actions.rs', 'w') as f:
    f.write(c)
