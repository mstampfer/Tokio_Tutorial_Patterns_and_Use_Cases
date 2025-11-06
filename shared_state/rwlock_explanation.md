# How RwLock Enables Multiple Concurrent Readers

This code demonstrates how `RwLock` (Read-Write Lock) enables **multiple concurrent readers** while maintaining exclusive access for writers. Here's how it works:

## Complete Code Example

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let data = Arc::new(RwLock::new(vec![1, 2, 3]));
    let mut handles = vec![];
    
    // Spawn 5 reader tasks
    for i in 0..5 {
        let data = Arc::clone(&data);
        let handle = tokio::spawn(async move {
            let vec = data.read().await;
            println!("Reader {} sees: {:?}", i, *vec);
        });
        handles.push(handle);
    }
    
    // Spawn 1 writer task
    let data_clone = Arc::clone(&data);
    let writer = tokio::spawn(async move {
        let mut vec = data_clone.write().await;
        vec.push(4);
        println!("Writer added element");
    });
    handles.push(writer);
    
    for handle in handles {
        handle.await.unwrap();
    }
}
```

## Key Concept: RwLock's Two Lock Types

`RwLock` provides two types of locks:
- **Read locks** (`read().await`) - Multiple tasks can hold these simultaneously
- **Write locks** (`write().await`) - Only one task can hold this, and only when no readers exist

## How the Code Works

**1. Shared Ownership with Arc**
```rust
let data = Arc::new(RwLock::new(vec![1, 2, 3]));
```
`Arc` (Atomic Reference Counting) allows multiple tasks to share ownership of the same `RwLock`-protected data.

**2. Multiple Concurrent Readers**
```rust
for i in 0..5 {
    let data = Arc::clone(&data);
    let handle = tokio::spawn(async move {
        let vec = data.read().await;  // ← Multiple readers can acquire this
        println!("Reader {} sees: {:?}", i, *vec);
    });
}
```
All 5 reader tasks can call `data.read().await` **at the same time**. The `RwLock` allows this because read operations don't modify the data, so there's no risk of data races.

**3. Exclusive Writer**
```rust
let mut vec = data_clone.write().await;  // ← Waits until no readers/writers
vec.push(4);
```
The writer must wait until:
- All existing read locks are released
- No other write lock is held

Only then can it acquire exclusive access to modify the data.

## Why This is Efficient

Without `RwLock`, you'd use a regular `Mutex`, which would force readers to wait for each other even though they're just reading. `RwLock` recognizes that **reading is safe to parallelize**, so those 5 reader tasks can execute simultaneously, improving performance when reads outnumber writes.

The trade-off is that writers may wait longer if many readers are active, but this is ideal for read-heavy workloads.

