# Changelog

All notable changes to Fukura will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

