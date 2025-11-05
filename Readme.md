# Table of Contents
## Part 3: Shared State<br>
### Section 1. [How Arc Shares Immutable Data Across Multiple Tasks](shared_state/mutex_explanation.md) 

This code demonstrates reference-counted thread-safe sharing of immutable data using Arc

### Section 2. [How a Mutex Shares Mutable State](shared_state/mutex_explanation.md)

This code demonstrates safe concurrent access to shared mutable state using Arc and Mutex

### Section 3. [How RwLock Enables Multiple Concurrent Readers](shared_state/rw_lock_explaination.md)

This code demonstrates how RwLock (Read-Write Lock) enables multiple concurrent readers while maintaining exclusive access for writers. 

### Section 4. [How Semaphores Limit Concurrent Access](shared_state/semaphore_explaination.md)

A semaphore is a synchronization primitive that limits the number of tasks that can access a resource simultaneously.

### Section 5. [Deadlock Prevention in Concurrent Code](shared_state/)

A deadlock occurs when two or more tasks are waiting for each other to release resources, creating a circular dependency where none can proceed.

### Section 6. [How Barriers Work for Task Synchronization](shared_state/) 

A Barrier is a synchronization point where tasks must wait until a specified number of tasks reach that point, then all proceed together.

### Section 7. [How Notify Works for Signaling Between Tasks](shared_state/)
Notify is a simple, lightweight synchronization primitive for signaling between tasks. One task waits for a signal, another task sends it.

### Section 8. [How Watch Channels Broadcast State Changes](shared_state/)

This code demonstrates how a watch channel broadcasts state changes to multiple receivers, where each receiver can observe the latest value.

    