# Browser Integration

Fukura provides seamless browser integration to display your notes in a beautiful, formatted view. This guide covers how to use the browser features across different platforms.

## Overview

The `fuku open` command opens your notes in a web browser with:
- **Beautiful formatting**: Notes are rendered with proper styling and syntax highlighting
- **Cross-platform support**: Works on Windows, macOS, and Linux
- **Smart fallbacks**: Multiple strategies ensure notes open reliably
- **Local server**: Automatic HTTP server for environments where direct file opening fails

## Basic Usage

### Open a Note in Browser
```bash
fuku open <note-id>
```

### Examples
```bash
# Open a specific note
fuku open abc123

# Open with light theme
fuku open abc123 --theme light

# Open with dark theme (default)
fuku open abc123 --theme dark
```

## Command Options

### `--theme`
Choose between light and dark themes:
```bash
fuku open abc123 --theme light
fuku open abc123 --theme dark
```

### `--browser-only`
Force direct browser opening, skip local server fallback:
```bash
fuku open abc123 --browser-only
```

### `--url-only`
Show file path for manual opening instead of automatic browser opening:
```bash
fuku open abc123 --url-only
```

### `--server-port`
Specify port for local server (when browser opening fails):
```bash
fuku open abc123 --server-port 8080
```

## Platform-Specific Behavior

### Windows
- Uses `rundll32 url.dll,FileProtocolHandler` for default browser
- Supports WSL2 with `wslview` command
- Falls back to local HTTP server if needed

### macOS
- Uses `open` command for default browser
- Integrates with macOS file associations
- Supports custom browser via `BROWSER` environment variable

### Linux
- Tries `xdg-open` for desktop environments
- Supports WSL2 with `wslview`
- Falls back to common browsers (Firefox, Chrome, Chromium, etc.)
- Uses `BROWSER` environment variable if set

## WSL2 Support

For Windows Subsystem for Linux (WSL2) users:

### Install wslview (Recommended)
```bash
sudo apt update
sudo apt install wslu
```

This enables automatic opening of notes in Windows default browser.

### Alternative: Use URL-only mode
```bash
fuku open abc123 --url-only
```

This saves the HTML file and shows the path for manual opening.

## Troubleshooting

### Browser Doesn't Open Automatically

1. **Check available commands**:
   ```bash
   # On Linux/WSL2
   which xdg-open wslview
   
   # On macOS
   which open
   
   # On Windows
   where rundll32
   ```

2. **Use browser-only mode**:
   ```bash
   fuku open abc123 --browser-only
   ```

3. **Use URL-only mode**:
   ```bash
   fuku open abc123 --url-only
   ```

4. **Set BROWSER environment variable**:
   ```bash
   export BROWSER=firefox
   fuku open abc123
   ```

### Local Server Issues

If the local server fails to start:

1. **Check port availability**:
   ```bash
   fuku open abc123 --server-port 8081
   ```

2. **Use browser-only mode**:
   ```bash
   fuku open abc123 --browser-only
   ```

### WSL2 Specific Issues

If you see Microsoft Store opening instead of your browser:

1. **Install wslu**:
   ```bash
   sudo apt install wslu
   ```

2. **Create xdg-open symlink** (optional):
   ```bash
   sudo ln -s $(which wslview) /usr/local/bin/xdg-open
   ```

3. **Use URL-only mode**:
   ```bash
   fuku open abc123 --url-only
   ```

## Advanced Configuration

### Environment Variables

- `BROWSER`: Override default browser command
- `FUKURA_BROWSER_PORT`: Default port for local server

### Custom Browser Setup

You can configure Fukura to use a specific browser:

```bash
# Use Firefox
export BROWSER=firefox
fuku open abc123

# Use Chrome
export BROWSER=google-chrome
fuku open abc123

# Use custom command
export BROWSER="firefox --new-window"
fuku open abc123
```

## HTML Output

Fukura generates clean, semantic HTML with:

- **Responsive design**: Works on desktop and mobile
- **Syntax highlighting**: Code blocks are properly formatted
- **Dark/light themes**: Choose your preferred theme
- **Print-friendly**: Optimized for printing
- **Accessibility**: Proper semantic markup and ARIA labels

### Sample HTML Structure
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Note Title</title>
    <style>
        /* Embedded CSS for styling */
    </style>
</head>
<body>
    <article class="note">
        <header>
            <h1>Note Title</h1>
            <div class="metadata">
                <span class="author">Author Name</span>
                <span class="date">2024-01-01</span>
                <span class="tags">tag1, tag2</span>
            </div>
        </header>
        <div class="content">
            <!-- Note content with proper formatting -->
        </div>
    </article>
</body>
</html>
```

## Integration with Other Tools

### VS Code
You can open Fukura notes directly in VS Code:
```bash
fuku open abc123 --url-only
code /tmp/fuku-abc123.html
```

### Custom Scripts
```bash
#!/bin/bash
# Open note and copy URL to clipboard
NOTE_ID=$1
fuku open "$NOTE_ID" --url-only | grep "file://" | xclip -selection clipboard
```

## Best Practices

1. **Use appropriate themes**: Light theme for bright environments, dark theme for low light
2. **Set BROWSER environment variable**: For consistent behavior across sessions
3. **Use --url-only for automation**: When integrating with other tools
4. **Keep wslview updated**: On WSL2 systems, keep wslu package updated

## Troubleshooting Commands

```bash
# Test browser detection
fuku open --help

# Check available browsers
which firefox google-chrome chromium brave-browser

# Test local server
fuku open abc123 --server-port 8080

# Force direct opening
fuku open abc123 --browser-only

# Manual file opening
fuku open abc123 --url-only
```

For more help, see the [Troubleshooting Guide](./troubleshooting.md) or create an issue on [GitHub](https://github.com/boostbit-inc/fukura/issues).
