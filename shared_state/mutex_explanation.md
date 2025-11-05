# Mutex for Shared Mutable State in Rust

This code demonstrates safe concurrent access to shared mutable state using `Arc` (Atomic Reference Counting) and `Mutex` (mutual exclusion). Here's how it works:

## The Core Pattern: Arc + Mutex

**`Arc<Mutex<i32>>`** creates a thread-safe reference-counted pointer to a mutex-protected integer:
- **`Arc`** allows multiple tasks to own the same data by tracking references and cleaning up when the last reference is dropped
- **`Mutex`** ensures only one task can access the data at a time, preventing data races

## Step-by-Step Breakdown

**1. Initialization**
```rust
let counter = Arc::new(Mutex::new(0));
```
Creates a counter starting at 0, wrapped in a Mutex, wrapped in an Arc.

**2. Spawning Concurrent Tasks**
```rust
for _ in 0..10 {
    let counter = Arc::clone(&counter);
    let handle = tokio::spawn(async move { ... });
}
```
- `Arc::clone(&counter)` creates a new reference to the same underlying data (not a copy of the data itself)
- Each spawned task gets its own `Arc` reference but they all point to the same `Mutex<i32>`
- The `async move` block takes ownership of that cloned `Arc`

**3. Safe Mutation**
```rust
let mut num = counter.lock().await;
*num += 1;
```
- `.lock().await` acquires the mutex lock asynchronously
- If another task holds the lock, this task waits until the lock is available
- `num` is a `MutexGuard` that dereferences to the inner value
- When `num` goes out of scope, the lock is automatically released
- Only one task can hold the lock at a time, ensuring safe mutation

**4. Synchronization**
```rust
for handle in handles {
    handle.await.unwrap();
}
```
Waits for all spawned tasks to complete before printing the result.

## Why This Works

Without the Mutex, multiple tasks incrementing the counter simultaneously would cause a **data race** (undefined behavior). The Mutex serializes access—tasks may run concurrently, but they take turns modifying the counter. The result is always predictably 10, regardless of task scheduling order.
