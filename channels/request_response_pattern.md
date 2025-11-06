# Request-Response Pattern in Tokio Using Oneshot Channels

## Overview

The request-response pattern is a common communication pattern where a client sends a request to a worker and waits for a response. In Tokio, this is elegantly implemented by combining:

- **`mpsc::channel`**: For sending requests from multiple clients to a single worker
- **`oneshot::channel`**: For sending a single response back to each specific client

This pattern enables asynchronous RPC (Remote Procedure Call) style communication between tasks.

## Code Example

```rust
use tokio::sync::{mpsc, oneshot};

struct Request {
    value: i32,
    response: oneshot::Sender<i32>,
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<Request>(32);
    
    // Worker task that processes requests
    let worker = tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            let result = req.value * 2;
            // Handle the Result from send()
            if let Err(_) = req.response.send(result) {
                eprintln!("Failed to send response - receiver dropped");
            }
        }
    });
    
    // Send a request and wait for response
    let (resp_tx, resp_rx) = oneshot::channel();
    let request = Request {
        value: 21,
        response: resp_tx,
    };
    
    tx.send(request).await.unwrap();
    
    // Handle the Result from recv
    let result = resp_rx.await.unwrap();
    println!("Result: {}", result);
    
    drop(tx);
    worker.await.unwrap();
}
```

**Expected Output:**
```
Result: 42
```

## How the Pattern Works

### Architecture Overview

```
Client Task                     Worker Task
    |                               |
    | 1. Create oneshot channel     |
    |    (resp_tx, resp_rx)         |
    |                               |
    | 2. Create Request with        |
    |    value + resp_tx            |
    |                               |
    | 3. Send Request via mpsc  --> | 4. Receive Request
    |                               |
    | 5. Wait on resp_rx            | 6. Process value
    |    (blocking)                 |
    |                               | 7. Send result via resp_tx
    |                               |
    | 8. Receive result         <-- |
    |                               |
    | 9. Continue execution         |
```

### Step-by-Step Execution

#### Step 1: Setup MPSC Channel

```rust
let (tx, mut rx) = mpsc::channel::<Request>(32);
```

- Creates a multi-producer, single-consumer channel
- Capacity of 32 requests
- `tx` can be cloned for multiple clients
- `rx` stays with the worker

#### Step 2: Spawn Worker Task

```rust
let worker = tokio::spawn(async move {
    while let Some(req) = rx.recv().await {
        let result = req.value * 2;
        if let Err(_) = req.response.send(result) {
            eprintln!("Failed to send response - receiver dropped");
        }
    }
});
```

The worker:
1. Receives `Request` objects from the mpsc channel
2. Processes each request (in this case, doubles the value)
3. Sends the result back through the `oneshot::Sender` included in the request
4. Continues until the mpsc channel is closed

#### Step 3: Client Creates Oneshot Channel

```rust
let (resp_tx, resp_rx) = oneshot::channel();
```

- **`resp_tx`**: Sender half - included in the request, used by worker
- **`resp_rx`**: Receiver half - kept by client, waits for response

**Key Point**: A new oneshot channel is created for **each request**. This ensures responses are matched to their specific requests.

#### Step 4: Client Sends Request

```rust
let request = Request {
    value: 21,
    response: resp_tx,  // Worker will use this to send back
};

tx.send(request).await.unwrap();
```

The client packages:
- The data to process (`value: 21`)
- The return address (`response: resp_tx`)

#### Step 5: Client Waits for Response

```rust
let result = resp_rx.await.unwrap();
println!("Result: {}", result);
```

- Client blocks waiting on the oneshot receiver
- When worker sends result, client receives it
- Execution continues with the result

## Why Use Oneshot Channels?

### Oneshot Channel Characteristics

A `oneshot::channel` is designed for exactly **one message**:

```rust
pub fn oneshot::channel<T>() -> (Sender<T>, Receiver<T>)
```

**Properties:**
- ✅ Extremely lightweight (no buffer needed)
- ✅ Zero allocation after creation
- ✅ Type-safe request-response matching
- ✅ Automatic cleanup (dropping sender/receiver signals cancellation)
- ✅ Works across task boundaries

**Comparison to alternatives:**

| Method | Oneshot | MPSC | Mutex/RwLock | Channel |
|--------|---------|------|--------------|---------|
| Single message | ✅ | ❌ | ❌ | ❌ |
| Across tasks | ✅ | ✅ | ⚠️ Complex | ✅ |
| Zero-cost | ✅ | ❌ | ✅ | ❌ |
| Request matching | ✅ | ❌ Manual | ❌ | ❌ Manual |

## Multiple Clients Example

The real power of this pattern emerges with multiple concurrent clients:

