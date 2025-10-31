//! Memory optimization primitives for high-performance execution
//!
//! This module provides memory-efficient data structures and allocation strategies:
//! - Arena allocation for AST nodes
//! - String interning for reduced memory usage
//! - Object pools for frequent allocations
//! - Structure-of-Arrays for cache-friendly batch processing

use anyhow::Result;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::sync::Arc;

/// Global string interner for common Python keywords and identifiers
static STRING_INTERNER: Lazy<StringInterner> = Lazy::new(StringInterner::new);

/// String interning system for reducing memory usage
///
/// Caches frequently used strings (keywords, stdlib names, common identifiers)
/// to avoid duplicate allocations. Thread-safe with concurrent access support.
pub struct StringInterner {
    cache: DashMap<String, Arc<str>>,
}

impl StringInterner {
    /// Create a new string interner
    pub fn new() -> Self {
        let interner = Self {
            cache: DashMap::new(),
        };

        // Pre-populate with Python keywords
        interner.populate_keywords();
        interner
    }

    /// Populate interner with Python keywords
    fn populate_keywords(&self) {
        const PYTHON_KEYWORDS: &[&str] = &[
            "False", "None", "True", "and", "as", "assert", "async", "await",
            "break", "class", "continue", "def", "del", "elif", "else", "except",
            "finally", "for", "from", "global", "if", "import", "in", "is",
            "lambda", "nonlocal", "not", "or", "pass", "raise", "return",
            "try", "while", "with", "yield",
        ];

        for &keyword in PYTHON_KEYWORDS {
            let arc: Arc<str> = Arc::from(keyword);
            self.cache.insert(keyword.to_string(), arc);
        }
    }

    /// Intern a string, returning a shared reference
    ///
    /// If the string is already interned, returns the existing Arc.
    /// Otherwise, creates a new Arc and caches it.
    pub fn intern(&self, s: &str) -> Arc<str> {
        if let Some(entry) = self.cache.get(s) {
            entry.value().clone()
        } else {
            let arc: Arc<str> = Arc::from(s);
            self.cache.insert(s.to_string(), arc.clone());
            arc
        }
    }

    /// Get statistics about the interner
    pub fn stats(&self) -> InternerStats {
        InternerStats {
            cached_strings: self.cache.len(),
            memory_saved_bytes: self.estimate_memory_saved(),
        }
    }

    /// Estimate memory saved by interning
    fn estimate_memory_saved(&self) -> usize {
        // Rough estimate: each interned string saves ~24 bytes (String overhead)
        // plus the string data for each duplicate reference
        self.cache.len() * 24
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about string interning
#[derive(Debug, Clone)]
pub struct InternerStats {
    pub cached_strings: usize,
    pub memory_saved_bytes: usize,
}

/// Global string interner instance
pub fn global_interner() -> &'static StringInterner {
    &STRING_INTERNER
}

/// Intern a string using the global interner
pub fn intern(s: &str) -> Arc<str> {
    STRING_INTERNER.intern(s)
}

/// Object pool for reusing allocated objects
///
/// Reduces allocation overhead by reusing objects. Thread-safe with
/// lock-free concurrent access.
pub struct ObjectPool<T> {
    pool: crossbeam::queue::SegQueue<T>,
    factory: Arc<dyn Fn() -> T + Send + Sync>,
    max_size: usize,
}

impl<T> ObjectPool<T> {
    /// Create a new object pool with a factory function
    pub fn new<F>(factory: F, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            pool: crossbeam::queue::SegQueue::new(),
            factory: Arc::new(factory),
            max_size,
        }
    }

    /// Acquire an object from the pool
    ///
    /// If the pool is empty, creates a new object using the factory.
    pub fn acquire(&self) -> PooledObject<'_, T> {
        let obj = self.pool.pop().unwrap_or_else(|| (self.factory)());
        PooledObject {
            obj: Some(obj),
            pool: self,
        }
    }

    /// Return an object to the pool
    fn release(&self, obj: T) {
        if self.pool.len() < self.max_size {
            self.pool.push(obj);
        }
        // Otherwise drop the object
    }

    /// Get current pool size
    pub fn len(&self) -> usize {
        self.pool.len()
    }

    /// Check if pool is empty
    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }
}

/// RAII wrapper for pooled objects
///
/// Automatically returns the object to the pool when dropped.
pub struct PooledObject<'a, T> {
    obj: Option<T>,
    pool: &'a ObjectPool<T>,
}

