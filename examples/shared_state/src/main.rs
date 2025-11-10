// Example: Mutex for shared mutable state
// This demonstrates safe concurrent access to shared mutable state using Arc + Mutex

use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    // Create a shared counter wrapped in Arc and Mutex
    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    // Spawn 10 concurrent tasks
    for _ in 0..10 {
        // Clone the Arc to share ownership across tasks
        let counter = Arc::clone(&counter);

        // Spawn an async task that increments the counter
        let handle = tokio::spawn(async move {
            // Acquire the lock and increment
            let mut num = counter.lock().await;
            *num += 1;
            // Lock is automatically released when 'num' goes out of scope
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Print the final counter value
    println!("Counter: {}", *counter.lock().await);
    // Output: Counter: 10
}