```rust
use tokio::sync::{mpsc, oneshot};

struct Request {
    id: usize,
    value: i32,
    response: oneshot::Sender<i32>,
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<Request>(32);
    
    // Single worker processes all requests
    let worker = tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            println!("Worker processing request {}", req.id);
            let result = req.value * 2;
            let _ = req.response.send(result);
        }
    });
    
    // Spawn multiple client tasks
    let mut handles = vec![];
    
    for i in 0..5 {
        let tx_clone = tx.clone();
        
        let handle = tokio::spawn(async move {
            let (resp_tx, resp_rx) = oneshot::channel();
            
            let request = Request {
                id: i,
                value: i as i32 * 10,
                response: resp_tx,
            };
            
            tx_clone.send(request).await.unwrap();
            let result = resp_rx.await.unwrap();
            
            println!("Client {} received result: {}", i, result);
        });
        
        handles.push(handle);
    }
    
    // Wait for all clients to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    drop(tx);
    worker.await.unwrap();
}
```

**Expected Output (order may vary):**
```
Worker processing request 0
Worker processing request 1
Client 0 received result: 0
Worker processing request 2
Client 1 received result: 20
Worker processing request 3
Client 2 received result: 40
Worker processing request 4
Client 3 received result: 60
Client 4 received result: 80
```

## Advanced: Actor Pattern

This request-response pattern forms the foundation of the Actor model:

```rust
use tokio::sync::{mpsc, oneshot};
use std::collections::HashMap;

enum ActorMessage {
    Get {
        key: String,
        response: oneshot::Sender<Option<String>>,
    },
    Set {
        key: String,
        value: String,
        response: oneshot::Sender<()>,
    },
    Delete {
        key: String,
        response: oneshot::Sender<bool>,
    },
}

struct KeyValueActor {
    receiver: mpsc::Receiver<ActorMessage>,
    store: HashMap<String, String>,
}

impl KeyValueActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>) -> Self {
        Self {
            receiver,
            store: HashMap::new(),
        }
    }
    
    async fn run(mut self) {
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                ActorMessage::Get { key, response } => {
                    let value = self.store.get(&key).cloned();
                    let _ = response.send(value);
                }
                ActorMessage::Set { key, value, response } => {
                    self.store.insert(key, value);
                    let _ = response.send(());
                }
                ActorMessage::Delete { key, response } => {
                    let existed = self.store.remove(&key).is_some();
                    let _ = response.send(existed);
                }
            }
        }
    }
}

// Actor handle for clients to use
#[derive(Clone)]
struct KVStore {
    sender: mpsc::Sender<ActorMessage>,
}

impl KVStore {
    fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        let actor = KeyValueActor::new(receiver);
        tokio::spawn(actor.run());
        
        Self { sender }
    }
    
    async fn get(&self, key: String) -> Option<String> {
        let (tx, rx) = oneshot::channel();
        let msg = ActorMessage::Get {
            key,
            response: tx,
        };
        
        self.sender.send(msg).await.unwrap();
        rx.await.unwrap()
    }
    
    async fn set(&self, key: String, value: String) {
        let (tx, rx) = oneshot::channel();
        let msg = ActorMessage::Set {
            key,
            value,
            response: tx,
        };
        
        self.sender.send(msg).await.unwrap();
        rx.await.unwrap();
    }
    
    async fn delete(&self, key: String) -> bool {
        let (tx, rx) = oneshot::channel();
        let msg = ActorMessage::Delete {
            key,
            response: tx,
        };
        
        self.sender.send(msg).await.unwrap();
        rx.await.unwrap()
    }
}

#[tokio::main]
async fn main() {
    let store = KVStore::new(32);
    
    // Use the actor like a regular async API
    store.set("name".to_string(), "Alice".to_string()).await;
    
    let value = store.get("name".to_string()).await;
    println!("Value: {:?}", value); // Value: Some("Alice")
    
    let deleted = store.delete("name".to_string()).await;
    println!("Deleted: {}", deleted); // Deleted: true
    
    let value = store.get("name".to_string()).await;
    println!("Value after delete: {:?}", value); // Value after delete: None
}
```

**Output:**
```
Value: Some("Alice")
Deleted: true
Value after delete: None
```

This actor pattern provides:
- **Encapsulation**: Internal state is never directly accessed
- **Thread-safety**: No locks needed, actor processes messages sequentially
- **Async-friendly**: Clean async API for clients
- **Type-safety**: Compile-time guarantees on message handling

## Error Handling Patterns

### Pattern 1: Propagate Errors Through Response

```rust
use tokio::sync::{mpsc, oneshot};

struct Request {
    value: i32,
    response: oneshot::Sender<Result<i32, String>>,
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<Request>(32);
    
    let worker = tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            let result = if req.value >= 0 {
                Ok(req.value * 2)
            } else {
                Err("Negative values not allowed".to_string())
            };
            
            let _ = req.response.send(result);
        }
    });
    
    // Test with valid value
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Request { value: 21, response: resp_tx }).await.unwrap();
    
    match resp_rx.await.unwrap() {
        Ok(result) => println!("Success: {}", result),
        Err(e) => println!("Error: {}", e),
    }
    
    // Test with invalid value
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Request { value: -5, response: resp_tx }).await.unwrap();
    
    match resp_rx.await.unwrap() {
        Ok(result) => println!("Success: {}", result),
        Err(e) => println!("Error: {}", e),
    }
    
    drop(tx);
    worker.await.unwrap();
}
```

