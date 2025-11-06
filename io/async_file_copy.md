# Async File Copy in Rust with Tokio

## Complete Code

```rust
use tokio::fs;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let bytes_copied = fs::copy("source.txt", "destination.txt").await?;
    println!("Copied {} bytes", bytes_copied);
    Ok(())
}
```

## How It Works

### 1. **Importing Tokio's Async File System Module**

```rust
use tokio::fs;
```

This imports Tokio's asynchronous file system operations. Unlike `std::fs`, which blocks the thread during I/O operations, `tokio::fs` provides non-blocking async alternatives.

### 2. **Setting Up the Async Runtime**

```rust
#[tokio::main]
async fn main() -> std::io::Result<()> {
```

The `#[tokio::main]` macro transforms the async `main` function into a synchronous one that sets up the Tokio runtime. This runtime manages:
- Thread pools for executing async tasks
- An event loop (reactor) that handles I/O operations
- Task scheduling and coordination

### 3. **Performing the Async Copy Operation**

```rust
let bytes_copied = fs::copy("source.txt", "destination.txt").await?;
```

This line does several things:

**`fs::copy("source.txt", "destination.txt")`**
- Initiates an asynchronous file copy operation
- Returns a `Future` that represents the pending operation
- Does not block the thread immediately

**`.await`**
- Suspends the current async function until the copy operation completes
- Yields control back to the Tokio runtime, which can execute other tasks
- The runtime uses efficient I/O mechanisms (epoll on Linux, kqueue on macOS, IOCP on Windows)
- When the copy completes, execution resumes and the `Future` resolves to `Result<u64, std::io::Error>`

**`?` operator**
- Unwraps the `Result` if successful, extracting the number of bytes copied
- Propagates any errors up the call stack, returning early from `main` if the copy fails

### 4. **Displaying the Result**

```rust
println!("Copied {} bytes", bytes_copied);
```

Prints the number of bytes successfully copied from source to destination.

### 5. **Returning Success**

```rust
Ok(())
```

Returns `Ok(())` to indicate successful completion, matching the `std::io::Result<()>` return type.

## Advantages of Async I/O

1. **Non-blocking**: The thread isn't blocked waiting for disk I/O to complete
2. **Efficiency**: The runtime can handle other tasks while waiting for I/O operations
3. **Scalability**: Better resource utilization when performing many I/O operations concurrently
4. **Cooperative multitasking**: Multiple async tasks can share the same thread efficiently

## Example Use Case

While this simple example shows a single file copy, async I/O shines when copying multiple files concurrently:

```rust
use tokio::fs;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Copy multiple files concurrently
    let task1 = tokio::spawn(async {
        fs::copy("file1.txt", "dest1.txt").await
    });
    
    let task2 = tokio::spawn(async {
        fs::copy("file2.txt", "dest2.txt").await
    });
    
    // Wait for both to complete
    let (result1, result2) = tokio::join!(task1, task2);
    
    println!("Copy operations completed!");
    Ok(())
}
```

This allows multiple I/O operations to progress simultaneously without blocking threads, making efficient use of system resources.