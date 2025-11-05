# How Watch Channels Broadcast State Changes

This code demonstrates how a **watch channel** broadcasts state changes to multiple receivers, where each receiver can observe the latest value. Here's how it works:

## Key Concept: Watch Channels

A `watch` channel is a **single-producer, multi-consumer** channel designed for broadcasting state updates:
- **One sender** can update a shared value
- **Multiple receivers** can read the latest value and get notified of changes
- Receivers only see the **most recent value** (intermediate updates may be skipped)

## How the Code Works

**1. Creating the Channel**
```rust
let (tx, mut rx1) = watch::channel(0);
let mut rx2 = tx.subscribe();
```
- `watch::channel(0)` creates a channel with initial value `0`
- `tx` is the sender (transmitter)
- `rx1` is the first receiver
- `rx2` is created by calling `tx.subscribe()` to get another receiver

**2. Receiver Tasks Listen for Changes**
```rust
let receiver1 = tokio::spawn(async move {
    while rx1.changed().await.is_ok() {
        println!("Receiver 1 saw change: {}", rx1.borrow());
    }
});
```
- `changed().await` blocks until the value changes (or sender is dropped)
- `borrow()` gets a read-only reference to the current value
- The loop continues until the sender is dropped (when `changed()` returns `Err`)

Both receivers run this same pattern independently and concurrently.

**3. Sender Broadcasts Updates**
```rust
for i in 1..=3 {
    sleep(Duration::from_millis(100)).await;
    tx.send(i);  // ← All receivers are notified
}
```
Each `send()` updates the shared state and notifies **all active receivers** that a change occurred. Both receiver tasks will wake up and see the new value.

**4. Clean Shutdown**
```rust
drop(tx);  // ← Signals "no more updates"
```
Dropping the sender causes `changed().await` to return `Err` in all receivers, allowing them to exit their loops gracefully.

## Why Watch Channels Are Useful

**Broadcasting State**: Perfect for scenarios where multiple tasks need to observe shared state like:
- Configuration changes
- Application status updates
- Feature flags
- Shutdown signals

**Always Current**: Unlike regular channels that queue messages, watch channels only keep the **latest value**. If a sender updates twice before a receiver checks, the receiver only sees the most recent value.

**Memory Efficient**: Only stores one value regardless of how many receivers exist or how fast updates occur.

## Example Output

```
Receiver 1 saw change: 1
Receiver 2 saw change: 1
Receiver 1 saw change: 2
Receiver 2 saw change: 2
Receiver 1 saw change: 3
Receiver 2 saw change: 3
```

The exact order may vary since both receivers run concurrently, but both will see all three updates (1, 2, 3) because the 100ms delays give them time to process each change.

## Complete Code Example

```rust
use tokio::sync::watch;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let (tx, mut rx1) = watch::channel(0);
    let mut rx2 = tx.subscribe();
    
    let receiver1 = tokio::spawn(async move {
        while rx1.changed().await.is_ok() {
            println!("Receiver 1 saw change: {}", rx1.borrow());
        }
    });
    
    let receiver2 = tokio::spawn(async move {
        while rx2.changed().await.is_ok() {
            println!("Receiver 2 saw change: {}", rx2.borrow());
        }
    });
    
    for i in 1..=3 {
        sleep(Duration::from_millis(100)).await;
        tx.send(i);
    }
    
    drop(tx);
    
    receiver1.await.unwrap();
    receiver2.await.unwrap();
}
```
