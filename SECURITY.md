# Security Policy

## Supported Versions

We provide security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please follow these steps:

### 1. Do NOT open a public issue

Security vulnerabilities should be reported privately to prevent exploitation.

### 2. Contact us privately

Send an email to: **security@fukura.dev**

Include the following information:
- Description of the vulnerability
- Steps to reproduce the issue
- Potential impact assessment
- Any suggested fixes (if applicable)

### 3. What to expect

- We will acknowledge receipt within 48 hours
- We will investigate and provide updates within 7 days
- We will work with you to verify the fix
- We will coordinate the public disclosure timeline

### 4. Responsible Disclosure

We follow responsible disclosure practices:
- We will not disclose the vulnerability until a fix is available
- We will credit you (if desired) when we announce the fix
- We will provide a reasonable timeframe for fixes (typically 30-90 days)

## Security Features

Fukura implements several security measures:

### Input Sanitization
- All user inputs are validated and sanitized
- Protection against path traversal attacks
- Prevention of script injection

### File System Security
- Repository files are stored with appropriate permissions
- No world-writable directories
- Secure temporary file handling

### Network Security
- HTTPS-only for remote operations
- Certificate validation
- No hardcoded credentials

### Dependency Security
- Regular security audits with `cargo audit`
- License compliance checking
- Dependency vulnerability scanning

## Security Best Practices

When using Fukura:

1. **Keep it updated**: Always use the latest version
2. **Secure storage**: Store repositories in secure locations
3. **Access control**: Limit access to sensitive repositories
4. **Backup**: Regularly backup your data
5. **Network**: Use secure networks for remote operations

## Security Tools

We use several tools to maintain security:

- `cargo audit` - Dependency vulnerability scanning
- `cargo deny` - License compliance and security checks
- Trivy - Container security scanning
- GitHub Security Advisories

## Bug Bounty

We appreciate security researchers who help improve Fukura's security. While we don't currently offer monetary rewards, we do:

- Provide public recognition for responsible disclosure
- Include contributors in our security hall of fame
- Offer early access to new features for security researchers

## Security Updates

Security updates are released as soon as possible after a vulnerability is discovered and fixed. We recommend:

- Enabling automatic updates where possible
- Monitoring our security advisories
- Subscribing to our security mailing list

Thank you for helping keep Fukura secure!

