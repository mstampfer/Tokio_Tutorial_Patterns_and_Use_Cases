# Tokio Patterns
This repository is a collection of Tokio (Rust async runtime) patterns and examples

## Table of Contents
### Part 1: $\color{yellow}{\textsf{Basic Operations}}$
#### Section 1. [Manual Tokio Runtime Creation](basic_operations/tokio_main_macro.md)

Instead of using the `#[tokio::main]` macro, manually create a Tokio runtime

#### Section 2. [Multithreaded Runtime](basic_operations/multi_threaded.md)

Configure the runtime to use 2 worker threads

#### Section 3. [Current Thread Runtime vs Multithread Runtime](basic_operations/current_thread_runtime.md)

This code demonstrates how to create a **single-threaded** Tokio runtime using `new_current_thread()` instead of a multi-threaded runtime.

### Part 2: $\color{yellow}{\textsf{Spawning}}$

#### Section 1: [Async Function](spawning/async_function.md)

This Rust code demonstrates basic asynchronous task spawning using the Tokio runtime.

### Section 2: [How Arc Shares Vector Data Across Multiple Tasks](spawning/spawning_with_owned_data.md)

This code demonstrates safe shared ownership of data across multiple asynchronous tasks using Arc

### Section 3: [Task Cancellation](spawning/task_cancellation.md)

This code demonstrates how to stop a running asynchronous task before it completes naturally. 

### Section 4: [How Tokio Ensures Data is Send in Spawned Tasks](spawning/send_bound.md)

This code demonstrates Rust's Send trait enforcement for data shared across asynchronous tasks.

### Part 3: $\color{yellow}{\textsf{Shared State}}$
#### Section 1. [How Arc Shares Immutable Data Across Multiple Tasks](shared_state/arc_sharing_explanation.md)

This code demonstrates reference-counted thread-safe sharing of immutable data using Arc

#### Section 2. [How a Mutex Shares Mutable State](shared_state/mutex_explanation.md)

This code demonstrates safe concurrent access to shared mutable state using Arc and Mutex

#### Section 3. [How RwLock Enables Multiple Concurrent Readers](shared_state/rwlock_explanation.md)

This code demonstrates how RwLock (Read-Write Lock) enables multiple concurrent readers while maintaining exclusive access for writers. 

#### Section 4. [How Semaphores Limit Concurrent Access](shared_state/semaphore_explanation.md)

A semaphore is a synchronization primitive that limits the number of tasks that can access a resource simultaneously.

#### Section 5. [Deadlock Prevention in Concurrent Code](shared_state/deadlock_prevention.md)

A deadlock occurs when two or more tasks are waiting for each other to release resources, creating a circular dependency where none can proceed.

#### Section 6. [How Barriers Work for Task Synchronization](shared_state/barrier_explanation.md) 

A Barrier is a synchronization point where tasks must wait until a specified number of tasks reach that point, then all proceed together.

#### Section 7. [How Notify Works for Signaling Between Tasks](shared_state/notify_explanation.md)
Notify is a simple, lightweight synchronization primitive for signaling between tasks. One task waits for a signal, another task sends it.

#### Section 8. [How Watch Channels Broadcast State Changes](shared_state/watch_channel_explanation.md)

This code demonstrates how a watch channel broadcasts state changes to multiple receivers, where each receiver can observe the latest value.

### Part 4: $\color{yellow}{\textsf{Channels}}$

#### Section 1. [Tokio MPSC Channel Explanation](channels/mpsc_explanation.md)

#### Section 2. [Tokio MPSC: Multiple Sender Tasks Explanation](channels/mpsc_explanation.md)

#### Section 3. [Tokio MPSC Backpressure Handling](channels/backpressure_explanation.md)

#### Section 4. [Oneshot Channel: Request-Response Pattern](channels/oneshot_channel_explanation.md)

#### Section 5. [Understanding Tokio Broadcast Channels](channels/broadcase_channel_explaination.md)

#### Section 6. [How Tokio MPSC Channels Handle Sender Drops and Closure](channels/tokio_channel_closure.md)

#### Section 7. [Understanding try_send in Tokio MPSC Channels](channels/tokio_try_send_explained.md)

#### Section 8. [Request-Response Pattern in Tokio Using Oneshot Channels](channels/channels/request_response_pattern.md)

#### Section 9. [Using tokio::select! to Wait on Multiple Channels](channels/tokio%20select%20explained.md)
    
### Part 5: $\color{yellow}{\textsf{I/O}}$
#### Section 1. [Asynchronous File Reading in Rust with Tokio](io/async_file_reading_explanation.md)

#### Section 2. [Asynchronous File Writing in Rust with Tokio](io/async_file_writing_explanation.md)

#### Section 3. [Async File Copy in Rust with Tokio](io/async_file_copy.md)

#### Section 4. [Reading a File Line by Line with Tokio's BufReader](io/reading_files_with_BufReader.md)

#### Section 5. [TCP Echo Server in Rust with Tokio](io/tcp_echo_server.md)
#### Section 6. [](io/)
#### Section 7. [](io/)
#### Section 8. [](io/)
#### Section 9. [](io/)
    
### Part 6: $\color{yellow}{\textsf{Framing}}$
#### Section 1. [](framing/)
#### Section 2. [](framing/)
#### Section 3. [](framing/)
#### Section 4. [](framing/)
#### Section 5. [](framing/)
#### Section 6. [](framing/)
#### Section 7. [](framing/)
#### Section 8. [](framing/)
#### Section 9. [](framing/)
    
### Part 7: $\color{yellow}{\textsf{Async in Depth}}$
#### Section 1. [](async_in_depth/)
#### Section 2. [](async_in_depth/)
#### Section 3. [](async_in_depth/)
#### Section 4. [](async_in_depth/)
#### Section 5. [](async_in_depth/)
#### Section 6. [](async_in_depth/)
#### Section 7. [](async_in_depth/)
#### Section 8. [](async_in_depth/)
#### Section 9. [](async_in_depth/)
    
### Part 8: $\color{yellow}{\textsf{Select}}$
#### Section 1. [](select/)
#### Section 2. [](select/)
#### Section 3. [](select/)
#### Section 4. [](select/)
#### Section 5. [](select/)
#### Section 6. [](select/)
#### Section 7. [](select/)
#### Section 8. [](select/)
#### Section 9. [](select/)
    
### Part 9: $\color{yellow}{\textsf{Streams}}$
#### Section 1. [](streams/)
#### Section 2. [](streams/)
#### Section 3. [](streams/)
#### Section 4. [](streams/)
#### Section 5. [](streams/)
#### Section 6. [](streams/)
#### Section 7. [](streams/)
#### Section 8. [](streams/)
#### Section 9. [](streams/)
    