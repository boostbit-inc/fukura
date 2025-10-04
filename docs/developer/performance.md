# Performance Considerations

This document outlines performance optimizations and considerations for Fukura.

## Recent Performance Improvements

### 1. Batch Processing Optimization

**Problem**: Individual note storage was inefficient for bulk operations.

**Solution**: Implemented batch processing for note storage and indexing.

```rust
// Before: Individual processing
for note in notes {
    repo.store_note(note)?; // Each call commits to index
}

// After: Batch processing
repo.store_notes_batch(notes)?; // Single commit for all notes
```

**Impact**: 
- Memory usage test: 60+ seconds → 2.20 seconds
- Bulk insert performance: 10+ seconds → 10.20 seconds (with 5x more data)

### 2. Memory Optimization in Pack Processing

**Problem**: Memory allocations during pack file creation.

**Solution**: Pre-allocated buffer reuse.

```rust
// Before: New Vec for each file
let mut data = Vec::new();

// After: Reused buffer
let mut buffer = Vec::with_capacity(1024 * 1024); // 1MB buffer
buffer.clear(); // Reuse for each file
```

**Impact**: Reduced memory allocations and improved pack processing performance.

### 3. Search Performance Optimization

**Problem**: Stable sorting was inefficient for large result sets.

**Solution**: Use unstable sorting for better performance.

```rust
// Before: Stable sort
hits.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

// After: Unstable sort (20-30% faster)
hits.sort_unstable_by(|a, b| b.updated_at.cmp(&a.updated_at));
```

**Impact**: 20-30% improvement in search result sorting.

### 4. Daemon Session Management Optimization

**Problem**: Session cleanup was inefficient with many active sessions.

**Solution**: Batch processing for session cleanup.

```rust
// Before: Iterate and modify simultaneously
sessions_guard.retain(|_, session| { ... });

// After: Collect then remove
let mut to_remove = Vec::new();
for (id, session) in sessions_guard.iter() { ... }
for id in to_remove { sessions_guard.remove(&id); }
```

**Impact**: Eliminated borrow checker issues and improved cleanup performance.

## Performance Monitoring

### Benchmarking

Run performance benchmarks to monitor improvements:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmarks
cargo bench --bench search_benchmark

# Compare before/after
cargo bench --bench search_benchmark -- --save-baseline before
# ... make changes ...
cargo bench --bench search_benchmark -- --baseline before
```

### Performance Tests

```bash
# Memory usage tests
cargo test --test performance

# Search performance tests
cargo test --test performance test_search_performance

# Bulk insert tests
cargo test --test performance test_bulk_insert_performance
```

## Data Volume Considerations

### Small Scale (1-100 notes)
- Current optimizations are sufficient
- No additional considerations needed

### Medium Scale (100-1000 notes)
- Batch processing provides significant benefits
- Pack files help reduce storage overhead
- Search performance remains good

### Large Scale (1000+ notes)
- Consider index segmentation
- Implement async processing for I/O operations
- Monitor memory usage during bulk operations
- Consider pagination for search results

## Configuration Tuning

### Daemon Configuration

```rust
// Default optimized settings
DaemonConfig {
    monitor_interval: Duration::from_secs(10),    // Reduced frequency
    session_timeout: Duration::from_secs(600),    // 10 minutes
    max_sessions: 50,                             // Reduced for memory
    // ...
}
```

### Directory Monitoring

```rust
// Optimized check interval
check_interval: Duration::from_secs(30), // Reduced CPU usage
```

## Best Practices

### 1. Use Batch Operations
Always prefer batch operations for multiple items:

```rust
// Good
repo.store_notes_batch(notes)?;

// Avoid
for note in notes {
    repo.store_note(note)?;
}
```

### 2. Minimize Index Commits
Batch index updates to reduce I/O:

```rust
// Good: Single commit
let mut writer = index.writer(50_000_000)?;
for record in records {
    writer.add_document(document)?;
}
writer.commit()?;

// Avoid: Multiple commits
for record in records {
    let mut writer = index.writer(50_000_000)?;
    writer.add_document(document)?;
    writer.commit()?;
}
```

### 3. Optimize Memory Usage
Reuse buffers and avoid unnecessary allocations:

```rust
// Good: Reuse buffer
let mut buffer = Vec::with_capacity(1024 * 1024);
buffer.clear();

// Avoid: New allocation each time
let mut data = Vec::new();
```

### 4. Use Efficient Sorting
Prefer unstable sort for large datasets:

```rust
// Good: Faster for large datasets
vec.sort_unstable_by(|a, b| a.cmp(b));

// OK: Stable but slower
vec.sort_by(|a, b| a.cmp(b));
```

## Monitoring Performance

### Key Metrics to Watch

1. **Memory Usage**: Monitor during bulk operations
2. **Search Latency**: Track search response times
3. **Index Size**: Monitor search index growth
4. **Disk I/O**: Watch for excessive file operations

### Performance Regression Prevention

1. Run benchmarks before/after changes
2. Monitor CI performance test results
3. Set performance budgets for critical operations
4. Use profiling tools for deep analysis

```bash
# Profile with perf (Linux)
perf record --call-graph dwarf cargo bench
perf report

# Profile with cargo-instruments (macOS)
cargo install cargo-instruments
cargo instruments -t time cargo bench
```
