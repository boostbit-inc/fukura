# Getting Started with Fukura

Fukura is a powerful CLI tool for capturing and searching recurring error fixes in a content-addressable store. This guide will help you get up and running quickly.

## What is Fukura?

Fukura helps developers and teams:
- **Capture** recurring error fixes and solutions
- **Store** them in a searchable knowledge base
- **Retrieve** solutions quickly when similar issues arise
- **Share** knowledge across team members

## Installation

### Option 1: Using Cargo (Recommended)
```bash
cargo install fukura
```

### Option 2: Using Docker
```bash
docker pull fukura/fukura:latest
```

### Option 3: Download Binary
Download the latest release from [GitHub Releases](https://github.com/boostbit-inc/fukura/releases).

## Quick Start

### 1. Initialize a Repository
```bash
fukura init
```

This creates a `.fukura` directory in your current location to store your knowledge base.

### 2. Add Your First Note
```bash
fukura add
```

Follow the interactive prompts to add a solution to your knowledge base.

### 3. Search for Solutions
```bash
fukura search "database connection error"
```

### 4. List All Notes
```bash
fukura list
```

## Basic Workflow

1. **Encounter an Error**: When you solve a recurring problem
2. **Capture the Solution**: Use `fukura add` to store it
3. **Tag and Categorize**: Add relevant tags for easy discovery
4. **Search When Needed**: Use `fukura search` to find similar solutions

## Example: Adding a Solution

```bash
$ fukura add
Title: Fix PostgreSQL connection timeout
Body: 
When PostgreSQL connections timeout, check:
1. Network connectivity to database server
2. Database server load and capacity
3. Connection pool settings
4. Firewall rules

Solution:
- Increase connection timeout in pg_hba.conf
- Adjust connection pool max_connections
- Check network latency

Tags: postgresql, database, connection, timeout
Privacy: private
```

## Configuration

Fukura uses a configuration file at `~/.fukura/config.toml`. You can customize:

- Default privacy settings
- Editor preferences
- Search behavior
- Output formatting

See [Configuration Guide](./configuration.md) for details.

## Next Steps

- [Configuration Guide](./configuration.md) - Customize Fukura for your workflow
- [Command Reference](./commands.md) - Complete command documentation
- [Troubleshooting](./troubleshooting.md) - Common issues and solutions

## Getting Help

- Check the [Troubleshooting Guide](./troubleshooting.md)
- Search existing issues on [GitHub](https://github.com/boostbit-inc/fukura/issues)
- Create a new issue if you can't find a solution
