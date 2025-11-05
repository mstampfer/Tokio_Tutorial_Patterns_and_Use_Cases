# How Arc Shares Immutable Data Across Multiple Tasks

This code demonstrates **reference-counted thread-safe sharing** of immutable data using `Arc` (Atomic Reference Counted). Here's how it works:

## Key Mechanisms

**1. Creating the Shared Data**
```rust
let data = Arc::new(vec![1, 2, 3, 4, 5]);
```
- Wraps the vector in an `Arc`, which enables multiple ownership
- The data is allocated on the heap with a reference counter

**2. Cloning the Arc (Not the Data)**
```rust
let data_clone = Arc::clone(&data);
```
- Creates a new `Arc` pointer to the *same* underlying data
- Increments the reference counter atomically (thread-safe)
- **Important**: Only the pointer is cloned, not the vector itself—all tasks share one copy of the data

**3. Moving into Async Tasks**
```rust
let handle = tokio::spawn(async move {
    println!("Task {} sees: {:?}", i, data_clone);
});
```
- The `move` keyword transfers ownership of `data_clone` into the task
- Each task gets its own `Arc` pointer, but all point to the same vector
- When each task completes, its `Arc` is dropped and the counter decrements

## Why This Works

- **Immutability**: `Arc` only allows shared *read* access (like `&T`), preventing data races
- **Atomic counting**: Reference count updates are thread-safe
- **Automatic cleanup**: When the last `Arc` is dropped (after all tasks finish), the vector is deallocated

## Output Example
```
Task 0 sees: [1, 2, 3, 4, 5]
Task 1 sees: [1, 2, 3, 4, 5]
Task 2 sees: [1, 2, 3, 4, 5]
```
(Order may vary due to concurrent execution)

## Full Code Example

```rust
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let data = Arc::new(vec![1, 2, 3, 4, 5]);
    
    let mut handles = vec![];
    
    for i in 0..3 {
        let data_clone = Arc::clone(&data);
        let handle = tokio::spawn(async move {
            println!("Task {} sees: {:?}", i, data_clone);
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
}
```

## Note on Mutability

If you needed to *mutate* the data across tasks, you'd combine `Arc` with a `Mutex` or `RwLock`: `Arc<Mutex<Vec<i32>>>`.
