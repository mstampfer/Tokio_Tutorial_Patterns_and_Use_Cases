# Understanding the Tokio Runtime Function

This function creates a **custom Tokio runtime with 2 worker threads** and then runs an async function on it.

## The Code

```rust
use tokio::runtime::Builder;

async fn async_work() {
    println!("Working in async context");
    println!("Current thread: {:?}", std::thread::current().id());
}

fn main() {
    let rt = Builder::new_multi_thread()
        // Set worker threads to 2
        .worker_threads(2)
        .build()
        .unwrap();
    
    rt.block_on(async_work());
}
```

## Breaking it Down

### What Each Part Does

1. **`Builder::new_multi_thread()`** - Creates a builder for a multi-threaded runtime (allows parallel async task execution)

2. **`.worker_threads(2)`** - Configures the runtime to use exactly **2 worker threads** instead of the default (which would be the number of CPU cores)

3. **`.build().unwrap()`** - Builds the runtime, panicking if creation fails

4. **`rt.block_on(async_work())`** - Blocks the main thread and runs the `async_work()` function to completion

### What Happens When You Run It

```
Working in async context
Current thread: ThreadId(2)  // Or ThreadId(3), depending on which worker picks it up
```

The output shows:
- The async function executes successfully
- It runs on one of the 2 worker threads (not the main thread)
- The thread ID will be 2 or 3 (thread 1 is typically the main thread)

## Key Points

- **Limits parallelism**: Only 2 async tasks can run truly concurrently (vs. default which uses all CPU cores)
- **Blocks main thread**: The main thread waits until `async_work()` completes
- **Custom configuration**: Useful when you want to control resource usage or limit concurrency

## Comparison

```rust
// Default (uses all CPU cores, e.g., 8 threads on 8-core CPU)
Runtime::new()

// This code (uses exactly 2 threads)
Builder::new_multi_thread()
    .worker_threads(2)
    .build()
```

## When to Use This Pattern

This pattern is useful for:

- **Testing** - Predictable thread behavior makes tests more reliable
- **Resource-constrained environments** - Limit CPU usage on shared systems
- **Controlled concurrency** - Prevent overwhelming external services
- **Debugging** - Easier to reason about with fewer threads
- **Embedded systems** - Match thread count to available cores

## Example with Multiple Tasks

```rust
fn main() {
    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .unwrap();
    
    rt.block_on(async {
        // These 4 tasks will be scheduled on 2 worker threads
        let task1 = tokio::spawn(async { println!("Task 1") });
        let task2 = tokio::spawn(async { println!("Task 2") });
        let task3 = tokio::spawn(async { println!("Task 3") });
        let task4 = tokio::spawn(async { println!("Task 4") });
        
        // Wait for all to complete
        let _ = tokio::join!(task1, task2, task3, task4);
    });
}
```

In this example, even though we spawn 4 tasks, only 2 can run concurrently because we limited the runtime to 2 worker threads.