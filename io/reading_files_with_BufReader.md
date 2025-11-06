# Reading a File Line by Line with Tokio's BufReader

## Complete Code

```rust
use tokio::fs::File;
use tokio::io::{BufReader, AsyncBufReadExt};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let file = File::open("example.txt").await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    
    while let Some(line) = lines.next_line().await? {
        println!("Line: {}", line);
    }
    
    Ok(())
}
```

## How It Works

### 1. Opening the File Asynchronously
```rust
let file = File::open("example.txt").await?;
```
- Uses `tokio::fs::File` to open the file asynchronously
- The `.await?` waits for the file to open and propagates any errors if the file doesn't exist

### 2. Wrapping with BufReader
```rust
let reader = BufReader::new(file);
```
- `BufReader` wraps the file handle and adds buffering
- Instead of reading the file byte-by-byte (which would be inefficient), it reads chunks of data into an internal buffer
- This significantly improves performance, especially for large files

### 3. Getting a Lines Iterator
```rust
let mut lines = reader.lines();
```
- The `.lines()` method (from `AsyncBufReadExt` trait) returns a `Lines` struct
- This provides an iterator-like interface for reading lines

### 4. Reading Lines in a Loop
```rust
while let Some(line) = lines.next_line().await? {
    println!("Line: {}", line);
}
```
- `next_line().await?` asynchronously reads the next line from the buffer
- Returns `Some(String)` if a line is available, or `None` when end-of-file is reached
- The `?` operator propagates any I/O errors that occur during reading
- Each line is returned without the newline character (`\n` or `\r\n`)

## Key Benefits

- **Asynchronous**: Doesn't block the thread while waiting for I/O operations
- **Buffered**: Reads data in chunks for better performance
- **Memory Efficient**: Only one line is loaded into memory at a time
- **Clean API**: Simple iterator-like pattern for processing lines