# Changelog

All notable changes to Fukura will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

