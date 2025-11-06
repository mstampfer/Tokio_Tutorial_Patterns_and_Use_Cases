# Using `tokio::select!` to Wait on Multiple Channels

## Overview

The `tokio::select!` macro allows you to wait on multiple async operations simultaneously and proceed with whichever completes first. It's essential for handling multiple channels, timeouts, and concurrent operations in Tokio applications.

## The Original Code (Already Correct!)

```rust
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let (tx1, mut rx1) = mpsc::channel(32);
    let (tx2, mut rx2) = mpsc::channel(32);
    
    tokio::spawn(async move {
        sleep(Duration::from_millis(50)).await;
        tx1.send("From channel 1").await.unwrap();
    });
    
    tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;
        tx2.send("From channel 2").await.unwrap();
    });
    
    for _ in 0..2 {
        tokio::select! {
            Some(msg) = rx1.recv() => {
                println!("Got from channel 1: {}", msg);
            }
            Some(msg) = rx2.recv() => {
                println!("Got from channel 2: {}", msg);
            }
        }
    }
}
```

**Expected Output:**
```
Got from channel 1: From channel 1
Got from channel 2: From channel 2
```

**Good news: This code is correct and works perfectly!**

However, there's an important behavioral nuance about `select!` that's worth understanding.

## How `tokio::select!` Works

### Basic Behavior

The `select!` macro:
1. **Polls all branches concurrently**: Checks if any futures are ready
2. **Picks the first ready branch**: Executes its code block
3. **Cancels other branches**: The other futures are dropped (this is safe)

### Key Characteristics

```rust
tokio::select! {
    result1 = future1 => { /* handle result1 */ }
    result2 = future2 => { /* handle result2 */ }
    result3 = future3 => { /* handle result3 */ }
}
```

- **Non-blocking**: Only waits until at least one branch is ready
- **Fair by default in Tokio 1.0+**: Randomly selects among ready branches
- **Cancel-safe**: Unselected branches are properly cancelled
- **Pattern matching**: You can match on `Option`, `Result`, etc.

## Important: Biased vs. Random Selection

### Default Behavior (Random/Fair - Tokio 1.0+)

By default in modern Tokio, `select!` uses **random selection** when multiple branches are ready:

```rust
// If both channels have messages, randomly picks one
tokio::select! {
    Some(msg) = rx1.recv() => println!("Channel 1: {}", msg),
    Some(msg) = rx2.recv() => println!("Channel 2: {}", msg),
}
```

This prevents starvation - one fast channel won't monopolize the selector.

### Biased Selection (Sequential Order)

You can explicitly enable **biased** selection to check branches in order:

```rust
tokio::select! {
    biased;  // Check branches in declaration order
    
    Some(msg) = rx1.recv() => println!("Channel 1: {}", msg),
    Some(msg) = rx2.recv() => println!("Channel 2: {}", msg),
}
```

With `biased;`, if both channels have messages, **channel 1 is always selected first**.

### When to Use Each

| Use Case | Selection Type | Why |
|----------|----------------|-----|
| Fair handling of multiple sources | Random (default) | Prevents starvation |
| Priority handling (high/low priority) | Biased | Process important messages first |
| Shutdown signals | Biased | Check shutdown before other work |
| Most applications | Random (default) | Generally the right choice |

## Complete Working Examples

### Example 1: Fair Selection (Default)

```rust
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let (tx1, mut rx1) = mpsc::channel(10);
    let (tx2, mut rx2) = mpsc::channel(10);
    
    // Fast producer on channel 1
    let producer1 = tokio::spawn(async move {
        for i in 0..5 {
            tx1.send(format!("Ch1-{}", i)).await.unwrap();
            sleep(Duration::from_millis(10)).await;
        }
    });
    
    // Fast producer on channel 2
    let producer2 = tokio::spawn(async move {
        for i in 0..5 {
            tx2.send(format!("Ch2-{}", i)).await.unwrap();
            sleep(Duration::from_millis(10)).await;
        }
    });
    
    // Consume from both channels fairly
    for _ in 0..10 {
        tokio::select! {
            Some(msg) = rx1.recv() => println!("Received: {}", msg),
            Some(msg) = rx2.recv() => println!("Received: {}", msg),
        }
    }
    
    producer1.await.unwrap();
    producer2.await.unwrap();
}
```

