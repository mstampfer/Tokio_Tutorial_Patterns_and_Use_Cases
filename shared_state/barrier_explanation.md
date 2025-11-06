# How Barriers Work for Task Synchronization

## What's a Barrier?

A **Barrier** is a synchronization point where tasks must **wait** until a specified number of tasks reach that point, then **all proceed together**.

## Complete Code Example

```rust
:dep tokio = { version = "1", features = ["full"] }
```

```rust
use std::sync::Arc;
use tokio::sync::Barrier;
use tokio::time::{sleep, Duration};
use tokio::runtime::Runtime;

let rt = Runtime::new().unwrap();

rt.block_on(async {
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = vec![];
    
    for i in 0..3 {
        let b = barrier.clone();
        let handle = tokio::spawn(async move {
            println!("Task {} doing setup", i);
            sleep(Duration::from_millis(i * 50)).await;
            
            // Wait at the barrier for all tasks
            b.wait().await;
            
            println!("Task {} proceeding", i);
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
});
```

## Setup: Create Barrier for 3 Tasks

```rust
let barrier = Arc::new(Barrier::new(3));
//                                  ^
//                                  Requires 3 tasks to reach barrier
```

## Execution Timeline

```
Time →    0ms      50ms      100ms     100ms+

Task 0:   setup──→[WAIT at barrier..................][PROCEED]
Task 1:   setup────→[WAIT at barrier..............][PROCEED]
Task 2:   setup──────→[WAIT at barrier..........][PROCEED]
                                                  ↑
                                    All 3 reached barrier → release all
```

## Step-by-Step Breakdown

### **Phase 1: Setup (Different Speeds)**
```rust
println!("Task {} doing setup", i);
sleep(Duration::from_millis(i * 50)).await;
```
- Task 0: sleeps 0ms (finishes first)
- Task 1: sleeps 50ms
- Task 2: sleeps 100ms (finishes last)

### **Phase 2: Wait at Barrier**
```rust
b.wait().await;
```
- **Task 0** arrives first → **waits**
- **Task 1** arrives second → **waits**
- **Task 2** arrives third → **barrier count reached!**
- **All 3 tasks released simultaneously**

### **Phase 3: Proceed Together**
```rust
println!("Task {} proceeding", i);
```
All tasks continue execution together

## Sample Output

```
Task 0 doing setup
Task 1 doing setup      ← 50ms later
Task 2 doing setup      ← 100ms later
Task 0 proceeding       ← All three print at approximately same time
Task 1 proceeding       ← (order may vary due to scheduling)
Task 2 proceeding
```

**Key observation:** Even though Task 0 finishes setup early, it waits for Tasks 1 and 2 before proceeding.

## Visualization

```
Without Barrier:
Task 0: [setup 0ms]────→[proceed immediately]
Task 1: [setup 50ms]───────→[proceed immediately]
Task 2: [setup 100ms]──────────→[proceed immediately]
        ↑ Tasks proceed at different times

With Barrier:
Task 0: [setup 0ms]────→[WAIT.........................]→[proceed]
Task 1: [setup 50ms]───────→[WAIT.................]→[proceed]
Task 2: [setup 100ms]──────────→[WAIT........]→[proceed]
                                                ↑ All proceed together
```

## Common Use Cases

### **1. Coordinated Start**
```rust
// All tasks start benchmarking at the same time
barrier.wait().await;
start_benchmark();
```

### **2. Phase Synchronization**
```rust
// Phase 1: Load data
load_my_data().await;
barrier.wait().await;  // Wait for all to load

// Phase 2: Process data (only after all loaded)
process_data().await;
barrier.wait().await;  // Wait for all to process

// Phase 3: Output results
output_results().await;
```

### **3. Parallel Algorithm Checkpoints**
```rust
// All workers must finish current iteration before starting next
for iteration in 0..10 {
    do_work(iteration).await;
    barrier.wait().await;  // Synchronize between iterations
}
```

## Key Points

1. **Count must match** - If you create `Barrier::new(3)`, exactly 3 tasks must call `.wait()`, or they'll wait forever
2. **Reusable** - After all tasks pass through, the barrier resets and can be used again
3. **Returns `BarrierWaitResult`** - Can check if this task was the "leader" (last to arrive)

```rust
let result = barrier.wait().await;
if result.is_leader() {
    println!("I was the last one to arrive!");
}
```

## Non-Deterministic Ordering After Barrier

After the barrier releases all tasks, the order in which they execute is **non-deterministic**:

```
Run 1:
Task 0 proceeding
Task 1 proceeding
Task 2 proceeding

Run 2:
Task 2 proceeding
Task 1 proceeding
Task 0 proceeding
```

This is normal - the Tokio scheduler decides which task runs first after they're all released. The important guarantee is that **all "proceeding" messages happen after all "setup" messages**.

## Barrier vs Other Synchronization Primitives

| Primitive | Purpose | Behavior |
|-----------|---------|----------|
| **Barrier** | Wait for N tasks to reach a point | All proceed together |
| **Semaphore** | Limit concurrent access | Up to N can proceed |
| **Mutex** | Exclusive access | Only 1 can access at a time |
| **Notify** | Signal between tasks | Wake up waiting tasks |

The barrier ensures **deterministic synchronization** - all tasks reach the same point before any can proceed, making it perfect for coordinating parallel work phases.
