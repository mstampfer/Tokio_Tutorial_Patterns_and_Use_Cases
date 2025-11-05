# How Semaphores Limit Concurrent Access

## What's a Semaphore?

A semaphore is a synchronization primitive that limits the number of tasks that can access a resource **simultaneously**. Think of it as having a fixed number of "permits" or "tickets."

## Code Example

```rust
:dep tokio = { version = "1", features = ["full"] }
```

```rust
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};
use tokio::runtime::Runtime;

let rt = Runtime::new().unwrap();

rt.block_on(async {
    // Allow only 2 concurrent tasks
    let semaphore = Arc::new(Semaphore::new(2));
    let mut handles = vec![];
    
    for i in 0..5 {
        let permit = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = permit.acquire().await;
            println!("Task {} started", i);
            sleep(Duration::from_millis(100)).await;
            println!("Task {} finished", i);
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
});
```

## Step-by-Step Breakdown

### 1. **Create Semaphore with 2 Permits**
```rust
let semaphore = Arc::new(Semaphore::new(2));
//                                      ^
//                                      Only 2 tasks can run at once
```
- Creates a semaphore with **2 permits**
- Wrapped in `Arc` so multiple tasks can share it

### 2. **Spawn 5 Tasks (but only 2 run at once)**
```rust
for i in 0..5 {
    let permit = semaphore.clone();  // Each task gets Arc clone
    let handle = tokio::spawn(async move {
        let _permit = permit.acquire().await;  // ← KEY: Acquire a permit
        //    ^
        //    When dropped, permit is returned to semaphore
        
        println!("Task {} started", i);
        sleep(Duration::from_millis(100)).await;  // Simulated work
        println!("Task {} finished", i);
        
        // _permit dropped here, releases permit back to semaphore
    });
    handles.push(handle);
}
```

### 3. **How `acquire()` Works**
```rust
let _permit = permit.acquire().await;
```
- **If permits available** (< 2 tasks running): Immediately acquires a permit and continues
- **If no permits available** (2 tasks already running): Task **waits** until a permit is released
- When `_permit` goes out of scope, the permit is **automatically returned** to the semaphore

## Execution Timeline

```
Time →        0ms        100ms       200ms       300ms
         
Task 0:  [acquired]───work───[released]
Task 1:  [acquired]───work───[released]
Task 2:  [waiting...]─[acquired]───work───[released]
Task 3:  [waiting...]─[acquired]───work───[released]
Task 4:  [waiting...............]─[acquired]───work───[released]

Permits:  2 → 0     → 2 → 0      → 2 → 1      → 2
```

**What happens:**
1. Tasks 0 & 1 start immediately (2 permits taken)
2. Tasks 2, 3, 4 wait (no permits available)
3. After 100ms, Task 0 finishes → releases permit → Task 2 starts
4. After 100ms, Task 1 finishes → releases permit → Task 3 starts
5. After 100ms, Task 2 finishes → releases permit → Task 4 starts
6. Continue until all tasks complete

## Sample Output

```
Task 0 started
Task 1 started
Task 0 finished       ← After 100ms
Task 1 finished       ← After 100ms
Task 2 started        ← Now permit available
Task 3 started        ← Now permit available
Task 2 finished       ← After another 100ms
Task 3 finished
Task 4 started        ← Finally gets a permit
Task 4 finished
```

## Key Points

### **Automatic Permit Release (RAII)**
```rust
let _permit = permit.acquire().await;
// Underscore prefix means "I'm not using this variable, but keep it alive"
```
- The permit is held as long as `_permit` is in scope
- When `_permit` is dropped (end of scope), permit automatically returns to semaphore
- No need to manually call "release"

### **Manual Release (if needed)**
```rust
let permit = semaphore.acquire().await.unwrap();
// Do some work
permit.forget();  // Release permit early without dropping
```

## Semaphore vs. Mutex

| Feature | Mutex | Semaphore |
|---------|-------|-----------|
| **Concurrent access** | 1 task only | N tasks (configurable) |
| **Use case** | Protect shared data | Limit resource usage |
| **Permits** | 1 (binary lock) | N (counting) |
| **Typical pattern** | Exclusive access to data | Throttling/rate limiting |

## Visual Comparison

### Mutex (1 permit):
```
Time →
Task 1: [──work──]
Task 2:            [──work──]
Task 3:                       [──work──]

Only one task at a time
```

### Semaphore with 2 permits:
```
Time →
Task 1: [──work──]
Task 2: [──work──]
Task 3:            [──work──]
Task 4:            [──work──]
Task 5:                       [──work──]

Two tasks at a time
```

## Common Use Cases

### **1. Rate Limiting**
```rust
let semaphore = Arc::new(Semaphore::new(10));
// Only 10 API calls at once

for request in requests {
    let sem = semaphore.clone();
    tokio::spawn(async move {
        let _permit = sem.acquire().await;
        make_api_call(request).await;
    });
}
```

### **2. Connection Pooling**
```rust
let semaphore = Arc::new(Semaphore::new(5));
// Only 5 database connections simultaneously

async fn query_database(sem: Arc<Semaphore>, query: Query) {
    let _permit = sem.acquire().await;
    let connection = get_db_connection().await;
    connection.execute(query).await;
}
```

### **3. Resource Throttling**
```rust
let semaphore = Arc::new(Semaphore::new(3));
// Only 3 file downloads at once

for file in files {
    let sem = semaphore.clone();
    tokio::spawn(async move {
        let _permit = sem.acquire().await;
        download_file(file).await;
    });
}
```

### **4. Worker Pool**
```rust
let semaphore = Arc::new(Semaphore::new(num_cpus::get()));
// Limit concurrent work to number of CPU cores

for task in tasks {
    let sem = semaphore.clone();
    tokio::spawn(async move {
        let _permit = sem.acquire().await;
        cpu_intensive_work(task).await;
    });
}
```

## Advanced: Try Acquire

```rust
// Try to acquire without waiting
match semaphore.try_acquire() {
    Ok(permit) => {
        println!("Got permit!");
        do_work().await;
    }
    Err(_) => {
        println!("No permits available, skipping");
    }
}
```

## Advanced: Acquire Multiple Permits

```rust
// Acquire 3 permits at once
let _permits = semaphore.acquire_many(3).await;
// Do work that needs 3 resources
// All 3 permits released when _permits is dropped
```

## Semaphore State Diagram

```
Initial State: 2 permits available
[◯ ◯]

Task A acquires:
[◉ ◯] ← 1 permit taken, 1 available

Task B acquires:
[◉ ◉] ← 0 permits available

Task C tries to acquire:
[◉ ◉] ← Task C waits...

Task A finishes:
[◯ ◉] ← 1 permit released → Task C wakes up and acquires
[◉ ◉] ← Task C now running
```

## Key Takeaway

The semaphore ensures that **no matter how many tasks** you spawn, only a **limited number** can execute their critical section simultaneously, preventing resource exhaustion. It's perfect for:

- Rate limiting external API calls
- Managing connection pools
- Throttling resource-intensive operations
- Controlling concurrency levels

The automatic permit release through RAII (dropping the `_permit` variable) makes it easy to use correctly without forgetting to release resources.