**Sample Output (order varies due to random selection):**
```
Received: Ch1-0
Received: Ch2-0
Received: Ch1-1
Received: Ch2-1
Received: Ch2-2
Received: Ch1-2
Received: Ch1-3
Received: Ch2-3
Received: Ch1-4
Received: Ch2-4
```

### Example 2: Biased Selection with Priorities

```rust
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let (priority_tx, mut priority_rx) = mpsc::channel(10);
    let (normal_tx, mut normal_rx) = mpsc::channel(10);
    
    // Send priority messages
    tokio::spawn(async move {
        for i in 0..3 {
            priority_tx.send(format!("PRIORITY-{}", i)).await.unwrap();
            sleep(Duration::from_millis(20)).await;
        }
    });
    
    // Send normal messages
    tokio::spawn(async move {
        for i in 0..5 {
            normal_tx.send(format!("Normal-{}", i)).await.unwrap();
            sleep(Duration::from_millis(10)).await;
        }
    });
    
    sleep(Duration::from_millis(100)).await; // Let messages accumulate
    
    // Process with priority
    for _ in 0..8 {
        tokio::select! {
            biased;  // Always check priority_rx first
            
            Some(msg) = priority_rx.recv() => {
                println!("🔴 {}", msg);
            }
            Some(msg) = normal_rx.recv() => {
                println!("⚪ {}", msg);
            }
        }
    }
}
```

**Output (priority messages always processed first):**
```
🔴 PRIORITY-0
🔴 PRIORITY-1
🔴 PRIORITY-2
⚪ Normal-0
⚪ Normal-1
⚪ Normal-2
⚪ Normal-3
⚪ Normal-4
```

### Example 3: Handling Channel Closure

```rust
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let (tx1, mut rx1) = mpsc::channel(32);
    let (tx2, mut rx2) = mpsc::channel(32);
    
    tokio::spawn(async move {
        tx1.send("Message 1").await.unwrap();
        sleep(Duration::from_millis(50)).await;
        // tx1 dropped here - channel 1 closes
    });
    
    tokio::spawn(async move {
        sleep(Duration::from_millis(30)).await;
        tx2.send("Message 2").await.unwrap();
        sleep(Duration::from_millis(30)).await;
        tx2.send("Message 3").await.unwrap();
        // tx2 dropped here - channel 2 closes
    });
    
    let mut ch1_open = true;
    let mut ch2_open = true;
    
    while ch1_open || ch2_open {
        tokio::select! {
            result = rx1.recv(), if ch1_open => {
                match result {
                    Some(msg) => println!("Channel 1: {}", msg),
                    None => {
                        println!("Channel 1 closed");
                        ch1_open = false;
                    }
                }
            }
            result = rx2.recv(), if ch2_open => {
                match result {
                    Some(msg) => println!("Channel 2: {}", msg),
                    None => {
                        println!("Channel 2 closed");
                        ch2_open = false;
                    }
                }
            }
        }
    }
    
    println!("Both channels closed");
}
```

**Output:**
```
Channel 1: Message 1
Channel 2: Message 2
Channel 1 closed
Channel 2: Message 3
Channel 2 closed
Both channels closed
```

### Example 4: Select with Timeout

```rust
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, timeout};

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(32);
    
    tokio::spawn(async move {
        sleep(Duration::from_millis(200)).await;
        tx.send("Late message").await.unwrap();
    });
    
    tokio::select! {
        Some(msg) = rx.recv() => {
            println!("Received: {}", msg);
        }
        _ = sleep(Duration::from_millis(100)) => {
            println!("Timed out waiting for message");
        }
    }
}
```

**Output:**
```
Timed out waiting for message
```

### Example 5: Disable Branch with Condition

