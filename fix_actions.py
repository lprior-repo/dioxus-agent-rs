import re

with open('src/actions.rs', 'r') as f:
    content = f.read()

# I see a block starting with `    // Launch browser` outside of any function, right after `dispatch_dual`.
# Let's find dispatch_dual and delete everything after it until `async fn run_repl`.

match_start = content.find("async fn dispatch_dual")
if match_start != -1:
    end_dispatch = content.find("}\n", content.find("dispatch_command(page, command).await,\n    }\n}")) + 2
    # Find async fn run_repl
    run_repl_start = content.find("async fn run_repl(page: &Page) -> Result<()> {")
    
    if end_dispatch != -1 and run_repl_start != -1 and end_dispatch < run_repl_start:
        content = content[:end_dispatch] + "\n" + content[run_repl_start:]
        
with open('src/actions.rs', 'w') as f:
    f.write(content)
