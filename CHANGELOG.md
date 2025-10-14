# Changelog

All notable changes to Fukura will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.7] - 2025-01-27

### Fixed
- Fixed version display to show correct binary name (fukura vs fuku)
- Fixed version number display to show current version instead of hardcoded value
- Improved binary name detection for proper CLI output

## [0.3.6] - 2025-01-27

### Added
- Auto-removal of old versions during installation on all supported platforms
- Enhanced installer scripts for Linux, macOS, and Windows
- Improved installation experience with automatic cleanup

### Changed
- Updated Linux postinstall script to remove old versions from common locations
- Updated macOS postinstall script to remove old versions from common locations
- Updated Windows WiX installer to remove old versions from common locations
- Enhanced cross-platform installation consistency

## [0.3.5] - 2025-01-27

### Fixed
- Fixed clippy linting errors across the codebase
- Improved code quality with proper error handling patterns
- Fixed dead code warnings in remote search functionality
- Improved iterator patterns for better performance
- Fixed field initialization patterns to follow Rust best practices

### Changed
- Updated code formatting to follow Rust conventions
- Improved test error handling with proper panic messages
- Enhanced code documentation and removed empty lines after doc comments

## [0.3.4] - 2025-10-10

### Added
- Simple daemon commands following systemctl/Docker best practices:
  * `fuku start` - Start daemon
  * `fuku stop` - Stop daemon
  * `fuku status` - Check status
  * `fuku restart` - Restart daemon
- Comprehensive help messages for all commands
- fuku init can be run multiple times safely (no error)

### Changed
- Improved UX: daemon commands now use simple verbs instead of flags
- `fuku daemon` reserved for advanced options (hooks, notifications, testing)
- Fixed fukura-site deployment trigger (now triggers on release published)

### Fixed
- fuku init no longer shows error when repository is already initialized
- All command help messages now show descriptive information
- site-deploy.yml now correctly triggers on new releases

## [0.3.3] - 2025-10-10

### Added
- OS-native notifications for error detection (macOS, Linux, Windows)
- Notification control commands (`fuku daemon --notifications-enable/disable/status`)
- Integrated shell hook management into `fuku daemon` command

### Changed
- Consolidated commands: removed `fuku monitor` (merged into `fuku daemon`)
- Consolidated commands: removed `fuku hook` (merged into `fuku daemon`)
- Shell hook management now available via `fuku daemon --install-hooks`, `--uninstall-hooks`, `--hooks-status`
- Improved error capture for all non-zero exit codes

### Removed
- `fuku monitor` command (functionality merged into `fuku daemon`)
- `fuku hook` command (functionality merged into `fuku daemon`)
- Decorative emojis from CLI output for better professionalism

### Fixed
- Better terminal compatibility by removing emoji symbols
- Improved error detection and reporting

## [0.3.2] - 2025-10-10

### Added
- APT repository support for Debian/Ubuntu users
- Automated APT repository deployment to fukura.dev
- Users can now install Fukura with `apt install fukura`

### Changed
- Enhanced CI/CD pipeline to build and deploy APT packages automatically
- Improved release workflow to trigger fukura-site APT deployment

## [0.3.1] - 2024-XX-XX

### Documentation
- Fixed installation instructions (APT repository was not yet hosted)

## [0.3.0] - 2024-XX-XX

### Added
- Enhanced security patterns
- Comprehensive security documentation

### Changed
- Simplified all commands with short options
- Removed emojis from output
- Reorganized documentation structure

### Fixed
- Search argument parsing
- CI/CD issues: cargo fmt and clippy compliance