```rust
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let (tx1, mut rx1) = mpsc::channel(32);
    let (tx2, mut rx2) = mpsc::channel(32);
    
    tokio::spawn(async move {
        for i in 0..5 {
            tx1.send(i).await.unwrap();
            sleep(Duration::from_millis(50)).await;
        }
    });
    
    tokio::spawn(async move {
        for i in 0..5 {
            tx2.send(i * 10).await.unwrap();
            sleep(Duration::from_millis(50)).await;
        }
    });
    
    let mut paused = false;
    
    for _ in 0..10 {
        tokio::select! {
            Some(msg) = rx1.recv() => {
                println!("Channel 1: {}", msg);
                if msg == 2 {
                    paused = true;
                    println!("Pausing channel 2");
                }
            }
            Some(msg) = rx2.recv(), if !paused => {
                println!("Channel 2: {}", msg);
            }
        }
    }
}
```

**Output:**
```
Channel 1: 0
Channel 2: 0
Channel 1: 1
Channel 2: 10
Channel 1: 2
Pausing channel 2
Channel 1: 3
Channel 1: 4
```

## Advanced Patterns

### Pattern 1: Select with else (All Branches Pending)

```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<i32>(32);
    
    let mut count = 0;
    
    loop {
        tokio::select! {
            Some(msg) = rx.recv() => {
                println!("Received: {}", msg);
            }
            else => {
                println!("No messages available");
                count += 1;
                if count > 3 {
                    break;
                }
            }
        }
    }
}
```

### Pattern 2: Combining Multiple Operations

```rust
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, interval};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel(32);
    let mut tick = interval(Duration::from_secs(1));
    
    tokio::spawn(async move {
        for i in 0..5 {
            sleep(Duration::from_millis(500)).await;
            let _ = tx.send(i).await;
        }
    });
    
    loop {
        tokio::select! {
            Some(msg) = rx.recv() => {
                println!("Message: {}", msg);
            }
            _ = tick.tick() => {
                println!("Tick!");
            }
            _ = signal::ctrl_c() => {
                println!("Shutdown signal received");
                break;
            }
        }
    }
    
    Ok(())
}
```

## Common Patterns and Best Practices

### ✅ Do This: Match on Option/Result

```rust
tokio::select! {
    Some(msg) = rx.recv() => {
        println!("Got: {}", msg);
    }
    None => {
        println!("Channel closed");
    }
}
```

### ✅ Do This: Use Conditions to Disable Branches

```rust
tokio::select! {
    Some(msg) = rx1.recv(), if !paused => {
        println!("Channel 1: {}", msg);
    }
    Some(msg) = rx2.recv() => {
        println!("Channel 2: {}", msg);
    }
}
```

### ❌ Avoid: Blocking Operations in Branches

```rust
// ❌ Bad - blocks the executor
tokio::select! {
    Some(msg) = rx.recv() => {
        std::thread::sleep(Duration::from_secs(1)); // Blocks!
        process(msg);
    }
}

// ✅ Good - use async sleep
tokio::select! {
    Some(msg) = rx.recv() => {
        tokio::time::sleep(Duration::from_secs(1)).await;
        process(msg);
    }
}
```

### ❌ Avoid: Complex Logic in Match Arms

```rust
// ❌ Bad - hard to read
tokio::select! {
    Some(msg) = rx.recv() => {
        // 50 lines of complex logic here
    }
}

// ✅ Good - extract to function
tokio::select! {
    Some(msg) = rx.recv() => {
        handle_message(msg).await;
    }
}
```

## Performance Considerations

1. **Branch Count**: `select!` scales well with many branches, but consider using a more specialized approach for 10+ channels
2. **Fairness Overhead**: Random selection has minimal overhead
3. **Cancellation**: Unselected branches are properly cancelled - no memory leaks

## Summary

The `tokio::select!` macro:

- ✅ Waits on multiple channels simultaneously
- ✅ Processes messages as they arrive
- ✅ Handles timing correctly (channel 1 arrives first, then channel 2)
- ✅ Uses fair/random selection by default (good for most cases)

**Key Points:**
- `select!` picks whichever branch becomes ready first
- Default behavior is fair (random selection among ready branches)
- Use `biased;` for priority handling
- Pattern match on `Some`/`None` to detect channel closure
- Great for combining channels, timeouts, and signals
