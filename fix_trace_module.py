# First let's address the trace logic in actions.rs
import re
with open('src/actions.rs', 'r') as f:
    content = f.read()

# I need to completely extract the trace logic from execute_command.
