/// Performance optimization utilities for fukura
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Performance metrics collector
pub struct PerformanceMetrics {
    activities_processed: AtomicUsize,
    activities_filtered: AtomicUsize,
    sessions_active: AtomicUsize,
    start_time: Instant,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            activities_processed: AtomicUsize::new(0),
            activities_filtered: AtomicUsize::new(0),
            sessions_active: AtomicUsize::new(0),
            start_time: Instant::now(),
        }
    }

    pub fn record_activity_processed(&self) {
        self.activities_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_activity_filtered(&self) {
        self.activities_filtered.fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_active_sessions(&self, count: usize) {
        self.sessions_active.store(count, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> PerformanceStats {
        PerformanceStats {
            activities_processed: self.activities_processed.load(Ordering::Relaxed),
            activities_filtered: self.activities_filtered.load(Ordering::Relaxed),
            sessions_active: self.sessions_active.load(Ordering::Relaxed),
            uptime: self.start_time.elapsed(),
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PerformanceStats {
    pub activities_processed: usize,
    pub activities_filtered: usize,
    pub sessions_active: usize,
    pub uptime: Duration,
}

impl PerformanceStats {
    pub fn activities_per_second(&self) -> f64 {
        if self.uptime.as_secs() == 0 {
            return 0.0;
        }
        self.activities_processed as f64 / self.uptime.as_secs() as f64
    }

    pub fn filter_rate(&self) -> f64 {
        if self.activities_processed == 0 {
            return 0.0;
        }
        self.activities_filtered as f64 / self.activities_processed as f64
    }
}

/// Adaptive rate limiter for activity monitoring
pub struct RateLimiter {
    max_rate: usize,
    window_size: Duration,
    last_reset: Instant,
    current_count: AtomicUsize,
}

impl RateLimiter {
    pub fn new(max_rate: usize, window_size: Duration) -> Self {
        Self {
            max_rate,
            window_size,
            last_reset: Instant::now(),
            current_count: AtomicUsize::new(0),
        }
    }

    pub fn should_allow(&mut self) -> bool {
        // Reset window if needed
        if self.last_reset.elapsed() >= self.window_size {
            self.current_count.store(0, Ordering::Relaxed);
            self.last_reset = Instant::now();
        }

        let current = self.current_count.fetch_add(1, Ordering::Relaxed);
        current < self.max_rate
    }

    pub fn current_rate(&self) -> usize {
        self.current_count.load(Ordering::Relaxed)
    }
}

/// Batch processor for efficient activity handling
pub struct BatchProcessor<T> {
    batch: Vec<T>,
    max_batch_size: usize,
    flush_interval: Duration,
    last_flush: Instant,
}

impl<T> BatchProcessor<T> {
    pub fn new(max_batch_size: usize, flush_interval: Duration) -> Self {
        Self {
            batch: Vec::with_capacity(max_batch_size),
            max_batch_size,
            flush_interval,
            last_flush: Instant::now(),
        }
    }

    pub fn add(&mut self, item: T) -> Option<Vec<T>> {
        self.batch.push(item);

        // Flush if batch is full or interval elapsed
        if self.batch.len() >= self.max_batch_size 
            || self.last_flush.elapsed() >= self.flush_interval 
        {
            self.flush()
        } else {
            None
        }
    }

    pub fn flush(&mut self) -> Option<Vec<T>> {
        if self.batch.is_empty() {
            return None;
        }

        self.last_flush = Instant::now();
        Some(std::mem::replace(&mut self.batch, Vec::with_capacity(self.max_batch_size)))
    }

    pub fn len(&self) -> usize {
        self.batch.len()
    }

    pub fn is_empty(&self) -> bool {
        self.batch.is_empty()
    }
}

/// Memory-efficient circular buffer for recent activities
pub struct CircularBuffer<T> {
    buffer: Vec<Option<T>>,
    capacity: usize,
    write_pos: usize,
    len: usize,
}

impl<T: Clone> CircularBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![None; capacity],
            capacity,
            write_pos: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, item: T) {
        self.buffer[self.write_pos] = Some(item);
        self.write_pos = (self.write_pos + 1) % self.capacity;
        if self.len < self.capacity {
            self.len += 1;
        }
    }

    pub fn to_vec(&self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.len);
        
        if self.len == self.capacity {
            // Buffer is full, start from write_pos
            for i in 0..self.capacity {
                let index = (self.write_pos + i) % self.capacity;
                if let Some(item) = &self.buffer[index] {
                    result.push(item.clone());
                }
            }
        } else {
            // Buffer not full yet, items are at start
            for item in &self.buffer[..self.len] {
                if let Some(item) = item {
                    result.push(item.clone());
                }
            }
        }
        
        result
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics::new();
        
        metrics.record_activity_processed();
        metrics.record_activity_processed();
        metrics.record_activity_filtered();
        
        let stats = metrics.get_stats();
        assert_eq!(stats.activities_processed, 2);
        assert_eq!(stats.activities_filtered, 1);
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(3, Duration::from_secs(1));
        
        assert!(limiter.should_allow());
        assert!(limiter.should_allow());
        assert!(limiter.should_allow());
        assert!(!limiter.should_allow()); // 4th should be blocked
    }

    #[test]
    fn test_batch_processor() {
        let mut processor = BatchProcessor::new(3, Duration::from_secs(10));
        
        assert_eq!(processor.add(1), None);
        assert_eq!(processor.add(2), None);
        
        // 3rd item should trigger flush
        let batch = processor.add(3);
        assert!(batch.is_some());
        assert_eq!(batch.unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_circular_buffer() {
        let mut buffer = CircularBuffer::new(3);
        
        buffer.push(1);
        buffer.push(2);
        assert_eq!(buffer.len(), 2);
        
        buffer.push(3);
        assert_eq!(buffer.to_vec(), vec![1, 2, 3]);
        
        // Overflow - should overwrite oldest
        buffer.push(4);
        assert_eq!(buffer.to_vec(), vec![2, 3, 4]);
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_circular_buffer_capacity() {
        let buffer: CircularBuffer<i32> = CircularBuffer::new(100);
        assert_eq!(buffer.capacity(), 100);
        assert!(buffer.is_empty());
    }
}