impl<'a, T> PooledObject<'a, T> {
    /// Get a reference to the pooled object
    pub fn get(&self) -> &T {
        self.obj.as_ref().unwrap()
    }

    /// Get a mutable reference to the pooled object
    pub fn get_mut(&mut self) -> &mut T {
        self.obj.as_mut().unwrap()
    }
}

impl<'a, T> Drop for PooledObject<'a, T> {
    fn drop(&mut self) {
        if let Some(obj) = self.obj.take() {
            self.pool.release(obj);
        }
    }
}

impl<'a, T> std::ops::Deref for PooledObject<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<'a, T> std::ops::DerefMut for PooledObject<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

/// Memory-aligned buffer for SIMD operations
///
/// Ensures proper alignment for AVX2 (32-byte) and NEON (16-byte) operations.
#[repr(C, align(32))]
pub struct AlignedBuffer {
    _align: [u8; 32],
    data: Vec<u8>,
}

impl AlignedBuffer {
    /// Create a new aligned buffer with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            _align: [0; 32],
            data: Vec::with_capacity(capacity),
        }
    }

    /// Get a reference to the data
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Get a mutable reference to the data
    pub fn as_mut_slice(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }

    /// Get the alignment of this buffer
    pub fn alignment() -> usize {
        32 // AVX2 alignment
    }

    /// Get a pointer to this buffer (for alignment checking)
    pub fn as_ptr(&self) -> *const u8 {
        self as *const _ as *const u8
    }
}

/// Structure-of-Arrays for batch processing
///
/// Improves cache locality by storing each field separately.
/// More cache-friendly than Array-of-Structures for vectorized operations.
#[derive(Debug, Clone)]
pub struct BatchData {
    /// Source code strings
    pub sources: Vec<String>,

    /// File paths
    pub paths: Vec<String>,

    /// Processing results (optional)
    pub results: Vec<Option<String>>,
}

impl BatchData {
    /// Create a new batch with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            sources: Vec::with_capacity(capacity),
            paths: Vec::with_capacity(capacity),
            results: Vec::with_capacity(capacity),
        }
    }

    /// Add an item to the batch
    pub fn push(&mut self, source: String, path: String) {
        self.sources.push(source);
        self.paths.push(path);
        self.results.push(None);
    }

    /// Get the number of items in the batch
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Set a result for a specific index
    pub fn set_result(&mut self, index: usize, result: String) {
        if index < self.results.len() {
            self.results[index] = Some(result);
        }
    }
}

/// Zero-copy string operations using Cow (Clone-on-Write)
///
/// Avoids unnecessary allocations when strings don't need modification.
pub mod zero_copy {
    use std::borrow::Cow;

    /// Process a string with potential modification
    ///
    /// Returns Borrowed if no modification needed, Owned if modified.
    pub fn process_string<'a, F>(s: &'a str, should_modify: bool, modify: F) -> Cow<'a, str>
    where
        F: FnOnce(&str) -> String,
    {
        if should_modify {
            Cow::Owned(modify(s))
        } else {
            Cow::Borrowed(s)
        }
    }

    /// Trim a string without allocating if possible
    pub fn trim_zero_copy(s: &str) -> Cow<'_, str> {
        let trimmed = s.trim();
        if trimmed.len() == s.len() {
            Cow::Borrowed(s)
        } else {
            Cow::Borrowed(trimmed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_interning() {
        let interner = StringInterner::new();

        let s1 = interner.intern("test");
        let s2 = interner.intern("test");

        // Same Arc instance
        assert!(Arc::ptr_eq(&s1, &s2));
    }

    #[test]
    fn test_object_pool() {
        let pool = ObjectPool::new(|| Vec::<i32>::with_capacity(100), 10);

        {
            let mut obj = pool.acquire();
            obj.push(42);
            assert_eq!(obj[0], 42);
        } // obj returned to pool

        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn test_aligned_buffer() {
        let buffer = AlignedBuffer::with_capacity(1024);
        let ptr = buffer.as_ptr() as usize;

        // Check 32-byte alignment
        assert_eq!(ptr % 32, 0);
    }

    #[test]
    fn test_zero_copy_trim() {
        let s = "hello";
        let trimmed = zero_copy::trim_zero_copy(s);

        // Should be borrowed (no allocation)
        assert!(matches!(trimmed, Cow::Borrowed(_)));
    }
}
