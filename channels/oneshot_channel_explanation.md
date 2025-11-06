# Oneshot Channel: Request-Response Pattern

## Overview

A oneshot channel is a specialized communication primitive in Tokio designed for **single-use, one-time message passing** between asynchronous tasks. Unlike regular channels that can send multiple messages, a oneshot channel is consumed after sending exactly one value.


## Complete Code

```rust
use tokio::sync::oneshot;

async fn compute(tx: oneshot::Sender<i32>) {
    let result = 42;
    let _ = tx.send(result);
}

#[tokio::main]
async fn main() {
    let (tx, rx) = oneshot::channel();
    
    tokio::spawn(compute(tx));
    
    match rx.await {
        Ok(result) => println!("Result: {}", result),
        Err(_) => println!("Computation failed"),
    }
}
```

## How the Code Works

### Channel Creation

```rust
let (tx, rx) = oneshot::channel();
```

This creates a **paired sender (`tx`) and receiver (`rx`)**:
- **`tx` (Sender)**: Used to send one value
- **`rx` (Receiver)**: Used to receive that one value

### The Request-Response Pattern

The code demonstrates a classic request-response pattern:

```
Main Task                    Spawned Task (compute)
    |                               |
    |------ spawn with tx --------->|
    |                               |
    |                          (computes 42)
    |                               |
    |<------ sends result ----------|
    |         via tx.send()         |
    |                               |
 rx.await                      (task ends)
    |
(receives 42)
```

### Step-by-Step Breakdown

#### 1. **Spawning the Worker Task**

```rust
tokio::spawn(compute(tx));
```

- The main task spawns a new asynchronous task
- It **hands off the sender (`tx`)** to the spawned task
- Ownership of `tx` is transferred, ensuring only the spawned task can send

#### 2. **Computing in the Background**

```rust
async fn compute(tx: oneshot::Sender<i32>) {
    let result = 42;
    tx.send(result).unwrap();
}
```

- The spawned task performs its computation (here, simply calculating 42)
- It sends the result back through the channel using `tx.send()`
- The channel is **consumed** after sending - it cannot be reused

#### 3. **Waiting for the Response**

```rust
let result = rx.await.unwrap();
println!("Result: {}", result);
```

- The main task **awaits** on the receiver
- This is asynchronous - the main task yields control while waiting
- When the value arrives, `rx.await` completes with `Ok(42)`
- The `unwrap()` extracts the value (or panics if the sender was dropped)

## Why Use Oneshot Channels?

### Advantages

1. **Type-Safe Communication**: Guarantees exactly one message will be sent
2. **Zero-Cost Abstraction**: Optimized for single-message scenario
3. **Ownership Semantics**: Compiler enforces that only one value can be sent
4. **Async-Friendly**: Works seamlessly with Tokio's async runtime

### Common Use Cases

- **RPC-style calls**: Request a computation and wait for the result
- **Task completion notification**: Signal when a background task finishes
- **Resource initialization**: Wait for a resource to be ready
- **Graceful shutdown**: Coordinate shutdown between tasks

## Comparison with Other Channels

| Feature | Oneshot | mpsc | broadcast |
|---------|---------|------|-----------|
| Messages per channel | 1 | Many | Many |
| Senders | 1 | Many | 1 |
| Receivers | 1 | 1 | Many |
| Use case | Request-response | Work queue | Pub-sub |

## Error Handling

The channel can fail in two ways:

### 1. Send Fails (Receiver Dropped)

```rust
tx.send(result).unwrap(); // Panics if rx was dropped
// Better:
let _ = tx.send(result); // Ignore error
```

### 2. Receive Fails (Sender Dropped)

```rust
rx.await.unwrap(); // Panics if tx was dropped without sending
// Better:
match rx.await {
    Ok(result) => println!("Got: {}", result),
    Err(_) => println!("Sender dropped without sending"),
}
```

## Key Takeaways

- Oneshot channels are **perfect for single request-response patterns**
- They provide **compile-time guarantees** about single-use semantics
- The pattern naturally maps to **async/await** programming
- They're more efficient than general-purpose channels for one-shot communication
- The receiver is a `Future` that completes when the value is sent


This pattern is fundamental to async Rust and forms the basis for many higher-level concurrency patterns.