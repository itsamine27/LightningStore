
This project investigates how to achieve high-throughput concurrent reads and writes across nested data structures without incurring heavy global lock contention.
 Concepts & Key Takeaways

    Sharded Concurrency with DashMap: Avoids standard Mutex<HashMap> global lock bottlenecks by sharding the map across multiple internal locks.

    Recursive / Nested Data Models: Implements table-in-table support using atomic reference counting (Arc<DashMap<...>>) inside a custom Value enum.

    Zero-Copy Concurrent Views: Shared inner maps can be updated or read concurrently across spawned tokio tasks without requiring expensive deep clones.

    Idiomatic Type Coercion: Leverages Rust's From trait implementations and custom accessor methods (as_i64, as_str, as_map) for safe type casting.

 Data Architecture

The core data structure relies on a top-level shared store containing strongly-typed Value variants:
Plaintext

Store (Arc<DashMap<String, Value>>)
 ├── "count"    ──> Value::Int(42)
 ├── "greeting" ──> Value::Str("hello")
 └── "my_table" ──> Value::Map( Arc<DashMap<String, Value>> )
                           ├── "key1" ──> Value::Int(100)
                           └── "key2" ──> Value::Str("value")

Rust

pub enum Value {
    Map(Arc<DashMap<String, Value>>),
    Str(String),
    Int(i64),
}

 Async Concurrency Pattern

The project demonstrates non-blocking multi-reader / multi-writer interactions running concurrently on the Tokio async runtime:

    Writers: Spawned tasks dynamically swap or insert nested table instances with minimal overhead using Arc::clone.

    Readers: Concurrent tasks read top-level keys and safely iterate over nested maps without blocking parallel execution paths.

 Performance Verification

The implementation includes an integrated multi-threaded stress test simulating a high-throughput load (90% Reads / 10% Writes) across 10 concurrent async workers (1,000 total operations).
Running the Test
Bash

cargo test test_thousand_queries_performance -- --nocapture

Benchmark Metrics
Plaintext

=========== PERF RESULT ===========
Time taken for 1,000 queries: ~250µs - 350µs
Average time per query:       ~250ns - 350ns
===================================

    Note: Microsecond-level latencies validate that sharded map operations remain extremely cheap under high read contention, avoiding task starvation on the thread pool.
