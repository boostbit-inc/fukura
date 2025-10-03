# Architecture Overview

This document provides a high-level overview of Fukura's architecture and design decisions.

## System Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CLI Interface │    │   Web Interface │    │   API Interface │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          └──────────────────────┼──────────────────────┘
                                 │
                    ┌─────────────┴─────────────┐
                    │      Core Library         │
                    │   (fukura crate)          │
                    └─────────────┬─────────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    │     Storage Layer         │
                    │   (Content-Addressable)   │
                    └─────────────┬─────────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    │     Search Engine         │
                    │      (Tantivy)            │
                    └───────────────────────────┘
```

## Core Components

### 1. CLI Interface (`src/cli.rs`)
- Command-line argument parsing using Clap
- Interactive prompts using Dialoguer
- Output formatting and display

### 2. Repository Management (`src/repo.rs`)
- Repository initialization and configuration
- Note storage and retrieval
- Object management (pack files, loose objects)

### 3. Search Engine (`src/index.rs`)
- Full-text search using Tantivy
- Index management and optimization
- Search result ranking and filtering

### 4. Data Models (`src/models.rs`)
- Note structure and metadata
- Author information
- Privacy settings and access control

### 5. Storage System (`src/pack.rs`)
- Content-addressable storage
- Object compression and deduplication
- Pack file management

## Data Flow

### Adding a Note
```
1. User input (CLI/API)
2. Validation and sanitization
3. Content-addressable storage
4. Search index update
5. Metadata storage
6. Confirmation to user
```

### Searching Notes
```
1. User query (CLI/API)
2. Query parsing and validation
3. Search index lookup
4. Result ranking and filtering
5. Content retrieval
6. Formatted output
```

## Storage Architecture

### Content-Addressable Storage
- **Objects**: Immutable blobs identified by SHA-256 hash
- **Packs**: Compressed collections of objects
- **Index**: Fast lookup of object locations

### Directory Structure
```
.fukura/
├── config.toml          # Repository configuration
├── objects/             # Loose objects
│   ├── 12/
│   │   └── 3456...     # Object files
├── packs/               # Pack files
│   ├── pack-1.pack     # Compressed objects
│   └── pack-1.idx      # Pack index
├── index/               # Search index
│   ├── meta.json       # Index metadata
│   └── tantivy/        # Tantivy index files
└── refs/                # Object references
    └── notes/          # Note object IDs
```

## Search Architecture

### Tantivy Integration
- **Schema**: Predefined fields for notes (title, body, tags, etc.)
- **Indexing**: Automatic index updates on note changes
- **Searching**: Full-text search with relevance scoring

### Search Features
- **Full-text search**: Search across title, body, and tags
- **Filtering**: Filter by author, privacy, date range
- **Sorting**: Sort by relevance, date, title
- **Highlighting**: Highlight matching terms in results

## Security Considerations

### Data Protection
- **Encryption**: Optional encryption for sensitive data
- **Access Control**: Privacy-based access control
- **Input Validation**: Comprehensive input sanitization

### Privacy Levels
- **Public**: Accessible to all users
- **Private**: Accessible only to the author
- **Protected**: Accessible to specific users/groups

## Performance Optimizations

### Indexing
- **Incremental updates**: Only update changed documents
- **Batch operations**: Group multiple operations
- **Background indexing**: Non-blocking index updates

### Storage
- **Compression**: Zlib compression for objects
- **Deduplication**: Automatic content deduplication
- **Pack optimization**: Periodic pack file optimization

### Search
- **Query optimization**: Optimized query parsing
- **Result caching**: Cache frequent search results
- **Index warming**: Preload frequently accessed data

## Extensibility

### Plugin System
- **Storage backends**: Pluggable storage implementations
- **Search engines**: Alternative search implementations
- **Export formats**: Multiple export format support

### API Design
- **Trait-based**: Core functionality exposed as traits
- **Async support**: Async/await for I/O operations
- **Error handling**: Comprehensive error types

## Configuration

### Repository Configuration
```toml
[repository]
version = 1
compression = "zlib"
encryption = false

[search]
index_path = "index"
max_results = 100
default_sort = "relevance"

[privacy]
default_level = "private"
allow_public = true
```

### User Configuration
```toml
[user]
name = "John Doe"
email = "john@example.com"
editor = "vim"

[cli]
color = true
pager = "less"
confirm_deletes = true
```

## Testing Strategy

### Unit Tests
- **Model validation**: Test data model constraints
- **Storage operations**: Test storage layer functionality
- **Search operations**: Test search functionality

### Integration Tests
- **End-to-end workflows**: Test complete user workflows
- **Cross-component**: Test component interactions
- **Performance**: Test performance characteristics

### Security Tests
- **Input validation**: Test malicious input handling
- **Access control**: Test privacy enforcement
- **Data integrity**: Test data corruption scenarios

## Monitoring and Observability

### Logging
- **Structured logging**: JSON-formatted logs
- **Log levels**: Configurable log verbosity
- **Performance metrics**: Operation timing and counts

### Metrics
- **Storage metrics**: Repository size, object counts
- **Search metrics**: Query performance, result counts
- **User metrics**: Usage patterns, feature adoption

## Future Considerations

### Scalability
- **Distributed storage**: Support for distributed backends
- **Horizontal scaling**: Multiple repository support
- **Cloud integration**: Cloud storage backends

### Features
- **Collaboration**: Multi-user collaboration features
- **Versioning**: Note versioning and history
- **Import/Export**: Enhanced data portability