**Output:**
```
Success: 42
Error: Negative values not allowed
```

### Pattern 2: Handle Timeout

```rust
use tokio::sync::{mpsc, oneshot};
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<Request>(32);
    
    // Worker that might be slow
    let worker = tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let _ = req.response.send(req.value * 2);
        }
    });
    
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Request { value: 21, response: resp_tx }).await.unwrap();
    
    // Wait with timeout
    match timeout(Duration::from_millis(50), resp_rx).await {
        Ok(Ok(result)) => println!("Got result: {}", result),
        Ok(Err(_)) => println!("Channel closed"),
        Err(_) => println!("Timeout waiting for response"),
    }
    
    drop(tx);
    worker.await.unwrap();
}
```

**Output:**
```
Timeout waiting for response
```

### Pattern 3: Detect Cancelled Requests

```rust
use tokio::sync::{mpsc, oneshot};

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<Request>(32);
    
    let worker = tokio::spawn(async move {
        let mut cancelled = 0;
        let mut processed = 0;
        
        while let Some(req) = rx.recv().await {
            // Check if client still waiting
            if req.response.is_closed() {
                cancelled += 1;
                println!("Request cancelled by client");
                continue;
            }
            
            let result = req.value * 2;
            match req.response.send(result) {
                Ok(_) => processed += 1,
                Err(_) => cancelled += 1,
            }
        }
        
        println!("Stats: processed={}, cancelled={}", processed, cancelled);
    });
    
    // Send request but drop receiver (cancel)
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Request { value: 21, response: resp_tx }).await.unwrap();
    drop(resp_rx); // Client no longer interested
    
    // Send normal request
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Request { value: 42, response: resp_tx }).await.unwrap();
    let result = resp_rx.await.unwrap();
    println!("Result: {}", result);
    
    drop(tx);
    worker.await.unwrap();
}
```

**Output:**
```
Request cancelled by client
Result: 84
Stats: processed=1, cancelled=1
```

## Performance Considerations

### When to Use This Pattern

✅ **Good for:**
- RPC-style communication between tasks
- Actor model implementations
- Request-response APIs with async workers
- Database connection pools
- Background job processing with results

❌ **Not ideal for:**
- High-frequency, low-latency operations (consider lock-free data structures)
- Simple shared state (consider `Arc<Mutex<T>>` or `Arc<RwLock<T>>`)
- Fire-and-forget messages (use mpsc without oneshot)
- Streaming responses (use mpsc for responses too)

### Optimization Tips

1. **Reuse MPSC sender**: Clone `tx` once per client task, not per request
2. **Batch requests**: Send multiple requests before awaiting responses
3. **Buffer sizing**: Tune mpsc capacity based on expected load
4. **Avoid unnecessary clones**: Pass ownership when possible

```rust
// Efficient: Clone sender once per client
let tx_clone = tx.clone();
tokio::spawn(async move {
    for i in 0..100 {
        // Use tx_clone for all requests
    }
});

// Inefficient: Cloning in hot loop
for i in 0..100 {
    let tx_clone = tx.clone(); // Wasteful
    // ...
}
```

## Common Pitfalls and Solutions

### Pitfall 1: Forgetting to Handle Send Errors

```rust
// ❌ Wrong - ignores error
req.response.send(result);

// ✅ Correct - handles error
if let Err(_) = req.response.send(result) {
    eprintln!("Response receiver dropped");
}

// ✅ Also correct - acknowledges intentionally ignoring
let _ = req.response.send(result);
```

### Pitfall 2: Not Unwrapping Response Receiver

```rust
// ❌ Wrong - result is Result<i32, RecvError>
let result = resp_rx.await;

// ✅ Correct - extracts the i32
let result = resp_rx.await.unwrap();

// ✅ Or handle error properly
let result = resp_rx.await.expect("Worker dropped response");
```

### Pitfall 3: Reusing Oneshot Channels

```rust
// ❌ Wrong - oneshot can only send once
let (resp_tx, resp_rx) = oneshot::channel();
for i in 0..3 {
    tx.send(Request { value: i, response: resp_tx }).await; // Won't compile!
}

// ✅ Correct - create new oneshot per request
for i in 0..3 {
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Request { value: i, response: resp_tx }).await.unwrap();
    let result = resp_rx.await.unwrap();
}
```

## Summary

The request-response pattern using oneshot channels provides:

1. **Type-safe RPC**: Compile-time guarantees on request-response matching
2. **Clean async API**: Natural async/await syntax for both client and worker
3. **Scalability**: Single worker handles multiple concurrent clients efficiently
4. **Cancellation support**: Automatic cleanup when either side drops
5. **Foundation for actors**: Building block for the actor pattern

**Key Components:**
- `mpsc::channel`: Routes requests from clients to worker
- `oneshot::channel`: Returns responses from worker to specific client
- `Request` struct: Bundles data + response channel

This pattern is fundamental to building robust concurrent applications in Rust with Tokio, enabling clean separation of concerns and safe communication across task boundaries.