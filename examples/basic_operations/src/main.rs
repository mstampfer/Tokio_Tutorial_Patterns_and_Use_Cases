// Example: Manual Tokio Runtime Creation
// This demonstrates creating a Tokio runtime without using the #[tokio::main] macro

use tokio::runtime::Runtime;

fn main() {
    // Create a new multi-threaded runtime
    let runtime = Runtime::new().unwrap();

    // Execute async code on the runtime
    runtime.block_on(async {
        println!("Hello from Tokio runtime!");

        // Spawn a task
        let handle = tokio::spawn(async {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            println!("Task completed");
        });

        handle.await.unwrap();
    });

    println!("Runtime shutdown complete");
}
