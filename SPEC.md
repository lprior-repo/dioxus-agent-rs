# Dioxus Agent RS - Full Specification

## Project Overview
- **Project Name**: dioxus-agent-rs
- **Type**: Pure Rust WebDriver CLI for browser automation
- **Core Functionality**: A command-line tool that connects to ChromeDriver to automate browser interactions, designed for testing Dioxus applications and general browser automation.
- **Target Users**: Developers testing Dioxus apps, AI agents performing browser automation

## Architecture
- **Pattern**: Data → Calculations → Actions (functional Rust)
- **Dependencies**: fantoccini (WebDriver), clap (CLI), tokio (async)
- **Output**: Single binary CLI tool

## Commands Specification

### Navigation Commands
| Command | Args | Description |
|---------|------|-------------|
| `dom` | none | Get full HTML of page |
| `title` | none | Get page title |
| `url` | none | Get current URL |
| `refresh` | none | Refresh the page |
| `back` | none | Go back in history |
| `forward` | none | Go forward in history |

### Element Interaction
| Command | Args | Description |
|---------|------|-------------|
| `click` | `selector` | Click element by CSS |
| `double-click` | `selector` | Double-click element |
| `right-click` | `selector` | Right-click (context menu) |
| `hover` | `selector` | Hover over element |
| `text` | `selector`, `value` | Set input value |
| `clear` | `selector` | Clear input field |
| `submit` | `selector` | Submit form |
| `select` | `selector`, `value` | Select dropdown option |
| `check` | `selector` | Check checkbox/radio |
| `uncheck` | `selector` | Uncheck checkbox |

### Element Queries
| Command | Args | Description |
|---------|------|-------------|
| `get-text` | `selector` | Get element text content |
| `attr` | `selector`, `attribute` | Get attribute value |
| `classes` | `selector` | Get CSS classes |
| `tag-name` | `selector` | Get element tag name |
| `visible` | `selector` | Check if visible |
| `enabled` | `selector` | Check if enabled |
| `selected` | `selector` | Check if selected |
| `count` | `selector` | Count matching elements |
| `find-all` | `selector` | Get all element HTML |
| `exists` | `selector` | Check if element exists |

### JavaScript & Execution
| Command | Args | Description |
|---------|------|-------------|
| `eval` | `js` | Execute JavaScript |
| `inject-css` | `css` | Inject CSS into page |

### Screenshot
| Command | Args | Description |
|---------|------|-------------|
| `screenshot` | `path` | Take full page screenshot |
| `element-screenshot` | `selector`, `path` | Take element screenshot |

### Viewport & Scrolling
| Command | Args | Description |
|---------|------|-------------|
| `viewport` | `width`, `height` | Set viewport size |
| `scroll` | `selector` | Scroll element into view |
| `scroll-by` | `x`, `y` | Scroll by pixels |

### Keyboard
| Command | Args | Description |
|---------|------|-------------|
| `key` | `key` | Press keyboard key |
| `key-combo` | `combo` | Press key combination |

### Storage
| Command | Args | Description |
|---------|------|-------------|
| `cookies` | none | Get all cookies |
| `set-cookie` | `name`, `value`, `domain?`, `path?` | Set cookie |
| `delete-cookie` | `name` | Delete cookie |
| `local-get` | `key` | Get localStorage item |
| `local-set` | `key`, `value` | Set localStorage item |
| `local-remove` | `key` | Remove localStorage item |
| `local-clear` | none | Clear localStorage |
| `session-get` | `key` | Get sessionStorage item |
| `session-set` | `key`, `value` | Set sessionStorage item |
| `session-clear` | none | Clear sessionStorage |

### Console
| Command | Args | Description |
|---------|------|-------------|
| `console` | none | Get console messages |
| `console-log` | `type?` | Get console by type |

### Waiting
| Command | Args | Description |
|---------|------|-------------|
| `wait` | `selector` | Wait for element |
| `wait-gone` | `selector` | Wait for element gone |
| `wait-nav` | none | Wait for navigation |
| `wait-hydration` | none | Wait for Dioxus hydration |

### Dioxus-Specific
| Command | Args | Description |
|---------|------|-------------|
| `dioxus-state` | none | Get Dioxus state |
| `dioxus-click` | `target` | Click Dioxus element by ID |

### Style
| Command | Args | Description |
|---------|------|-------------|
| `style` | `selector`, `property` | Get computed style |

## CLI Structure
```
dioxus-agent-rs [OPTIONS] <COMMAND>
  --url <URL>      Target URL (default: http://localhost:8080)
  --timeout <SEC>  Timeout in seconds (default: 10)
```

## Error Handling
- All errors return non-zero exit code
- Errors printed to stderr with context
- No panics in source code

## Implementation Notes
- Use JavaScript fallbacks for methods not in fantoccini
- Proper escaping for JS injection prevention
- Timeout configurable per-command via CLI flag
