# Security Policy

## Automatic Secret Redaction

Fukura automatically redacts sensitive information from notes to prevent accidental exposure of secrets.

### Protected Patterns

The following patterns are automatically detected and redacted:

#### Cloud Credentials
- **AWS Access Keys**: `AKIA[0-9A-Z]{16}`
- **AWS Secret Keys**: AWS secret patterns
- **GitHub Tokens**: `ghp_*` and `gho_*` patterns

#### API Keys & Tokens
- **Bearer Tokens**: `bearer [token]` patterns
- **API Keys**: `api_key=...` patterns
- **JWT Tokens**: JWT format tokens

#### Sensitive Data
- **Passwords**: Password assignment patterns
- **Database URLs**: Connection strings for Postgres, MySQL, MongoDB
- **Private Keys**: RSA and EC private key headers
- **Email Addresses**: Valid email patterns
- **IP Addresses**: IPv4 addresses

### Custom Redaction Rules

You can add custom redaction patterns:

```bash
# Add a custom pattern
fuku config redact --set my_token='MY_TOKEN_[A-Z0-9]{20}'

# Remove a pattern
fuku config redact --unset email  # Disable email redaction
```

## Data Storage

### Local Storage
- All notes are stored locally in `.fukura/` directory
- Each project has its own isolated repository
- No data is transmitted without explicit user action

### Remote Sync (Optional)
- Notes are only synced when you run `fuku sync`
- All sync operations use HTTPS
- Redaction is applied before any network transmission

## Privacy Levels

### Private (Default)
- Notes are stored locally only
- Not shared with anyone
- Recommended for all sensitive information

### Org (Organization)
- For enterprise use
- Shared within organization boundary
- Requires FukuraHub enterprise deployment

### Public
- Shared publicly
- Must be explicitly set by user
- Redaction still applied

## Reporting Security Issues

If you discover a security vulnerability, please email: security@fukura.dev

**Do NOT** create a public GitHub issue for security vulnerabilities.

## Best Practices

1. **Always review before syncing**: Check notes with `fuku view` before `fuku sync`
2. **Use custom redaction**: Add patterns for your organization's secret formats
3. **Keep notes private**: Only make public after thorough review
4. **Regular audits**: Periodically review stored notes for sensitive data
5. **Disable IP redaction if needed**: `fuku config redact --unset ipv4`

## Encryption (Roadmap)

Future versions will support:
- At-rest encryption for local storage
- End-to-end encryption for remote sync
- Hardware security key support
