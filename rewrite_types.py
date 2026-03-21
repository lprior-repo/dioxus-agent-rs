import re

# We need to rewrite data.rs completely to use proper Newtypes for validation
with open('src/data.rs', 'r') as f:
    content = f.read()

# Let's modify the Config and Commands manually.
