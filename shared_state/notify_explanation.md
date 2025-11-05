# How Notify Works for Signaling Between Tasks

`Notify` is a simple, lightweight synchronization primitive for **signaling between tasks**. One task waits for a signal, another task sends it.

```rust
:dep tokio = { version = "1", features = ["full"] }
```

```rust
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{sleep, Duration};
use tokio::runtime::Runtime;

let rt = Runtime::new().unwrap();

rt.block_on(async {
    let notify = Arc::new(Notify::new());
    
    let notify_clone = notify.clone();
    let waiter = tokio::spawn(async move {
        println!("Waiting for notification...");
        notify_clone.notified().await;  // FIX: Actually wait for notification!
        println!("Received notification!");
    });
    
    let notifier = tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;
        println!("Sending notification");
        notify.notify_one();
    });
    
    waiter.await.unwrap();
    notifier.await.unwrap();
});
```

## How `Notify` Works

### Execution Timeline

```
Time →     0ms                    100ms

Waiter:    [Start waiting...]────→[Receives signal]→[Continue]
Notifier:  [Sleep 100ms.........]→[Send signal]
```

## Step-by-Step Breakdown

### **1. Create Shared Notify**
```rust
let notify = Arc::new(Notify::new());
```
- Creates a `Notify` instance
- Wrapped in `Arc` for sharing between tasks

### **2. Waiter Task - Blocks Until Notified**
```rust
let waiter = tokio::spawn(async move {
    println!("Waiting for notification...");
    notify_clone.notified().await;  // ← BLOCKS here until notified
    println!("Received notification!");
});
```

**What happens:**
- Prints "Waiting for notification..."
- Calls `.notified()` which returns a future
- `.await` suspends the task until someone calls `notify_one()` or `notify_waiters()`

### **3. Notifier Task - Sends Signal After Delay**
```rust
let notifier = tokio::spawn(async move {
    sleep(Duration::from_millis(100)).await;  // Simulate work
    println!("Sending notification");
    notify.notify_one();  // ← Wakes up ONE waiting task
});
```

**What happens:**
- Sleeps for 100ms
- Sends notification using `notify_one()`
- This wakes up the waiter task

## Sample Output

```
Waiting for notification...
Sending notification          ← 100ms later
Received notification!
```

## Notify Methods

### **`notify_one()`**
```rust
notify.notify_one();  // Wakes up ONE waiting task
```

### **`notify_waiters()`**
```rust
notify.notify_waiters();  // Wakes up ALL waiting tasks
```

## Example with Multiple Waiters

```rust
let notify = Arc::new(Notify::new());

// Spawn 3 waiters
for i in 0..3 {
    let n = notify.clone();
    tokio::spawn(async move {
        println!("Task {} waiting", i);
        n.notified().await;
        println!("Task {} notified", i);
    });
}

sleep(Duration::from_millis(100)).await;

// Wake up only ONE task
notify.notify_one();
// Output: Only one "Task X notified" message

sleep(Duration::from_millis(100)).await;

// Wake up ALL remaining tasks
notify.notify_waiters();
// Output: Two more "Task X notified" messages
```

## Common Use Cases

### **1. Signaling Completion**
```rust
let notify = Arc::new(Notify::new());

// Background worker
let n = notify.clone();
tokio::spawn(async move {
    expensive_computation().await;
    n.notify_one();  // Signal we're done
});

// Main task waits
notify.notified().await;
println!("Computation complete!");
```

### **2. Producer-Consumer Signaling**
```rust
let notify = Arc::new(Notify::new());
let data = Arc::new(Mutex::new(None));

// Producer
let (n, d) = (notify.clone(), data.clone());
tokio::spawn(async move {
    *d.lock().await = Some(42);
    n.notify_one();  // Signal data is ready
});

// Consumer
notify.notified().await;
let value = data.lock().await.unwrap();
println!("Got value: {}", value);
```

### **3. Shutdown Signal**
```rust
let shutdown = Arc::new(Notify::new());

// Worker loop
let s = shutdown.clone();
tokio::spawn(async move {
    loop {
        tokio::select! {
            _ = s.notified() => {
                println!("Shutting down...");
                break;
            }
            _ = do_work() => {}
        }
    }
});

// Trigger shutdown
shutdown.notify_waiters();
```

## Key Differences: `Notify` vs Other Primitives

| Primitive | Use Case | State |
|-----------|----------|-------|
| **Notify** | Simple signaling | No data, just wake-up |
| **Mutex** | Protect shared data | Holds mutable data |
| **Channel** | Pass messages | Queues data |
| **Semaphore** | Limit concurrency | Counts permits |

## Important: Lost Notifications

`Notify` doesn't queue notifications:

```rust
notify.notify_one();  // Send notification

// Later...
notify.notified().await;  // ❌ Will wait forever! Notification was already sent
```

To avoid this, call `.notified()` **before** the notification is sent, or use channels for queued messages.

The `Notify` primitive is perfect for simple, lightweight signaling where you just need to wake up waiting tasks without passing data.
