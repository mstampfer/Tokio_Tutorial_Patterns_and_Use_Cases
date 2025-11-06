# Tokio MPSC Channel Explanation

This document demonstrates asynchronous communication between tasks using Tokio's multi-producer, single-consumer (mpsc) channel.

## Complete Code Example

```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(32);
    
    tokio::spawn(async move {
        tx.send("Hello from sender!").await;
    });
    
    let message = rx.recv().await;
    println!("Received: {:?}", message);
}
```

## How It Works

### Channel Creation

```rust
let (tx, mut rx) = mpsc::channel(32);
```

This creates a bounded channel with capacity for 32 messages. It returns two parts:
- `tx` (transmitter/sender) - used to send messages into the channel
- `rx` (receiver) - used to receive messages from the channel

### Spawning the Sender Task

```rust
tokio::spawn(async move {
    tx.send("Hello from sender!").await;
});
```

This spawns a new asynchronous task that runs concurrently with the main task. The `move` keyword transfers ownership of `tx` into this task. Inside, it sends the string `"Hello from sender!"` into the channel. The `.await` means if the channel is full, it will asynchronously wait until space is available (though with only one message and capacity of 32, it won't block here).

### Receiving the Message

```rust
let message = rx.recv().await;
println!("Received: {:?}", message);
```

Back in the main task, `rx.recv().await` waits asynchronously until a message arrives in the channel. When the spawned task sends its message, `recv()` returns `Some("Hello from sender!")`. The program then prints `Received: Some("Hello from sender!")`.

## Key Points

- The channel allows tasks to communicate without shared memory or locks
- Both sending and receiving are asynchronous operations that don't block the thread
- The spawned task runs independently and may execute before, during, or after the `recv()` call
- `recv()` returns `Option<T>` - `Some(message)` when data arrives, or `None` if all senders are dropped