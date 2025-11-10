// Example: Spawning async tasks with Arc for shared data
// This demonstrates how to spawn multiple tasks that share immutable data

use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Create shared data wrapped in Arc
    let data = Arc::new(vec![1, 2, 3, 4, 5]);

    let mut handles = vec![];

    // Spawn 3 tasks
    for i in 0..3 {
        let data_clone = Arc::clone(&data);
        let handle = tokio::spawn(async move {
            println!("Task {} sees: {:?}", i, data_clone);
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    println!("All tasks completed");
}
