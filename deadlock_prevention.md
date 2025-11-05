# Deadlock Prevention in Concurrent Code

## What is a Deadlock?

A **deadlock** occurs when two or more tasks are waiting for each other to release resources, creating a circular dependency where none can proceed.

## The Problem: Original Code with Deadlock

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::runtime::Runtime;

let rt = Runtime::new().unwrap();

rt.block_on(async {
    let resource_a = Arc::new(Mutex::new(1));
    let resource_b = Arc::new(Mutex::new(2));
    
    let a1 = resource_a.clone();
    let b1 = resource_b.clone();
    let task1 = tokio::spawn(async move {
        let _lock_a = a1.lock().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let _lock_b = b1.lock().await;  // ← Waiting for B
        println!("Task 1 completed");
    });
    
    let a2 = resource_a.clone();
    let b2 = resource_b.clone();
    let task2 = tokio::spawn(async move {
        let _lock_b = b2.lock().await;  // ← Locks B first (different order!)
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let _lock_a = a2.lock().await;  // ← Waiting for A
        println!("Task 2 completed");
    });
    
    task1.await.unwrap();
    task2.await.unwrap();
});
```

### Why This Deadlocks

**Different lock ordering creates circular dependency:**

```
Task 1: Lock A → [sleep] → Try to lock B (waiting...)
Task 2: Lock B → [sleep] → Try to lock A (waiting...)

Result: Both tasks wait forever (circular dependency)
```

**Timeline:**
```
Time:     0ms                    10ms
Task 1:   [Lock A acquired]──────[Waiting for B...]──(deadlock)
Task 2:   [Lock B acquired]──────[Waiting for A...]──(deadlock)
```

## The Solution: Consistent Lock Ordering

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::runtime::Runtime;

let rt = Runtime::new().unwrap();

rt.block_on(async {
    let resource_a = Arc::new(Mutex::new(1));
    let resource_b = Arc::new(Mutex::new(2));
    
    let a1 = resource_a.clone();
    let b1 = resource_b.clone();
    let task1 = tokio::spawn(async move {
        let _lock_a = a1.lock().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let _lock_b = b1.lock().await;
        println!("Task 1 completed");
    });
    
    let a2 = resource_a.clone();
    let b2 = resource_b.clone();
    let task2 = tokio::spawn(async move {
        // FIX: Lock in the SAME ORDER as task1 (A then B)
        let _lock_a = a2.lock().await;  // Changed from b2 to a2
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let _lock_b = b2.lock().await;  // Changed from a2 to b2
        println!("Task 2 completed");
    });
    
    task1.await.unwrap();
    task2.await.unwrap();
});
```

### Why This Works

**Same lock ordering prevents circular dependency:**

```
Task 1: Lock A → [sleep] → Lock B ✓
Task 2: Lock A → [sleep] → Lock B ✓

Result: Task 2 waits for Task 1 to finish, then proceeds
```

**Timeline:**
```
Time:     0ms                    10ms                   20ms
Task 1:   [Lock A]───────────────[Lock B]──────────────[Done]
Task 2:   [Wait for A...........]──[Lock A]─[Lock B]───[Done]
```

## The Fix Explained

### Rule: Always Acquire Locks in the Same Order

```rust
// ✅ GOOD: Both tasks lock A, then B
Task 1: A → B
Task 2: A → B

// ❌ BAD: Different order creates deadlock potential
Task 1: A → B
Task 2: B → A
```

## Alternative Solutions

### **Solution 2: Use `try_lock()` with Timeout**
```rust
use tokio::time::{timeout, Duration};

let task2 = tokio::spawn(async move {
    let _lock_b = b2.lock().await;
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    // Try to acquire with timeout
    match timeout(Duration::from_secs(1), a2.lock()).await {
        Ok(_lock_a) => println!("Task 2 completed"),
        Err(_) => {
            println!("Task 2 timed out, releasing locks");
            // Could retry or handle gracefully
        }
    }
});
```

**Pros:** Detects deadlocks and allows recovery  
**Cons:** More complex, may waste time on timeouts

### **Solution 3: Combine Resources into One Lock**
```rust
// Instead of two separate locks
let resources = Arc::new(Mutex::new((1, 2)));

let task1 = tokio::spawn(async move {
    let mut data = resources.lock().await;
    data.0 += 1;  // Modify resource A
    data.1 += 1;  // Modify resource B
});
```

**Pros:** Impossible to deadlock (only one lock)  
**Cons:** Reduces concurrency (can't access A and B independently)

### **Solution 4: Acquire Both Locks Atomically**
```rust
let task2 = tokio::spawn(async move {
    // Acquire both locks atomically using tokio::join!
    let (_lock_a, _lock_b) = tokio::join!(
        a2.lock(),
        b2.lock()
    );
    println!("Task 2 completed");
});
```

**Pros:** Simple syntax  
**Cons:** Still subject to deadlock if other tasks use different ordering

## Deadlock Visualization

### Deadlock State (Original Code)
```
┌─────────┐           ┌─────────┐
│ Task 1  │──holds───→│ Lock A  │
│         │           │         │
│         │←──waits───│ Lock B  │
└─────────┘           └─────────┘
     ↑                     ↓
     │                     │
  waits                 holds
     │                     │
     ↓                     ↑
┌─────────┐           ┌─────────┐
│ Task 2  │──holds───→│ Lock B  │
│         │           │         │
│         │←──waits───│ Lock A  │
└─────────┘           └─────────┘

Circular dependency: Task 1 → Lock B → Task 2 → Lock A → Task 1
```

### Fixed State (Consistent Ordering)
```
┌─────────┐           ┌─────────┐
│ Task 1  │──holds───→│ Lock A  │
│         │           │         │
│         │──holds───→│ Lock B  │
└─────────┘           └─────────┘
     
┌─────────┐           
│ Task 2  │──waits───→│ Lock A  │
│         │           │(held by Task 1)
└─────────┘           

No circular dependency: Task 2 simply waits for Task 1 to finish
```

## Deadlock Prevention Best Practices

1. **Lock ordering** - Always acquire locks in a consistent global order
2. **Minimize lock scope** - Hold locks for the shortest time possible
3. **Avoid nested locks** - If possible, redesign to use a single lock
4. **Use timeouts** - Detect and recover from potential deadlocks
5. **Lock-free alternatives** - Consider using channels or other lock-free structures

## Example: Global Lock Order Convention

```rust
// Define a global ordering for all locks in your system
// Rule: Always lock in alphabetical/numerical order

let lock_1 = Arc::new(Mutex::new(data1));
let lock_2 = Arc::new(Mutex::new(data2));
let lock_3 = Arc::new(Mutex::new(data3));

// ✅ Always lock in order: 1 → 2 → 3
async fn good_function() {
    let _l1 = lock_1.lock().await;
    let _l2 = lock_2.lock().await;
    let _l3 = lock_3.lock().await;
    // Do work
}

// ❌ Never lock out of order: 3 → 1 → 2 (BAD!)
async fn bad_function() {
    let _l3 = lock_3.lock().await;
    let _l1 = lock_1.lock().await;  // Potential deadlock!
    let _l2 = lock_2.lock().await;
}
```

## Key Takeaway

The **simplest and most reliable** solution to prevent deadlocks is **consistent lock ordering**. Establish a global order for all locks in your system and always acquire them in that order. This makes deadlocks impossible by preventing circular wait conditions.
