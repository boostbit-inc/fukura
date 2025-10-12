# Time-Based Recording Feature Implementation

## Overview

Successfully implemented the requested time-based recording feature for the `fuku rec` command. This allows you to start recording from a specific time in the past, making it easier to capture command history retroactively.

## Usage

### Basic Commands

```bash
# Start recording from 3 minutes ago
fuku rec "Debugging database issue" 3m ago

# Start recording from 2 hours ago  
fuku rec "Deploy hotfix" 2h ago

# Start recording from 1 hour and 30 minutes ago
fuku rec "Fix authentication" 1h 30m ago

# Check current recording status
fuku rec --status
```

### Supported Time Formats

- `3m ago` - 3 minutes ago
- `2h ago` - 2 hours ago  
- `1h 30m ago` - 1 hour 30 minutes ago
- `45m ago` - 45 minutes ago
- `2h 15m ago` - 2 hours 15 minutes ago

## Configuration

The feature includes configurable limits that can be set in `.fukura/config.toml`:

```toml
[recording]
max_lookback_hours = 3      # Maximum time to look back (default: 3h)
min_lookback_minutes = 1    # Minimum time to look back (default: 1m)
```

### Default Limits
- **Maximum lookback**: 3 hours (configurable)
- **Minimum lookback**: 1 minute (configurable)

These limits prevent performance issues from retrieving too much historical data while ensuring practical usage.

## Features Implemented

### ✅ Core Components

1. **Time Expression Parser** (`src/time_parser.rs`)
   - Parses natural language time expressions
   - Supports multiple formats (hours, minutes, combinations)
   - Comprehensive error handling and validation

2. **Configuration System** (`src/config.rs`)
   - Added `RecordingConfig` struct with configurable limits
   - Integrates with existing configuration system
   - Supports both local and global configuration

3. **Daemon Integration** (`src/daemon.rs`)
   - Extended daemon to support historical command retrieval
   - Added methods for time-based recording session creation
   - Maintains existing functionality while adding new capabilities

4. **Command Interface** (`src/cli.rs`)
   - Added `from_time` parameter to `RecCommand`
   - Integrated time-based logic with existing rec command
   - Automatic daemon startup if needed
   - User-friendly error messages and help text

### ✅ Key Features

- **Backward Compatibility**: Existing `fuku rec` commands continue to work unchanged
- **Automatic Validation**: Validates time expressions against configured limits
- **Daemon Integration**: Automatically starts daemon if needed for historical retrieval
- **Error Handling**: Comprehensive validation and user-friendly error messages
- **Configurable Limits**: Prevents performance issues while maintaining flexibility

## How It Works

1. **Time Parsing**: The system parses time expressions like "3m ago" into absolute timestamps
2. **Validation**: Checks against configured minimum and maximum lookback limits
3. **Daemon Check**: Ensures the daemon is running for command history access
4. **Historical Retrieval**: Retrieves commands from the specified time period
5. **Recording Start**: Creates a recording session that includes historical commands
6. **Continued Recording**: Continues recording new commands as they happen

## Technical Implementation

The implementation follows Rust best practices and integrates seamlessly with the existing codebase:

- **Type Safety**: Uses Rust's type system for robust time handling
- **Error Handling**: Comprehensive error handling with `anyhow::Result`
- **Configuration**: Integrates with existing TOML-based configuration
- **Async Support**: Properly handles async operations for daemon communication
- **Testing**: Includes test cases for time parsing and validation

## Usage Examples

### Scenario 1: Forgot to Start Recording
```bash
# You realize you want to record a debugging session that started 10 minutes ago
fuku rec "Database connection debugging" 10m ago
```

### Scenario 2: Capture Long Troubleshooting Session
```bash  
# Capture a troubleshooting session that started 2 hours ago
fuku rec "API performance investigation" 2h ago
```

### Scenario 3: Quick Fix Documentation
```bash
# Document a quick fix you implemented 5 minutes ago
fuku rec "Fix CORS headers" 5m ago
```

## Error Messages and Help

The feature provides clear, helpful error messages:

- Invalid time formats are explained with examples
- Time limit violations show current configuration
- Missing daemon gives instructions for starting it
- Configuration errors provide guidance for fixing

## Best Practices

1. **Use Reasonable Time Ranges**: Don't go back too far to avoid performance issues
2. **Configure Limits**: Adjust limits based on your usage patterns and system performance
3. **Descriptive Titles**: Use clear, descriptive titles for your recording sessions
4. **Regular Commits**: Use `fuku done` regularly to save your recordings

## Future Enhancements

The implementation provides a solid foundation for future enhancements:
- Support for more time formats (e.g., "yesterday", "this morning")
- Integration with shell history for better command matching
- Advanced filtering options for command selection
- Performance optimizations for large command histories

This feature makes `fuku` significantly more user-friendly by reducing the friction of retroactive command recording, addressing the common use case of realizing you want to record something after you've already started working on it.
