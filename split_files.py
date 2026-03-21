import os
import re

os.makedirs('src/data', exist_ok=True)
os.makedirs('src/calculations', exist_ok=True)
os.makedirs('src/actions', exist_ok=True)

# 1. SPLIT DATA.RS
with open('src/data.rs', 'r') as f:
    data_content = f.read()

types_start = data_content.find("pub mod types;")
commands_start = data_content.find("pub enum Commands {")
config_start = data_content.find("pub struct Config {")
output_start = data_content.find("pub struct CommandOutput {")

# Just move `data.rs` to `src/data/mod.rs` and then split it out
os.rename('src/data.rs', 'src/data/mod.rs')

# Actually, doing this via Python parsing is very brittle.
# I'll create the `mod.rs` files directly with `pub mod` declarations.
