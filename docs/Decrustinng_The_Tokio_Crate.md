
# The parts of tokio
on the face of it, what it does is it takes things that implement the future trait.
So, if we pull up the future trait here. It is just the thing that takes things that implement this future trait and then returns the thing that is in the associated output type.
That is all Tokyo kind of really does, right? It runs these futures and produces the output that that future represents the computation of.
In practice, there's a lot that goes into that kind of machinery. And so when we go into Tokyo,
I'm really going to sort of split this roughly into four parts. Let's see if it actually ends up being four,
but roughly four. The first is about Tokyo, the runtime. Like basically, how does it execute futures?
Here we're going to talk about things like worker threads and the thread pool, work stealing,
blocking, shutdown. The sort of... inner workings of the scheduler,
although not at the very, very lowest level, but enough that you sort of understand how the pieces fit together.
The second thing we're going to talk about is resources. So this is anything that has to do with IO,
anything that has to do with threading processes, the file system,
timers as well. So all of the things that a future might use in order to interact with the outside world.
Then we're going to talk about utilities. So this is more about what additional useful features Tokyo provides you with,
both in the crate itself and in slightly adjacent crates, for things like coordination and synchronization between futures,
handling multiple tasks, things like the select macro for allowing whichever one of these futures finishes first.
So you can run multiple of them concurrently, as well as things like the sort of Tokyo util crate we'll touch on a little bit on bridging between async read and async write,
which we'll talk about, and sync and stream, which we'll talk less about, and various other things like that.
And then the fourth part is going to be talking about common complications when working with Tokyo.
And I don't mean that as in bugs with Tokyo itself. I mean that as in...
sort of things that are easy to shoot yourself in the foot with if you don't realize exactly how something works or the caveats that comes with something,
the trade-offs that are involved with something. And so we'll talk through some common ones of those.
That is... That's sort of the rough outline here.


# The tokio runtime
So let's start with how Tokyo works as a runtime. So in
Tokyo, first of all, the documentation is quite good. Like I do recommend that in general, spend the time to read through the Tokyo docs.
They do talk about a lot of the things that I'm going to be talking about here. It's just that you have to remember to read them in order to realize these things.
There's a lot of really good documentation in there. So in Tokyo, there's a bunch of sub-modules and one of them is runtime.
And the runtime is sort of the main, the heart of Tokyo, if you will. It is the thing that you give futures to and it gives you back the thing that the future computes.
And at its core, the runtime, it says this at the top here,
it has an IO event loop. This gets back to things like resources, which we'll talk about later, timers,
which we'll also talk about later, and then a scheduler. And the scheduler's job is to take futures that come in,
call the pull method on that future. So remember, futures are these things that implement the future trait,
where async blocks and async functions are examples of things that automatically the compiler turns into things that implement the future trait.
The scheduler is going to pick a future that hasn't completed yet, call its poll method.
The poll method just executes like a normal function. And at some point, the poll function returns.
And if we look at what it looks like. The return value of pull is this pull type that is either ready,
which means the future has finished, it doesn't need to be run anymore, and that includes then the return value of that asynchronous task,
or pending, which means I didn't finish, there's more work to do. And when a future returns ready,
then Tokyo doesn't really do anything more with it. Depending on how you gave the task to Tokyo in the first place,
it's either going to give you that T, or that T is sort of going to be dropped into the ether, usually because it's just units and no one cares about it anyway.
But when a future returns pending from poll, what happens is that that future gets sort of put back into the runtime to be executed again,
like for poll to be called again at some later point in time. And we'll talk about how that mechanism works in a little bit more detail.
So, the scheduler part is the first thing we're going to talk about. There are two schedulers in Tokyo.
There is a multi-threaded scheduler and there's the current thread scheduler. For almost all use cases,
you want to use the multi-thread scheduler. The multi-thread scheduler creates,
by default at least, one thread, one operating system thread for every CPU core that you have.
And the idea here is that the different worker threads can be sort of in the process of calling pull on different futures at the same time.
And so that way you get to have actual parallelism where these different futures are executing in parallel with each other so that you get to use the CPU resources of your machine.
As opposed to, for example, just switching back and forth between them quickly on a single core where you wouldn't really get the parallelism but you get the perception of concurrency.
The multi-threaded scheduler is configurable, so you can choose how many threads you actually want,
but in general, having one per CPU core is usually the right choice.
There are some caveats to that when it comes to blocking threads that we'll talk to in a second. When you have the multi-threaded scheduler,
actually, I'll talk about the current scheduler first before I dive into either of these in more detail.
The current thread scheduler does not start any threads. Instead what it does, like it doesn't start one thread per core,
it doesn't even start one thread, it just uses, as its name implies, the current thread.
And it starts up a Tokyo runtime on the current thread that is able to, that has this sort of IO integration and this little queue for which things haven't been run,
which things can be run. And it just stores all of that locally on the current thread.
It executes some future that you give it and ultimately...
blocks the current thread until that work is done and only uses sort of CPU cycles on the current thread and nowhere else in order to drive those futures to completion.
We can talk about how these work internally, and that might be a little bit interesting. But before we do that,
I want to show you what this interface actually looks like. So if you have a runtime,
and this is the same type, whether it's configured as a current thread runtime or a multi-threaded runtime.
I wish this was default view. When you have a runtime, you have this method called spawn,
which we're going to ignore for right now. And then you have this method called block on.
And block on is the most straightforward interface to a runtime. It is also not the one you're going to use most commonly,
but it is the sort of most straightforward.
interface into a runtime. It takes a future that implements the future trait and it returns the output of that future.
That is all, right? So if you create a current thread runtime, you pass it some future to block on,
then it will do all the stuff that's needed behind the scene until that future ultimately returns the ready variant of the pull enum.
And then it's going to extract the value and give that back to you. If you have the current thread runtime, at that point, the runtime doesn't do anything more,
right? Because it does not spawn any threads, it doesn't do anything in the background. So the moment block on returns,
the current thread is now under your control again. And there's no way for Tokyo to run any more logic because you're not in Tokyo anymore.
This function has returned and whatever is in the sort of calling code is what's going to happen next.
Um, and, and this talks about a little bit, uh, what, what that actually means. Um,
So BlockCon is sort of the main interface here to Tokyo for turning futures into their outputs.
In the multi-threaded scheduler, and actually in the current thread scheduler,
you can call spawn. And what spawn does is it takes a future.
and it puts it onto the sort of queue of futures.
In fact, the queue of tasks, and I'll talk about the distinction between those in a second. It puts the future that you give onto the queue of tasks for the current runtime,
regardless of whether that's a current thread runtime or a multi-threaded runtime, but it doesn't execute anything.
It just sort of puts it on the queue and then returns, and it returns you a join handle. So this is sort of similar to if you use,
you know, Threadspawn. So if you use Threadspawn from the standard library, it spawns a new thread that is eventually going to run the closure that you give it,
and it gives you back a handle to that thread that you can use, for example, to wait for the thread to complete and get its result.
And the semantics here for join handle are very similar. So the join handle you get back from calling spawn...
is something that you can use to abort the task you've given,
or you see it itself implements future, and so you can await the join handle you get back in order to wait for the task,
the future that you spawned, to yield its value eventually and get it back.
If you drop the handle, nothing happens. There's no automatic abort when you drop a join handle.
This is the same thing as with standard thread spawn. The join handle you get back or the thread handle you get back does not do anything if you drop it.
The distinction between spawn and block on, as I mentioned, is that block on will block where you call it until the future is resolved.
Spawn will just sort of throw it out in a queue and then return. Every future that has been spawned onto a runtime becomes a task.
That does not mean that every future that is on a runtime is a task. And the distinction here is that futures can contain other futures.
So imagine that you have, you know, you write async, like an async block, and inside of the async block,
you call some asynchronous function. That asynchronous function itself creates a future. And maybe inside of there,
there's an async block. Maybe inside of there, it calls, I don't know, uh... it tries to read a file asynchronously,
or it tries to open a socket, whatever it might be, that is also a future. And so these are futures that contain futures that contain futures that contain futures.
And when you await one of those futures, you just sort of resolve that sub-future, but the larger future you're inside of is still there and has not returned ready yet.
It's only when the outermost future returns that sort of that task has completed.
And so again, you only the thing, only the futures that you pass to spawn become these top-level tasks.
And Tokyo only knows about the top level. It can't see inside your future types and see all the inner futures.
It only sees the top-level tasks. So the futures that have specifically been passed to runtime spawn,
or if you look in Tokyo itself, you see there's a macro called spawn.
And the spawn macro under the hood really does the same thing. It gets a handle to the current runtime and then it spawns the current,
the future you give it onto that runtime. It gives you back the join handle. And so it's only things that have been passed to spawn that turn into tasks.
In addition to, you know, the thing that you pass to block on also becomes one of these top level tasks.
And the Tokyo scheduler only knows about these top-level tasks. When you have a thread in Tokyo,
regardless of whether it's the current thread or a multi-threaded runtime, and that worker thread is looking for the next thing to work on,
it will look at the set of tasks, not the set of all nested futures.
And the thinking here, right, is that the only entry point that Tokyo has into your futures is calling the poll method on the future that it was actually passed.
And then that sort of cascades all the way down into polling all the inner futures, but the real interface is just the poll call on the topmost task,
the topmost future. So now we get to the distinction here between the current thread runtime and the multi-threaded runtime.
In the current thread runtime, if you call spawn, nothing happens. The future gets turned into a task,
gets put on the task queue, and then it returns. The future does not actually run in any meaningful sense.
It is only when you call block on that you're sort of giving over execution control to the current thread runtime,
to Tokyo. At that point, with the current thread runtime, Tokyo is going to look at the set of...
tasks that it has, execute them. until they yield either pull ready,
in which it's going to return, or yield pull pending, in which case they're going to put them on the not ready task queue.
And then it just keeps doing this in a loop. It keeps calling pull on all of the tasks that it knows about until it gets to a point where the future that was passed to block on returns ready,
and at that point it returns. And if there are tasks left in the queue, it just leaves them there, and then does not execute them any further.
So it's only in the context of block on that things in the task queue actually run. If you have a multi-threaded scheduler,
which again is the default, it's also what you get if you use like the Tokyo main annotation on a function,
then what will actually happen is you have this pool of worker threads, right? So when Tokyo starts up,
starts one worker thread for every CPU core you have. That pool of worker threads,
each one has its own local queue. of tasks that it knows about, and there's also a global queue that they can all read from.
And the idea is that every worker thread is just in a loop. And what they do in that loop is they look at their local queue and see if there are any tasks that can be executed.
If all of them are pending and there's no reason to continue, assume that you might make progress if you pull them,
then it looks at the global queue and sees if there are tasks there. If there are no tasks there either, then they can actually steal from each other.
So it's a work stealing scheduler. So, you know, if you have, let's say you have two CPU cores and...
You know, this thread has no tasks to run,
and this one has a queue that's full of tasks that are ready to run. When this one realizes, oh, I don't have any more work,
it's actually going to start looking at this other worker thread's queue and start to steal some tasks of it so that you're better utilizing your CPU resources.
So this means that futures can start to move between threads. When this evens out load between your CPU cores so that you get a...
make more use of the computer resources you have, but it also means, and this is sort of where that requirement that is sometimes annoying comes from,
is that futures must be send. They must be allowed to move between threads. They must implement the send trait in order for
TokyoP to be able to do this load distribution. And because Tokyo uses the same interface for current thread runtimes and multi-threaded runtimes,
all futures you pass to Tokyo have to be sent, regardless of whether you're actually only using the current thread runtime.
There is a way around this. We'll talk about that in a second, about local sets. But this is sort of the basic premise for the setup.
Before I now talk more about how... Sort of how this interacts with blocking,
when you can block threads, the co-op budget, block in place, shut down,
that kind of stuff. Does the general sense for what a runtime looks like make sense?
Let's see. Let's see.
How does this model work with blocking worklets? We'll talk about that in a second. What's the use case for the current thread scheduler?
The use case for the current thread scheduler... So there are sort of two. One is less so than the other.
The first one is you just... Like let's say you're running your test suite. Every test is annotated with like TokyoTest.
And you don't really want every test to spin up... Because you're going to run them all in parallel. You don't want every test to spin up
N threads. where n is the number of CPU cores, because now you end up with n times your number of tests.
And so you might use the current thread runtime there. In practice, and I think the Tokyo test annotation does this by default now,
it actually starts a multi-threaded scheduler with only one thread. And so that is very often the thing you want instead of a current thread scheduler.
just so that you don't have the sort of complexity of having to manage multiple schedulers and test multiple schedulers.
The real reason for using the current thread scheduler is if you have things like,
you want to limit the amount of movement that happens between threads, because you have a workload where you know that it's very costly if you're executing some sequence of instructions on one CPU,
and suddenly they get moved to a different CPU, right? That ends up sort of making your...
CPU cache utilization worse. Sometimes you have sort of CPU pinning that you do so that all of your workload happens on a particular set of CPU cores and you don't want the other CPU cores to be used at all for some reason.
Maybe you've dedicated them to some other worker pool. So the current thread runtime gives you more control over exactly what runs where,
but it does come with the cost. It's like it's much less of a general purpose executor.
Why would I use Tokyo spawn over Rust std thread spawn? So the difference there is that Tokyo spawn lets you give a future.
The standard library thread spawn spawns an operating system level thread. So you give it a closure and it executes that synchronous closure.
There's no async involved.
Is Tokyo main also a task? Yeah, so this is actually an interesting thing to look at. So if I go here and I do cargo new bin...
Kyoto. Aha! Kyoto. Yeah.
Because Tokyo, right? Anyway. Let me do add
Tokyo here with features full, source main, and I'll just do
Tokyo main, asyncfn main. And then I run cargos band.
Let's see what this gives us. So this is what we actually get when we run...
When we add the Tokyo main annotation to an asyncfn, it really just replaces your main function with a non-async main function.
It takes the body of the main function that you gave and turns it into an async block.
And then it creates a Tokyo runtime, enables all the features on it. It's a multi-threaded runtime.
And then it passes that body to block on. And so in other words, when you use Tokyo main,
all it turns into is that future just becomes both a top level task and it becomes the main thing that the runtime is blocking on.
And so when the main function returns, when the future that is in your asyncfn main is completed,
it returns ready, then the whole runtime is considered done and yields control to the actual main function,
which then returns and closes your program.
Do you know what the Tokyo approach is regarding the work stealing logic? Because when we think about work stealing, we think about costly context switches potentially.
So there's actually pretty decent documentation in Tokyo about exactly how it does work stealing.
If you go to Tokyo runtime, you scroll down a bunch and you go to multi-threaded runtime behavior at the time of writing.
This one talks about the different cues that they have and what the... policies are for when you look at the local queue,
when you look at the global queue, when you're allowed to steal. And they basically try to balance the cost here,
right? Because there is a cost to stealing, but there's also a huge benefit to stealing. And so you basically tweak the parameters until it hits something that generally works.
And then there's a bunch of configuration parameters you can set if that the default behavior doesn't work well for your application.
But you should really benchmark and make sure that if you change these settings, it's actually under like real load,
realistic load, that these numbers make sense.
While looping on pending check when available instead of park and unpark like threads. So it's not actually a loop on just if it returns pending then loop.
This is where we get into the idea of wakers and notify,
which is sort of the asynchronous version of park and unpark. So it is actually more sophisticated than that,
which we'll get to when we talk about resources a little bit later. how that mechanism works for choosing which futures have become ready.
So in general, the sort of logical way to think about it, or the theoretical way to think about it maybe, is that every worker has,
or every runtime really, has two queues. One of them is a queue of runnable tasks,
and one of them is a queue of not runnable tasks, or pending tasks,
if you will, but not runnable tasks. When you choose which thing to execute next, you only look at the things in the runnable task column,
in that queue. And those are things where you know that if you call poll on them, they're going to do some useful work.
The ones that are in the non-runnable queue, those you have no reason to believe the calling poll will make progress on,
and so you just don't do it. And then there's this mechanism that moves things from one queue to the other,
so from the non-runnable to the runnable queue. whenever there's a signal that indicates that this future,
this task may now be able to make progress. Again, we'll talk about that when we get to resources. And when you run a future from the runnable queue and it returns pending,
you move it to the non-runnable queue. So you don't try to execute it again. And if a worker thread runs out of the runtime,
runs out of thing in the runnable column, then it will actually park itself and only get unparked when there is some future,
some task that has moved into the runnable state again.
Why not spawn more threads than cores? So there's no real reason to have more threads than cores,
right? The number of cores is the amount of work you can do in parallel anyway, and so there's not really a benefit to having more of them.
CurrentThreadScheduler is also important if you're on a system without multithreading. So things like in WASI where there are no threads,
the CurrentThreadScheduler is the only scheduler you can have because there aren't threads, you can't spawn them. And so you can only execute in the current execution context anyway.
Thank you.
Okay. Ah, so kernel CPU scheduler,
so the operating system CPU scheduler also implement work stealing in the sense that like if one CPU core is executing a thread,
then, and another CPU core, you know, wants to choose something to work, it can steal the threads that are sort of locally locally queued on some other CPU.
Sort of, right? So basically the observation here is that threads on the OS level can move between CPU cores.
And that is totally true. It's not usually work stealing, but it does mean that the threads can move in this way.
The difference here is that the operating system is not aware of futures. It doesn't know about different futures,
different tasks. And so you could spawn one thread for every single task that you have.
The problem is every time the kernel wants to switch between threads...
It has to do like a kernel level context switch, which are fairly expensive. Whereas futures are a user space concept,
not just a user space concept, but a Rust based concept. And so you don't really need to go via the kernel.
So if you instead have your own scheduler that moves a single operating system thread between multiple tasks,
you can do that way more efficiently. And this is sort of the whole idea behind green threading.
Is the queue of tasks an OS-based event queue? No.
So these are actually just, you can think of them as linked lists that Tokyo keeps internally of tasks.
That it just, you can atomically add things to either queue. There's no operating system level part of those task lists.
The only place where the operating system comes in is in... spawning these different threads, the worker threads initially,
and some of the mechanisms for dealing with IO resources and timer resources, which we'll talk about in a second.


# Blocking
Okay, great. So now it seems like there's a sort of general understanding for what the runtime here is.
Let's then talk about blocking. So when you, given that we only have these n worker threads,
where n is your number of CPUs or whatever value you configure it to, Where you run into trouble is if you pass a future,
an async block, an async function to Tokyo, and one of the worker threads is executing that future,
and that future ends up calling something that blocks the current thread. Think of like it calls,
let's say it tries to read from the standard in from the terminal, but it doesn't do it through like an asynchronous std in handle from Tokyo.
It uses like std io, std io. stood, IO,
stood in, and just like a normal read call. Well, that's going to block the current operating system level thread.
This is not a Tokyo concept. You're just doing something that blocks. And at that point,
you're not just blocking the execution of this future. You're actually holding up that entire worker thread. It has no way to now start working on some other task because the operating system level thread is blocked.
And so this is why it is really important that when you have asynchronous futures that you pass to Tokyo,
that you don't have them do blocking calls. Because if they do, they're going to hold up the execution of any other task on that CPU core.
Again, because the number of worker threads you have is just the number of CPU cores.
And this is not a... ...sort of hard limit, right? Like, what does blocking mean?
Because reading from standard in just means you're blocking until the user presses a key. So it's not an infinite block.
Very few blocking things are. In general, what we're talking about is just you should avoid holding up a worker thread for a very long period of time.
And what very long means is sort of up to you. But imagine that you have a system where like a bunch of your asynchronous futures all do some kind of blocking call.
And those blocking calls take, I don't know, they take a second for whatever reason. If you, let's say you have two CPU cores,
both of them end up executing futures that have this one second block, then now you're going to have some periods of time where for
500 milliseconds at a time or something, no futures are being executed because both of the worker threads are held up in the blocking call.
So even though there are lots of other futures that could be making progress, there are no worker threads to pick up those tasks.
And so that's why when we talk about you don't want to block in an asynchronous context,
we don't really mean like you can't do an operating system call that might block.
We're talking about you don't want to hold up any worker thread for a prolonged period of time because that's going to mean not picking up other asynchronous tasks for that period of time.
So even if you do just very compute heavy work, so let's say you're not actually pausing, you're not blocking,
you just have like a matrix multiply or something that takes like two or three seconds, then that still has the same property that you're holding up the pickup of other tasks.
And so there isn't really a rule here for how long to wait. It just depends on your tolerance for futures not being executed.
The general rule of thumb I've seen is that if you have a
chunks of code in your futures, so that is code that executes between two await points with no awaits in this critical block.
If you have blocks like this that take more than 100 milliseconds to execute, you should probably move them to a separate blocking thread or use block in place.
Basically, you need to tell Tokyo, hey, I'm going to do something blocking now.
Do that in a different thread or... We'll talk about block in place in a second, but basically you need to notify Tokyo,
I'm about to do something that's going to take a while on this thread.
And there are also calls that are fast most of the time and sometimes really slow.
So you can imagine things like taking a lock. And I'm not talking here about the Tokyo mutex, we'll talk about the distinction in a second.
I'm talking about standard sync mutexes. If you take one of those,
usually it's super fast. Usually you take the lock, you get the lock, and then you do some work and you release the lock.
There's no real delay. But it could be that some other thread in your system has the lock and it's doing something that takes like 10 seconds.
And so now your thread that tries to take the same lock is actually going to block for 10 seconds while waiting for this other thing to release the lock.
And now you're holding up your thread for that period of time as well. And so... So...
What we're talking about when we say blocking is like, in the worst case, does this take too long?
Or longer than you're willing to wait for your other futures to be picked up?
There are two ways to deal with this problem.
One of them is this spawn blocking thing. So, Tokyo actually has two worker pools.
It has two thread pools. One of them is for the workers. So, these are the things that execute asynchronous tasks.
The other is a blocking thread pool. These are threads that are... expected to block.
They're just doing something that we know is going to block the current thread. We have a huge,
like the limit for how many blocking threads you can run is like super high. We don't really cut you off at any point.
There is theoretically a limit, but in practice, we assume that most of those are sort of sleeping or something.
And that pool is initially zero. Or rather, that's not quite true, all of the worker threads are actually in that pool,
but let's ignore that for a second. The blocking thread pool is a place where you can spawn your own threads using the spawn blocking thing.
And this is almost like using std thread spawn, that's basically what it turns into under the hood, except that Tokyo sort of remembers that thread for you and deals with it when it comes to clean up at shutdown for example.
But this is really just an interface for spawning, blocking synchronous threads.
The reason it's a little more efficient than using the standard thread version is because Tokyo actually knows to reuse these threads.
So if you call spawn blocking and something runs and then finishes, and you call spawn blocking again,
Tokyo might realize that, oh, there is already a thread in the blocking pool that doesn't have any work to do. we're just going to schedule this closure on that same thread.
And there's some policy for when threads get sort of actually shut down from this pool.
But so spawn blocking is one way in which you, if you know that you're going to do some kind of blocking work, you can use spawn blocking to take that closure,
spawn it on one of these blocking threads, and you get back a join handle that is a future that you can then choose to await whenever you want to wait for that thing to have finished.
And you can await it immediately after spawning. Right, that's fine because you're awaiting,
which means that you're going to return pending, which means other futures are going to get to run while that blocking task is executing on the other pool.
The other thing you can do if you're not willing to take, you know, whatever the thing you want to execute blockingly is,
if you're not willing to send that over to another thread, for example, because it's not send,
and you'll notice here that this indeed requires that the closure you pass in is send,
is you can use... Block in place. So in Tokyo task block in place,
this is a function you can call. You pass it a closure,
but this closure does not need to be sent. And what happens when you call this function is that Tokyo takes all of the state that the worker thread has and sort of saves it somewhere.
And then it, which you can think of as when it saves it somewhere, it actually sort of gives it away all of the state related to this worker thread,
and it becomes a non-worker thread. And it sort of gives Tokyo the green light to start a new worker thread in its place.
And so that way, this thread that was a worker thread stops being a worker thread and gives its worker state away to Tokyo or back to Tokyo.
And then it just continues executing your blocking code. And at this point, you're not holding up any other futures because some new thread that Tokyo spawns can pick up the worker state and then resume executing all of the futures from here.
So you can think of it as a spawn blocking. If you have this as the worker thread, it starts a new thread and moves your work over there.
Whereas block in place is you have a worker thread, you start a new thread that is the worker thread and you continue your blocking work here.
So they are sort of symmetrical, if you will, in that sense. Now, block in place comes with a couple of performance things to be aware of.
This is fairly subtle stuff, so I recommend reading the docs for block in place. In general,
only use block in place if you truly need to do some blocking stuff that is not send. But when you do,
that's what it's for. Okay, let's pause there before I continue on runtime stuff.
How much sense does this make?
How much overhead would using async in an embedded system with one core be?
There is overhead to using asynchrony, right? Because you now need to have this stuff that keeps track of the...
of the task and it chooses how to schedule them, the overhead is less than you might think.
And usually the overhead sort of, you can make up for it by the fact that you get to use green threading rather than going through the kernel if you do a lot of context switching.
But in terms of performance, like async is usually slightly slower.
The difference, again, for exactly the same workload, assuming that you can use...
Can't use threads. Where asynchronous shines is both if you have a huge number of tasks.
So think of things like you have a task for every connection and you're expecting, you know, tens of millions of connections or something.
Then doing them in a green threading approach rather than going through the kernel every time does end up increasing your overall performance.
The second reason, if you want to do things concurrently. So one pattern that we see a lot when used in asynchronous context is that you want to like wait for...
either a TCP packet to come in or the user to press something on the keyboard.
If you were in the threading model, you would need to have one thread that blocks on a read from the TCP socket and one thread that blocks on a read from the standard in,
and then you need some way for those threads to synchronize. In the async world, each of those is just a future,
and then you can use select, which we'll talk to in a second, so that one task awaits either of the two things.
And you don't actually need two separate threads that then need, you know, atomics in order to synchronize,
because they're happening on one task, so there's no synchronization needed. And very often,
programs end up wanting that kind of concurrency.
And it's actually, I would argue, easier to write programs in this way when you have a lot of these either-or events.
Does Tokyo create a thread pool and automatically spawn more tasks if needed or is this something one has to manually set?
When you start Tokyo by default, it will use the multi-threaded scheduler which starts this thread pool for you with n worker threads where n is the number of cores and with a blocking pool that has no threads.
And then it will start blocking threads as needed whenever you call spawn blocking.
Are you planning to talk about cancellation? Yes I am.
Is there overhead in promoting the threads? Right? So this is the, you call block in place. So you take the worker state and you give it away to Tokyo and then Tokyo is going to start a new thread that picks up that worker state.
And the answer is yes, there is some cost in starting up a new thread and then it picking up the state.
In practice, Tokyo is actually a little bit smarter than what I just said. What will happen is when you call block in place,
the worker state is saved in Tokyo. This thread runs its blocking code and when it finishes it'll look at has another thread picked up this worker state yet and if not I'm just going to take it back.
And so that sort of minimizes the cost for short blocking sections.
So you can often avoid this overhead of moving the execution.
What does the worker thread state contain? It's mostly the queues of tasks and a couple of other things,
but it's mostly the queues.
How much of the stuff we're talking about can be generalized to other asynchronous runtimes? A lot of this applies to other asynchronous runtimes as well.
The exact nature of how they manage blocking, whether they do work stealing,
whether they have pools and how large they are will vary, but many of the concepts translate.
So the programmer needs to be aware of the functions that may block. Can we instrument the code to send off blocking calls to separate IO workers?
Yeah, you can think of this as like, in asynchronous context,
you should not be doing significant amounts of work without calling await.
Like between any two await points, you should not be taking, you know, hundreds of milliseconds or many seconds before you call the next await,
because chances are you're holding up the runtime. Detecting this automatically is super hard. I'm not going to claim it's impossible,
but I haven't seen any sound strategies for doing it automatically, which means that it becomes a sort of manual task.
There are tools like Tokyo Console will highlight when these kinds of things happen.
But you still then manually need to figure out why and how to rectify.
Doesn't it still need to be using an OS level asynchronous primitive in the end? Yeah, we'll talk about that when we get to resources.
Okay. If a worker thread is blocked by synchronous code,
wouldn't it be fine because we have other worker threads? It's fine until it's not, right? So if you have a bunch of futures or want many instances of a single future,
all of which may block at some time, what is inevitably going to happen is you're going to get at some point in time,
all of the worker threads are in the blocking section. And so there are no other worker threads anymore. So that only works if you generally don't run into the situation.
But, but... You generally don't want to rely on that being the case, that there's always another thread.
Okay, there are then three more things to talk about with the runtime. The runtime is the biggest segment of this,
I think. The first of them is shutdown. So when you have a runtime,
down here, when you have a runtime, the general rule is that whenever the future that is passed to block on resolves,
finishes. Then the runtime yields control back to its caller. And like we saw in the expansion here,
when you use Tokyo main, that means that when block on returns, the whole main function returns. And so at that point,
the program exits. And when the program exits, the operating system shuts down all of the threads.
That may or may not be what you want. This means that when the main future exits,
everything else just gets dropped. There are ways to try to mitigate this by having the main thing that you do in the main future actually wait for,
for example, all the join handles so you've gotten back from spawn. So that you make sure that you don't actually exit until all the things you spawned have finished.
There are some tools that can help with this. We can talk about that later on when we talk about utilities. But you also have the means to shut down a runtime by using shutdown background or shutdown timeout,
which lets you give a grace period as well. When you call shutdown background, what actually happens is that...
the moment things that are spawned yield, they will then be dropped.
So it won't immediately shut down because there might be futures that are still in the call to pull.
But when the call to pull for every spawn future returns, that future or each future as it happens to them will then be dropped and not executed anymore,
even if they haven't returned ready yet. And so the moment you call shutdown background,
The number of tasks is just going to steadily decrease until eventually there are none left.
And at that point, shutdown background will return saying, okay, there are no more futures now, you can safely shut down.
This does mean that they are not guaranteed to run to completion, right? They will just be dropped.
They will be canceled, like we'll talk about a little bit later.
But these are, in some sense, a graceful way to shut down. If you instead just return from,
so we had that here, if you instead just return when the main future returns or resolves,
then at this point, after you finish main, after you return from main, all of the threads are just killed by the operating system.
What that means is the drop method will not be called on the future, so they don't get a chance to clean up after themselves.
That might not matter to you, but if it matters to you, chances are you want something like shutdown background as a call at the end of your main to make sure that you actually wait for the spawn task to also finish and at least get dropped.

# ƒSend bounds and LocalSet
The next thing I want to talk about is send bounds. So Tokyo does have a way to run futures that aren't sent.
So these are things where once you start executing them on a given thread, you are not allowed to move them to another thread.
In fact, once you've constructed them, you're not allowed to move them to another thread. This obviously sort of doesn't really work with work stealing,
right? Because work stealing assumes that you can move threads to another task.
So Tokyo has this type called a local set, and a local set is a set of tasks that are all guaranteed to be executed on the same thread.
So the idea here is that you create a local set and then you can spawn things using the spawn local method onto the local set.
And that adds the future to this local set. But again, the local set does not send. It's guaranteed to not move between threads.
So these spawn locals creates additional top level tasks for the local set. And then you can await the local set at the end.
And that will run all of the futures that have been spawned on that local set. until they all resolve.
The trick with local set though is that it cannot be used anywhere. You can't just like arbitrarily use them somewhere deep down in a future somewhere.
And the reason to that is if you can start the local set deep down in a future stack, Tokyo doesn't know that there's a local set there.
So like when it calls, when all it sees again are the tasks at the top level.
So up there, it doesn't know that this thing can't be moved between threads because that would move the inner local set.
And so local set can only be used in top-level tasks. It can only be used as you'll see here on it passed to block on or yeah so remember Tokyo main basically ends up calling block on right so here we're allowed to use the local set because we are in a block on
You can also use this run until which is sort of a block on on a local set but let's ignore that for now.
So you can just await this inside of a block on and if you need to be able to use it somewhere else what you actually need to do is you need to spawn your own thread that owns the local set.
And it locally executes with its own current thread runtime and then you send tasks to it over a channel.
It's a little awkward, but again it is because it needs to be known at the very top level that this task is not safe to move between threads.
If you truly need something like this, if you need non-send futures, I recommend you read the docs for local set because there are a bunch of nuances here that we don't quite have time to go into.
But know that this mechanism does exist. And then the last thing that I want to talk about when it comes to the runtime,

# tokio vs std Mutex
and then I'll do questions after this last one, is the difference between a Tokyo mutex and a standard library mutex.
So, and again, here you see there is a giant section that talks about this in the docs,
but the standard library has a standard sync mutex, and Tokyo has its own Tokyo sync mutex.
And you might wonder, well, why do we have both? Why do they differ? Well, the Tokyo mutex...
The sort of main difference at a high level is that the lock methods on the Tokyo mutex are asynchronous,
right? So if you call lock, you then call dot await. Great. That seems straightforward. And so you might then assume,
okay, I should just use Tokyo mutexes everywhere. That's not quite true.
And this ties back to the nature of how these runtimes work and the nature of blocking.
So when you take a standard library mutex, what you're doing is you might block the current thread,
but let's assume that like this mutex is only held for a short period of time in general.
So you take the mutex, it doesn't really block the thread because you got the mutex right away.
So you do a work and then you release the mutex. There's not really a lot of risk there, because as long as your critical section is not very long and as long as your mutex is not very contended,
so you never wait like, you know, a second for it to become available, that works fine,
right? The fact that this is using a synchronization primitive at the OS level, Tokyo doesn't really care.
It just cares how much time you spent in there, how much time you spent holding up that runtime thread, that worker thread.
And it turns out that a standard library mutex is actually really efficient.
Like just performance-wise,
it is an efficient synchronization mechanism. The Tokyo mutex, in order to make lock and unlock asynchronous,
is actually fairly inefficient. And so when you can use a standard library mutex, you probably should because it has lower overhead than the Tokyo one.
Where this gets subtle and the reason why Tokyo provides its own mutex is not just because it's nice to be able to call lock.await,
but it is because if it might take a long time for you to get access to the mutex to lock the mutex,
then you're holding up the worker thread, that's not okay, so you need to use a Tokyo mutex.
Or if you take the lock and then you call await on something before you unlock.
Because, so the way this would actually look is, imagine in here,
right, I do something like mutex.lock.await.
So that gives me back a guard. That's not how you spell guard. And then I do some func.await,
and then I drop the guard.
Right? If I do this, the problem you run into is that some function here could take an arbitrarily long amount of time.
Maybe it tries to read from standard in. Maybe it tries to wait for a TCP packet. Maybe it tries to send a TCP packet to a peer that's disconnected.
Whatever it might be. This can take an arbitrary amount of time. What that means is,
now that you hold the lock... Um... For the period that this blocks,
what will happen is that you're holding up.
So if this was not a Tokyo mutex,
if it was a standard library mutex, then during this time, you're still holding the mutex,
even though you've yielded and you're trying to now execute other tasks. Imagine that there's another task on this runtime that now tries to call mutex lock.
It will now be blocked. forever, or at least until this await call returns, because it can't get the lock because it's held up by this await call.
And it actually gets even worse than that, which is that a mutex guard in the standard library is not send.
But if you're holding it across an await point, that means that this entire future is not send because it might need to hold on to that guard until this await comes back because who knows,
we might do guard.foo equals 42 down here. So during the await call it needs to hold on to the guard in here,
and guard from the standard library is not send, and therefore this whole future is not send, and therefore you can't use it with Tokyo.
And so there's some really tricky situations that come up the moment you have await calls inside or during which you're holding a standard library lock.
The Tokyo mutex does not have this problem. The Tokyo mutex guard that you get back, first of all,
is send. So if you hold it across an await point, then it's fine because the mutex guard is send,
and so the future remains send. But the other reason is because when you call await here,
Tokyo knows that this future is actually holding onto that mutex.
And so if another task comes along and tries to lock that same mutex and it calls await,
right? Then Tokyo knows, oh, I should put this task to sleep and pick up something else instead.
Instead of, in this case, where it will block that worker thread and therefore block any other tasks that might be executing.
So the rule, the general rule here is that use a standard library mutex if you have short critical sections,
like the stuff you do while holding the lock is short and there are no awaits in the critical section.
For anything else, use the Tokyo mutex.

# Runtime questions
Okay, I think those are all the things I wanted to say about the runtimes. Let's do questions about the runtime before I move on to IOResources.
Um... Might it make sense to have two separate Tokio runtimes and spawn tasks of those runtimes to increase performance,
assuming we assign separate CPUs to those runtimes? it usually should not make sense to have two separate Tokio runtimes.
Usually you're better off just allowing work stealing across your entire task pool. There are some exceptions to this.
So for example, if you have an application where you have some tasks that are all very similar to each other and some other tasks that are all very similar to each other,
you might get enough and the performance requirements are strict enough, you might get some benefit from having one task pool that only runs futures of a certain type and another one that only runs futures of a different type.
so that you get better cache performance on your CPUs. But this is one of those like,
you should be damn sure with performance benchmarks and profiling that that is actually worth the complexity.
Because once you manage multiple runtimes, it's really easy to shoot yourself in the foot.
So I would very much default to just having one.
Why do they not use implicit return in the Tokyo main macro when you expand it?
You know, I don't know. I don't know why they don't use an implicit return here.
I don't think there's a great reason for it. Is local set slower than regular tasks in the current thread runtime?
No, local set does not, as far as I'm aware, come with a performance penalty.
When sending through channels you probably need fuchsias to be sent anyway. This is for local set if you want to sort of spawn it on a separate thread.
The idea here is that there are cases where you can make the description of what to do be sent,
but have the execution of that thing not be sent. And then you should be able to still use this channel and local set trick.
And that's it.
I've heard the term green thread but never fully understood it. Is it just a task in a task queue slash event loop?
Yeah, you can think of a green thread here as basically being equivalent to a task, the way I've described it so far.
Are there any tools you can use to discover or visualize how often your async tasks are being blocked and if your CPU bond tasks should be awaiting more?
Yeah, so that is the tool called Tokyo Console. So if you look up Tokyo Console,
this thing right here. It has this view under extremely cool and amazing screenshots that shows you for all of the tasks that Tokyo knows about,
how much time has been spent busy, how much time they've been spent sort of scheduling,
how much time has been idle, how many times have they been pulled. And it also has information about tasks that have been that are blocking the runtime for a longer period of time.
And it shows these little warning icons for those. So that's a great. Great tool for learning this information about Tokyo.
Do IO resources include TokyoFS? Yes it does. Is the Tokyo lock guard dropped when another await is called in the future?
No. In fact it would be unsafe to do so. So if you had guard is mutex.lock.await something.await drop guard.
If this await caused this guard to be dropped, that would mean you release the lock and then call this thing.
But that's not the semantics of this code, right? The implication here is the lock is held during this entire call.
And so it's not okay for Tokyo to drop this guard sort of implicitly just because you're waiting.
When using a single-threaded runtime and spawn blocking, it still spawns a separate OS thread. Yeah, so spawn blocking will always happen on a separate thread.
I forget whether with the current thread runtime there even is a blocking pool.
I think there is, but I'm not sure. You would have to double-check that.
There are no task priorities in Tokyo, so you can't say, this task is more important than this task, so schedule it more regularly.
That's not a feature that Tokyo currently has. Can a worker thread steal from a currently blocked other worker thread?
It's complicated. So if a worker thread is currently blocked, because the future called some kind of blocking system call,
the queue of that worker can still be stolen from, but the currently executed task,
the one that is blocked, cannot be stolen because it's currently active in that worker thread.
There are also some other things that are... I don't want to say hard to steal, but there's some optimizations that mean certain other tasks may also not be stealable.
Nope, I have not switched to Emacs. Can there be a deadlock or similar if you share the same task between multiple runtimes?
You cannot share a task between runtimes. So a task fundamentally is a single instance of the future trait.
And so there's no way to share that, right? Whoever owns it, whichever runtime owns it, is the one that will be executing it.
So there's no way for one task to be handled by multiple runtimes. If you do have multiple runtimes,
you could get into a deadlock situation between the tasks of the two runtimes.
But that's sort of a different concern. Since Tokyo shares the work across threads and the futures need to be sent,
does this have overhead every time a future is shared across threads? There's no real overhead of futures being work stealable because Tokyo will generally avoid stealing futures unless specifically a core is running out of,
one worker thread is running out of work and another worker thread has too much work for it.
And in that case, you would rather it be executed, even if it's with a little bit of overhead, than just have one thread execute and the other one just sit idle and do nothing.
So usually it's worth it, even though there is a small overhead when you choose to steal.
Is there a way to shut down Tokyo Runtime and wait for all current pending tasks? Not to my knowledge,
but the way you do this is you implement this yourself. And we'll talk about some of the ways in which you can do that.
There are some utilities in Tokyo Util, for example, which is a separate crate that helps you do this kind of stuff.
Is parking lock mutex faster than stood mutex? I'll leave that for a separate conversation because it doesn't have to do with async.
Um... What are the advantages of using the local set?
It's that it allows non-send futures.
What's the difference between Tokyo's instant now and the standard library instant now? They're the same instant type so there's no difference.
I got your email, I've just been busy.
Can you use Tokio resources with a different runtime? We'll talk about that when we get to resources. Um...
Okay... Yeah, so if you're running on particularly newer machines that have large numbers of CPU cores,
the CPU cores might physically be located far from each other on what these things are known as NUMA nodes.
And stealing across NUMA node boundaries can actually be quite expensive,
much more expensive than it being worth it. And again, Tokyo tries really hard to not steal unless it's necessary.
But there might be an outsized cost on these high CPU core count machines. for stealing.
There isn't a NUMA-aware scheduler in Tokyo at the moment, but that is something that I'm guessing Tokyo will probably grow over time.
Tokyo does not have its own instant type.
Great. Okay, I think it's time to move on to resources.

# tokio resources
So, we've talked now about the runtime, but the runtime is just about executing futures,
like calling the poll function and nothing else. In reality, you generally want these futures that you spawn to interact with the real world.
You want them to do I.O. Things like... Okay, wait,
someone is insisting that Tokio has its own instant type. It is just a standard time instant.
There is no difference. So yeah, you're right. They do have their own instant type, but it's just the standard library instant type.
I believe, yeah, so here, if you look at the note, that explains why.
Okay, so Tokio has these resources that are intended for interacting with I.O.
This is stuff like TCP and UDP, file system, other processes,
signal handlers, all of that kind of stuff. I'm not really counting synchronization primitives like channels here,
but they are in some sense resources as well. And the-
The way to think about these is that these are provided by Tokyo because if you,
let's say you have a TCP stream, right? And you want to read from a TCP stream.
So you have one of these and you want to call...
Where is... AsyncRead. So asyncRead is the sort of underlying trait that Tokio uses for things that can be read from in an asynchronous context.
So you have a poll read that reads stuff out of, you know, whatever type implements it, like TCP stream,
into a provided buffer, and then returns poll to either say, okay, I successfully read a bunch of bytes,
or it returns pending saying, oh, I tried to read some bytes, but... there weren't any bytes available.
And so, you know, yield in this future and come back later. And ultimately, if you think about the sort of stacks of futures that make up a task,
generally near the bottom is where you're going to have these resources that ultimately,
some giant future, whatever its construction might be, at the bottom you have things like read from a socket,
write to a socket, read from a file, write to a file, read from standard in,
write to standard in, or wait for a timer. So you have these resources all the way at the bottom.
So that are in, they're not actually futures, right? They are things like async read that have a future compatible interface,
right? So they have a poll method, they're given asynchronous context and they return poll,
but they do not themselves contain futures. And all of these leaf resources, these leaf future-like things or asynchronous primitives are provided by Tokyo.
And the reason for this is because whenever a future is blocked, sorry,
I should say, whenever a future is pending, it's yielded, it claims that it can't do any more work,
then that is usually because one of these resource level calls returned pending.
That is the ultimate reason why the future can't make any more progress. And that also means that the way in which it can start to make progress again is if one of those resources suddenly becomes available.
Like, you did a read from a TCP socket and now there is data available on that socket.
Or you tried to read from standard in and now there is data available. Or you tried to wait on a timer and now that timer has expired,
like that time has passed. Or you were waiting for a process to finish executing and now that process has finished executing.
And those things, those events that indicate that, okay, now you should try this future again,
is... the connection between these IO resources and the future traits that happens through this context thing.
And you'll see this concept argument is in async read. It's also in all of the low-level resource primitives that have these poll-like calls,
but aren't themselves futures. It's also passed to future, right? So the top-level call to poll on any given task,
so on any given future, is given a context. And that context is passed over. all the way down the chain of futures.
And this mostly happens without you thinking about it. So when you call.await, it de-sugars to get the current context,
pass that into poll, and then it gets threaded all the way down on your behalf,
which means that the context to get passed in at the top to a task is the same thing that makes it down to the resource at the bottom.
And context is defined in the standard library, and the main thing that a context has is this waker thing.
And a waker only has one real method that matters,
which is wake. So a waker gets passed all the way down to the resource. And the idea is when a future returns pending,
so the whole task becomes pending, it's moved to the sort of non-runnable queue.
And when the waker that was passed into that future, when it returns pending,
when someone calls wake on it, that means this future might be able to make progress again.
And so this is a signal to Tokyo that this future should be moved from the non-runnable queue into the runnable queue and then get picked up by a worker pool thread again.
And so you might wonder, well, who calls wake? And this is where, when we looked at runtime.
you'll see up here that runtime has this IO event loop. And this is separate from the scheduler.
The IO event loop and the scheduler, they don't run on different threads necessarily, but they're sort of distinct components or services that are provided by the runtime.
The scheduler chooses what to pick up next. And the IO event loop is looking for these events that might mean that some task can make progress and then calling wake on the appropriate waker.
If we take an example of something that ultimately reads from a TCP stream, the context, the pull method gets called by the Tokyo runtime scheduler.
It passes in the context that represents the current task. It's passed all the way down to the async read call on the TCP stream.
That tries to read from the socket and gets told there's no more data. What it will do is then internally in Tokyo,
internally in the TCP stream type, it'll take the waker in the context that was given into Paul Reed,
so the waker that was inside here, and it will save that waker inside of a little secret storage area next to the TCP stream's file descriptor.
And all of that is stored inside of the IO event loop inside Tokyo. And the IO event loop,
whenever a worker runs out of stuff to do, or in reality, every now and again,
this event loop is going to be looking for events on all of these little secret compartment file descriptors.
So look for, has anything happened on any resource that I know about and have a waker for?
And if so, I will go to that thing and call wake on it, which ultimately then kicks off all this machinery of moving things into the runnable queue.
And so this is what I mean by the runtime does not really have a loop that just calls pull on every future or every task,
even if they return pending. What actually happens is it calls pull on all of the ones that are runnable.
And when it runs out of things that are runnable, it will call into the IO event loop.
and see whether it can make some of them runnable. And in reality, it's actually more sophisticated than that.
It will periodically check whether things should have become runnable so that it doesn't like starve out the ones that are waiting for IO.
So that's the sort of low level connection here to resources. Let's see if that makes sense first before we keep going.
Can Tokyo react to mouse or keyboard input? In theory, yes.
Tokyo doesn't actually mandate that you use Tokyo-provided resources. The rule is just that if you implement your own resource that represents,
for example, mouse input, you need to provide something.
some resource you implement yourself that will take the waker out of the context that your pull method is given for,
let's say, mouse, and make sure that when the mouse moves, the wake method on the waker is called.
And Tokyo doesn't dictate that you have to do this in a particular way. And this isn't even a Tokyo requirement. This is the requirement for futures more broadly.
So if we go to look at future, You'll see here, the core method of future poll attempts to resolve the future into a final value.
This method does not block if the value is not ready. Instead, the current task is scheduled to be woken up when it's possible to make further progress by polling again.
The context passed to the poll method can provide a waker, which is a handle for waking up the current task. That is to say that it is now ready to make progress.
And the contract here is that...
Let's see... Okay... Yeah, the pull function is not called repeatedly in a type loop,
instead it should only be called when the future indicates that it's ready to make progress by calling wake.
And there are a bunch of different ways to do that. The way that Tokyo does it internally for most of its IO resources is to use something like ePoll,
which is an operating level mechanism for saying, I have all these file descriptors,
all these, you know, UDP, TCP sockets, files, whatever. Tell me if any of them become readable or writable.
That is if they have bytes available or are ready to be sent on again. And so Tokyo will handle that sort of
IO loop for you.
Would it be possible to implement zero copy using async read?
It depends what you mean by zero copy. So, do you count a copy from kernel space to user space?
Because if so, yes. The tricky part is when you really want to give a buffer to the operating system and say the operating system gets to write into this.
And this is sort of IOU ring kind of setup. This interface doesn't let you do that, although there are experiments with this and I think it's the Tokyo
Uring Crate that tries to figure out how this might, what this might look like.
Is there a new waker given to each future or async read poll? I'm confused about who creates the waker.
The waker is created by Tokyo when it calls poll on the top level task.
And it's actually pretty cheap to create a waker. It's really just a V table,
like a virtual dispatch table, a list of function pointers that it constructs. And the idea is that what wake actually,
what sort of gets stored in the waker is really just a pointer to the task.
so that Tokyo knows which tasks, like when wake gets called, which task was actually awoken,
and a function pointer to the Tokyo code to execute when wake is called.
So it's executed by, it's created by Tokyo at the top before it calls poll.
So it's not actually, it's not like a waker is created when a task is created and it's stored next to the task.
It's actually created on the fly for every call to poll, because it's so cheap to create.
The implementation details of waker are actually kind of interesting, the vtable stuff. It feels like waker could also have been a trait instead of a concrete struct.
So the reason why waker isn't a trait is because it also needs to implement drop.
Let me see here. Local waker.
Where is the type I'm after? Raw waker.
So you see that there's implementation of drop for waker, which means that you...
it needs to be able to implement drop so that the runtime, for example, can keep track of whether a waker is still around and do cleanup.
But the drop trait is not object safe because...
Well, it's complicated. No, sorry, clone, sorry, it's clone, it's not drop,
that's the problem. Clone is the problem. So clone is not object safe because it names the self type,
which object safe methods are not allowed to do, or object safe trace are not allowed to do. And so as a result,
if you made waker a trait, the trait would not be object safe, which means that you couldn't have a box din waker or an arc din waker.
Which is a problem because that's very often what you want. And so the alternative would be that you have a generic everywhere for your waker.
But that gets super annoying because now you have a generic for every future going all across the system design.
And so instead what they did is they have this raw...
this raw waker API that is the V table that is a V table that includes what call should do.
And the clone implementation here returns a new raw waker rather than self to get around this problem.
There is actually a waker trait as well,
which is this task wake trait, which basically forcibly wraps it in an arc to get around this problem.
But there's a little bit of like, there's a reason why it's not just a trait.
Why does Tokyo have async read and async write instead of std? Ah, so the reason for that is because if you look here at the read trait.
So the read trait in the standard library has two problems. The first of them is that it takes a mutable reference to self,
whereas the async read takes a pin of self. This is important because
I don't want to get into all of pin, but in order to have futures be able to have local state,
that is like local stack variables inside of a future, you need to have a pin of self.
to be able to do that soundly. And read does not take a pin of self and therefore wouldn't work in a future context.
The second reason is because we want this poll type that indicates whether the read did something or failed to do something,
or rather couldn't do it yet. The difference between I'm done and I have more work to do.
the signature of read in the standard library does not have that type. Now, IO result,
technically, if you look at the error, there's an error kind and there's an error kind called would block.
And you could look at that, but that gets really, really ugly to have to parse out exactly which error kind in every call to read.
So that's the reason why this is a separate trait.
So is the scheduler and the IO event loop kind of always waiting for each other?
Sort of, right? So the schedule doesn't wait for the IO loop. Instead, you can think of it as the runtime is running the scheduler and the IO loop at the same time.
The scheduler just chooses, the scheduler is to run locally for every worker thread,
and it just chooses which future to run next. And the only thing it really knows about is...
whether tasks are runnable or not runnable, and it only considers running the ones in the runnable column,
or the runnable queue. The event loop, its job, is to move things from non-runnable to runnable.
by calling wake so it doesn't actually know about these two queues all it knows is here's a bunch of stuff that we're waiting for and here's the method i should call whenever they become whenever a resource becomes available it just so happens that the the
code that gets called when something some resource is now available happens to do the move from non-runnable to runnable queues so they actually end up being relatively discrete but cooperating services
So who calls wake? The IO event loop calls wake.
Or you can call wake. So this is the idea that you can implement your own resource and all you have to do is you need to have a method that looks kind of like this.
So if we look at something like, if we look at the Tokyo channel,
for instance, and we look at receiver, you'll see that there is a poll receive method.
And this one is not pin. for reasons that I'm not going to get into,
but it has a poll method that kind of looks like the future poll method. It takes a context and returns a poll.
And so this is really the only signature that you need to provide for your own resource type. And then you just need to guarantee somehow that if you ever return poll pending instead of poll ready here,
then you have some way to guarantee that the waker inside of this context will eventually be called wake on.
when you might be able to make progress. That's the contract you have to fulfill for your own resources.
Okay, so there's then a question of, okay, if async read and read have to be different, why isn't async read in the standard library?
The answer to that is a little more complicated, but it's basically because we don't know that this is the,
and I say we, like we as a community, don't know what the correct definition of the async read trait should be.
This one works pretty well, but it doesn't work so well when you get to sort of next generation.
OS-level interfaces for asynchronous I.O., things like IOU Ring. It's not clear that this is actually the right interface for async read once you want the ability to do
IOU Ring-type stuff, which is more efficient. And so it's mostly just a matter of the moment we're sure that this is the right interface,
then it will move into the Zend library, and we're just not sure yet. In some sense, we're sure for now,
but we're not sure we want to commit to it permanently.
There's also this read buff stuff, which takes a mutable reference to a slice of U8s.
I'm not going to dig into that here, but this is another thing where we're not quite sure what the interface should look like.
Is the IO loop local to each thread? No.
The nature of exactly how the IO event loop gets executed, I'm not going to get into here because it's not terribly important.
And also it's kind of complicated. And also it's not guaranteed to stay the same. This is sort of considered an internal implementation detail of Tokyo.
So the I-O event loop periodically runs and checks if a wake needs to be called. Kind of.
The way I would probably think about it is that the I-O event loop is kind of always running, rather than thinking about running periodically.
So it should pick up on data being available on a socket, for example,
basically immediately. And it should call wake basically immediately. It's not like you have to wait for the next time for it to be time to do an I-O check.
Okay, great. So let's now talk about the different...
IO resource types in a little bit more detail, now that we know how the structure of this works.
The IO types in Tokyo look mostly like the ones in the standard library, right? You have TCP listeners,
you have UDP stream, you have UDP socket, you have TCP stream,
you have, you know, Tokyo FS, which has things that are,
you know, files. directories, readdir, you have a bunch of the same methods that you have inside of stdfs.
Similarly for under process. You'll see that you have a child,
you have command, which is the builder. All of this looks very much like the standard library, except of course for the fact that all of the methods are async,
and instead of implementing read and write, they implement async read and async write. So it's really just sort of an async adaptation of mostly the same interface.
But there are a couple of differences and also surprising similarities to be aware of.
So let's start with Tokyo FS. So, this is Tokyo's implementation of,

# tokio::fs nuances
sort of, asynchronous implementation of ways to access the file system. Now, the file system is actually a little bit weird because a lot of operating system operations on the file system do not support asynchronous access.
There just isn't an asynchronous interface to working with files.
This may be surprising and kind of annoying, but it is the case. And so, Depending a little bit on your operating system and depending on the current phase of the moon,
Tokyo will often use a blocking thread, like a dedicated blocking thread or set of blocking threads,
in order to give you asynchronous file system access. There's a little bit of discussion about it here.
But what it means is for some of the file system operations, even though it looks like an asyncfn,
what actually happens is that it has a blocking thread that's going to do the real operating system invocation for you,
and it does a sort of spawn blocking, and then it just waits on the join handle as a result. This works,
and it does give you an asynchronous interface, but it actually means that for Tokyo FS specifically,
it can be a decent amount slower. than the standard library implementation, or rather I should say the synchronous implementation.
And that is because the operating system just doesn't provide an asynchronous interface. So we kind of need to fake one by sticking a bunch of overhead on top of the synchronous interface.
You still usually want to use the asynchronous interface because it gives you things like select and join and you can combine them with other asynchronous futures and you can stack these inside of futures and not worry about blocking the executor for example or the worker pool thread.
So It's usually still worth it, but you should be aware of this cost.
And as this also talks about at the top, there are some cases where if you're seeking the sort of highest level of performance,
it can actually be worthwhile to turn your asynchronous file system stuff into a synchronous thing that you run in a spawn blocking closure.
So that you get the sort of high performance for all the synchronous file system operations. And then it's just the final result that gets turned into something that's async.
That can be a path you want to take for something where performance of the file system operations specifically are on your critical path.
That's the main caveat about TokioFS. Apart from that, it mostly looks like all the things you would expect.
Yeah, so there's some discussion in chat here about what the ace and green and right traits would look like in an IOU ring world.
And the answer is they just wouldn't be there. It just is completely different. And again, if you look,
if you're curious about this, there is no, that's not what I want. I want
Tokyo IOU ring.
Tokyo U ring. Tokyo U ring.
Tokyo U-ring, which is basically an attempt to figure out what would Tokyo look like on top of
IOU-ring for sort of higher performance asynchronous interaction, specifically on Linux.
And it just doesn't have async read and async write. They're just not there. So worth taking a look at if you're curious about this.
IOUring is a high-performance asynchronous I.O. interface that's available in Linux 5.10 and later.
And so this is why it's not the standard thing that's being used, because it's a very new feature of only certain Linux kernels.
Its API also looks completely different from the older APIs that are used for asynchronous interaction,
which allows it to get higher performance. but you have to match the interface well in order to get that performance benefit.
And so hence, there's still some experimentation on what should abstractions on top of IOU ring look like in order to maximize the chance you get those performance boosts.
Yeah, so IOU ring would replace at least part of the IOU event loop.
Okay, so that is Tokyo FS. Let's then...
Yeah, so Windows I.O. is also completion-based, which is, so I.O. U-Ring is a completion-based rather than,
what's the word? It's escaping me now. The way that Linux does asynchronous I.O.
is different from how Windows does asynchronous I.O. Windows has a completion-based I.O. model,
which Linux does not. I.O. Uring, which is a Linux thing, does have a completion-based interface that's different from the one that's currently on Windows.
But it might be that we actually end up in a world where the completion-based interface is what's used on all platforms down the line.
But how that looks like is probably not async read and async write, but something else. Hence why it's probably a good idea that we haven't put it in the standard library.
I guess readiness-based or polling-based as opposed to completion-based is the wording.

# tokio::process nuances
Okay, next thing I want to talk about is Tokyo Process. This is sort of the equivalent to
Stood Process. It looks very similar. Command is very similar for constructing things.
The main two differences, there's one similarity and one difference to be aware of.
The similarity is that... When you drop a handle to a child process,
and this is also true for the standard library version of child, when you drop it, the process is not terminated.
And this again is similar to stuff like... If you have a join handle,
both in the standard library, you spawned a thread, and in Tokyo, you used Tokyo spawn.
The handle you get back, if you drop it, the future that you have a handle to, or in this case, the process that you have a handle to,
continues executing, even though you dropped your handle to it. This can be surprising,
even though it's the same as the standard library, it can be surprising there too, because in general, what we expect in futures is that when you drop a future,
it doesn't do any more work. It's sort of, you drop it and then it cancels, like it stops doing everything it was doing.
That's not the case when you have a child handle in there. When the future gets dropped, when the future gets cancelled,
the child process will continue executing. There are ways you can change this behavior.
There's a kill on drop method you can call on command that changes this behavior, but you should be aware that this is the default.
And again, it is the default to match the standard library. There's also some stuff about just Unix processes that are complicated.
I'm not going to talk about this too much, but you should just be aware of how Unix handles processes if you try to use this in an advanced way.
The other thing you should be aware of is that on child, which is the sort of reference to a child that you might have.
there is an asynchronous function wait that you can use for waiting the process.
Child itself is not a future. So you can't await, you can't,
if you spawn a process, you can't just await the child in order to get its output.
Instead, what you have to do is you have to call wait or wait with output and await that thing.
It's just, it's not super complicated. It's just a nuance to be aware of here.
Okay. So that's the only thing I want to talk about process. You should just be aware of this particular idiosyncrasy maybe.

# tokio::io things
Okay. Two more things here. See this is why I have my notes because there are a lot of things to touch on.
When we talk about IO. So there's async-read and async-write which we've talked about,
but there's also the async-read-ext and async-write-ext extension traits.
These are the things that provide you with convenience methods on top of things that implement async read and async write.
Things like read this many bytes or read a single thing of this type or read to string or write all,
which is when you do a write, just like in the standard library, when you write to a socket,
it's not guaranteed that it writes the entire buffer you give it. It might only write the first eight bytes or something.
And you might need to do it in a loop. write all will make sure you actually write all of them before the future resolves.
So async read x and async write x provide futures on top of the low level interface that async read and async write has.
So again, the async read interface is just a pull read method. And you generally don't want to call those methods directly from a future.
And so instead, you would use dot read, which returns a read,
which is a future here that you can await instead.
And there's also variants of this for buff read, which is if you have similar to buff reader and buff writer in the standard library,
you have the same thing in async world where if you have an async buff read,
so something that internally knows how to buffer reads, you can do additional things like split by line, for example,
and get futures for those. So you should just be aware of these extension traits that make it easier to work with IO resources.
rather than directly working with the async read and async write traits. And these are implemented for anything that implements the underlying traits,
right? So if we go look here at read, for instance,
just to see what it looks like. Read.
Oh, boy.
I'm going to regret this aren't I? I want to go... Maybe it's just actually imported somewhere.
Yeah, it's IOUtilRead. Let's go to the repo and see.
Tokyo. Source. IOUtilRead.
Read. So the read method, like the actual thing that gets called when you call read here inside of async-redext is this read type.
The read type just holds a reference to the reader and implements future. And what it does is it creates a buffer and then calls pull read and then returns you the stuff that got back.
So it's just a convenient wrapper so that you don't have to call pull read yourself. There's no magic to it really.
It is just the X methods are just so that you don't have to write a bunch of code to call the low level future methods yourself.
And similarly for writes. The other thing you should be aware of when it comes to I.O.
is that for a lot of these methods, and again we can look at,
we can just look at async read I suppose, you see it takes a mutable reference to self. This is the same thing as in the standard library.
If you try to read from a TCP stream, you need to have a mutable reference to that TCP stream.
And what we often see, unfortunately, is that people have a TCP stream or something,
and it's in some larger struct somewhere, and you have two different futures that both need to access the struct,
and so you end up doing an ARC mutex TCP stream, so that these two different futures can both access this underlying IOR resource.
This works, but it is very often not what you want. In general,
if you find yourself putting an IO resource behind a mutex, you can probably do better.
Like you're probably leaving performance on the table because now you're forcing these threads to interlock on the mutex,
to sort of interleave their execution on the mutex with the other work that they might do.
In reality, this is a really good fit for things like the actor pattern. So the idea here is that instead of storing that TCP stream inside of this deeply nested struct with a mutex,
you actually spawn a separate top-level task that owns the TCP stream. It doesn't need a mutex at all.
And it has, let's say, a channel that people can send things for it to write into or send messages for it to then read out stuff from that TCP stream.
The idea being that you have one task that owns the TCP stream and can access it without going through a mutex without the overhead that that entails,
and then do the work over it and then choose to either, you know, fan in requests that need to be written or fan out things that have been read.
So very, very rarely do you actually want a mutex TCP stream, just so that that's something you're aware of.
Okay, I'm going to pause there. The next thing I'll talk about is the connection between async read and async write and sync and stream.
But let's take questions here before we continue.
Yeah, edge triggered versus level triggered is the framing I was thinking of.
Don't drop a child, bro. That's true.
Um... Are there any examples of such actors that own TCP connections or something like that?
Curious about some implementation details. So it's actually a pretty straightforward pattern,
right? So the idea here is that you just
TXRX is Tokyo Sync
MPSC channel. uh eight why not um and you tokyo spawn and i guess here i can do a tcp is tokyo net tcp uh tcp stream connect i don't know 127
001
on whatever port 8080 await unwrap and then what I can do here is async move and in here
I do while let some bytes is rx dot next dot await
TCP dot write all bytes, dot await,
dot unwrap. Again, I'm not. I'm just sort of demonstrating the pattern here. And now what I can do is here,
you know, I can send, I don't know, a vec of I want to write these bytes.
But I could also Tokyo spawn.
Async move loop.
this, I could have multiple tasks now that are each individually sending bytes for this thing to write.
If I didn't have this dedicated actor, like this thing that owns the I-O resource, I would have to take the TCP stream and stick it behind a mutex and an arc,
and then have each of these have clones of the arc, and then they would each take the mutex,
sort of alternating, right, in order to write out the bytes. There are trade-offs involved here. This is not always a better strategy.
This does mean, for example, that you need to allocate a vector for every sequence of bytes you want to write out.
Or similarly, if this was inverted, so this would read from the TCP stream and then send it out on a channel,
we would have to allocate a vector for every read we did in order to be able to give away ownership.
The advantage is that now these don't have to wait for the I-O to complete in order to continue.
And also, they don't need to have a mutex around the I-O resource that they both need to sort of compete on,
contend on, and reduce the performance of.
Yeah, you can still have back pressure if you want to make sure you don't just spam this, right? You await this.
And then you can also adjust how much concurrency you allow here with this.
Ah, and if you want responses, then the thing, this is a pretty common trick as well.
You do this.
Send and here
I do Tokyo Sync one shot channel.
And the thing I send is a one shot sender and the bytes.
And so this here will be the number of bytes written.
So that is the way that you ensure that you make sure that the response is also sent to the one who sent it.
As you introduce a one-shot channel with every interaction.
Oh yeah, ring buffers are nice for this too, but they're a little different than what you would use in this case. But I do like ring buffers as well.
And if you haven't looked at ThingBuff, which is a really cool crate, go look at it. Okay,

# tokio-stream
so last thing I want to talk about when it comes to IO resources then is the connection, or lack thereof, between async read and async write,
and the sync and stream traits. So... AsyncRead and AsyncWrite are about individual sequences of bytes that you want to write into or read out of a resource.
Sync and Stream, on the other hand, Stream being the more well-known of them, are traits specifically for
units coming out of a stream. Think iterator. Like each iterator yields you a thing,
a single thing every time you call next. Think of something like, again, an iterator or a channel where every time you do receive,
you get a new thing out. Whereas again, async read, you read and you read a bunch of bytes of varying size into a buffer.
So it's a different kind of interface. The bytes do not dictate framing, they just say here's a bunch of bytes that I wrote for you.
And sync is the inverse, sync is more like async write, except that where async write takes a bunch of bytes and tries to write them into a socket,
sync takes a single element and tries to put it somewhere. Think something like,
the most close analogy here is something like a channel sender, where you can send in one thing at a time.
Stream and sync are in that way the asynchronous versions of iterator and channel send.
Sync is not very widely used these days, it sort of fell out of favor, but stream is still pretty common.
If you have something that implements async read or async write, and you need something that implements sync and stream,
what you really need to provide is some way to do framing. How do you say, you know,
this string of bytes is now one element? It's one type, one value.
And similarly, I have a bunch of bytes. At what point do I turn them into a single value,
a single element for the purposes of a sync? And this is essentially what a codec is.
This is what framing means. It is the conversion between elements or values that are independent to streams of bytes that hopefully match that framing so that you can turn them back into their individual elements.
There are some tools in Tokyo util, like specifically there's a sub module called codec that helps you write adapters between these two traits,
but just keep in mind that they are separate. And this is also not something that's in Tokyo itself.
And part of the reason for that is because the stream trait and the sync trait are also examples of things that
We're not quite sure we've arrived at the right abstraction here. We're not sure that we know the right definition of these traits.
And this is why they're not in the standard library. There's been a proposal to get stream into the standard library for a while,
but at least as of the time of this taping, I guess, stream is not in the standard library,
and sync I don't think is even being considered at the moment. There is a crate called
TokyoStream, which provides you with the stream trait by re-exporting it from futures core,
and then implements a bunch of convenience methods around stream. But you'll notice that none of the Tokyo types implement stream,
even the ones you would expect. Like an MPSC channel receiver, for example, will not implement stream.
And that is because Tokyo... ...does not want to take a public dependency on this future's core trait,
or crate rather, because if it ever moves into the standard library with a different definition, Tokyo would have to do a breaking change to align...
Sorry, with that change in the stream trade. And so this is why all of Tokyo's implementations of stream live in the separate crate called Tokyo stream so that it can be upgraded separately from Tokyo.
And so that if stream does move into the standard library, the Tokyo team could cut a new major release just of this crate,
but not of all of Tokyo, which would be pretty sort of disturbing to the whole ecosystem and just update this one re-export and then all the other types would stay the same.
What you'll notice about Tokyo Stream is that it has these wrappers that are essentially types that you take a type you got out of Tokyo,
you stick it into one of these types, and now it implements Stream.
So this is the way that you bridge between Tokyo types and Stream. There is not an equivalent for Sync because people seem to generally have agreed that Sync is probably not that great of an idea.
And you'll see the same if you use that Tokyo Yuto codec in order to adapt between async-read and async-write and stream and sync,
that it also operates with stream from the futures core crate and sync from the futures sync crate.
So it too, if there was a standard library implementation of stream, would start using the standard library version of stream instead.
Okay. I think that's all I wanted to talk about when it comes to IO resources. Let's pause there for questions.
Sync and stream are often used for web sockets. Yeah, I've seen it being used for...
events or streams as well. So Tokyo Stream certainly, the stream trait is pretty useful.
And that's also why I think it will end up in the standard library. The question is just what should its exact type signature be?
And that's still under some debate.
Any hints to why sync is a bad idea? I think the observation is that it's not clear that it's valuable to have a trait here.
Like... It's rare, I think, the observation is that you actually want to be generic over sync.
I think you quite often want to be generic over stream, either in arguments or in return values.
But being generic over sync is just kind of a little weird. And I don't think we've come across very...
common needs for that to be the case. And if you don't have particularly salient concrete needs,
it's hard to make sure that the interface is the right one. So this is not to say that sync shouldn't be standardized.
It's just we don't have enough evidence for what it should look like to be comfortable stabilizing it.
Tokyo streams are the same as std async iterators, right?
I mean, you can think of the stream trait as the asynchronous version of iterators,
if you will. Yeah, there's actually a really good blog post by Boats that talks about,
where is it? The registers of Rust that I thought was really cool,
where they talk about sort of what is the mapping between different concepts in the async world versus the non-async world versus the iteration world versus the fallibility world.
And like, how does iterator map to stream, for example? And they also have a follow-up thing on,
is it patterns and abstractions where they talk about stream?
No, it's a later one. I think it's the pull next one. In general, all of these things on asynchrony,
I recommend reading if you really want to dig further into this. Just without dot boats.
I'm specifically talking about the experimental std async iter.
Async iterator. Yeah, so async iterator is the thing that stream is going to become.
I don't know whether they've even landed on the name that it really should be async iterator as opposed to stream. There's still some debate about that.
It could be that it ends up stabilizing under the name async iterator, but this is the same thing as stream.
Just trying to figure out what it should look like when stabilized.
Okay, so now we've talked about resources. Let's then move on to utilities.

# tokio::sync
We've started talking a little bit about this when talking about Tokyo Util Codec, for example, and Tokyo Stream,
but there are a couple of things inside of Tokyo itself that you should be aware of.
In particular, let's start with the sync module. So the sync module provides synchronization primitives that you can use for having different futures,
different tasks, but also just specifically futures, interact with each other, coordinate with each other.
There are a bunch of different patterns you can use here. I'm not going to try to talk about all of the ways in which futures can cooperate.
The docs here do a decent job of talking about the different ones, but I do want to highlight some of the ones that are easy to miss.
The most basic one, of course, is the MPSC channel, the Multi-Producer Single Consumer Channel. This maps directly to what you have in the standard library with std sync mpsc.
That one, I'm not even going to spend a lot of time on. It just works. It's great. There are variations outside of Tokyo that are sometimes faster or work better for particular use cases.
Again, ThingBuff is a thing worth looking at here. But ignoring MPSC,
there are a bunch of other ways in which you can coordinate between futures. And usually,
if you can use one of them, chances are it will work better for you than MPSC.
Think of MPSC as sort of a hammer, right? Like a lot of things will look like nails, you can solve a lot of things with them.
But often there are more specialized tools that will perform better, have better APIs when you can use them.
The first of these is the one-shot channel. I showed you one use of it already. So a one-shot channel is a channel that you can only send on once.
And when you receive, you only receive once. That means it does not implement the equivalent of stream,
it just implements future. So when you have a one-shot receiver, all you can do is await it.
There's no next because it will only ever yield once. And one of the things that is particularly worthwhile to know about the one-shot channel
over here I'll show you, is so there's a receiver and a sender.
And receiver has a blocking receive. So when you have a one-shot receiver, you can call blocking receive.
And so this is one way to bridge between asynchronous and synchronous context.
And the same thing is true for sender. So if you look at sender, the send method is not async on one shot.
It is a synchronous method. And so again, this is a way to communicate from the synchronous world to the asynchronous world,
or from the asynchronous world to the synchronous world. And this is a particularly useful thing once you do things like spin up a blocking thread somewhere to do some computation and you either want to send it some stuff to do,
like not regularly, you just want to send it one packet of work, or crucially, you want it to, after it finishes,
send some stuff back. If you can, you can just use the join handle for this. So remember,
spawn blocking, for example, returns you a join handle. You can then wait in order to get the result of the closure executing in.
But if you want more fine-tuned control over it, you could use a one-shot sender instead, which you can use from a blocking context.
So, so one shot, very useful tool for, for bridging, also very useful tool for anything that sort of acknowledgement.
So you very frequently see something like an MPSC, uh, that where the thing you send is both some operation and the sender of a one shot for you to send the result of that operation when it's completed.
This ends up fitting really well into the actor model of the world.
The next thing in sync that I want to talk about is, and I think these are roughly listed in order,
skipping up MPSC. So there's a broadcast channel. The broadcast channel is just, you produce a value and you know that when you produce that value,
there's going to be a bunch of copies of the receiver and every consumer receives every value.
So, this is not like a single producer multiple consumer queue, where you send in and whoever reads it first has it.
It is the thing that ensures that everyone sees every broadcast of a value.
Not very widely used, but useful to know about. But the really cool one is watch.
So, watch is kind of like a broadcast channel, except that it doesn't keep every value you send.
Instead, it only keeps the most recent value. And the reason this is cool is because imagine that you have a bunch of receivers,
but they're not necessarily always checking on the value. Like they're not regularly receiving from the channel,
but every now and again they are. What watch allows you to do is not build up like an infinite queue because you had some slow readers.
And instead, when you send, you don't really send, you just update the broadcast value.
You update the value that readers will see when they read. And then on the reader side,
you can basically subscribe to changes to the latest value. And so if you come back,
you know, way later and it's changed 20 times, you only get woken up once and told about the new value.
This can be extremely useful for communicating things like config changes.
The fact that something has changed, so now go look somewhere else, it's a really useful mechanism where the individual notifications are not important.
What is important is just the latest state. So it allows you to, you can think of it as sort of as batching updates and amortizing that cost of doing a bunch of receives in a row.
And then the last one, no actually I'll talk about two more, because I do want to talk about Semaphore.
But the next one I want to talk about is Notify. So Notify is sort of the second to lowest level primitive that you have in Sync.
Notify is just a type. that you, there's an example here,
you create a notify, you put it in an arc or something, and it's sort of like the condition variable,
if you will, from the synchronous world, except not associated with the mutex. It's not associated with the value at all.
You can call.notified and you get a future back that you can await. And you continue executing,
like this await resolves when someone calls notify1.
That's all it takes. So this is a way to just tell someone else, hey, wake up.
This can be a really useful or simple way to implement wakers in your own resources.
Rather than figure out how to do poll calls and distributing this waker and stuff, you can often get away with just using a notify.
And so in your resource, what you actually do is you just construct a notify and you do.
give a clone of the notify await to something or stick it in a queue somewhere that is going to call notify one when there's more work to do.
And then in the resource, wherever you are in your asynchronous function, you then just call notify.await.
And that way, the only thing that you need to be concerned about is making sure you call notify one from somewhere and you don't ever need to touch the sort of low level pull interface.
So that's where notify can be really useful. It's useful in other places too, but that's the main place I've found it to be useful.
Okay, before I go to Semaphores, let's do questions there.
Does it make sense to use Broadcast for a WebSocket server where the different clients are stored in memory?
So the main problem with Broadcast is... the slow readers problem,
right? So if you have a bunch of WebSocket connections that are all listening on the broadcast channel,
then imagine what happens if you have a particularly slow client, like someone's connected over like 2G network to your server over WebSocket.
So your writes out of that channel are really slow. That means you're draining things out of the broadcast queue very slowly,
which means that those can't be dropped because you haven't read them yet, which means the broadcast queue just keeps increasing.
You can set up... like a bound or a behavior on broadcast for what should happen if you try to send and the sort of broadcast buffer is full,
you can choose for it to drop like the latest thing or just refuse to send. But ultimately you have a problem there,
right? Which is your application is either going to have to drop things that go out in that web socket,
that slow web socket, or it's gonna have to buffer them forever. And that's an application design question that you just have to figure out what the right behavior is.
The broadcast implementation allows you to choose whatever behavior you want, but you do have to choose.
Oh yeah, notify can also be really useful for canceling, although I'll talk about a potentially better way to do that later.
Where imagine that you have a sort of... Actually,
no, I don't want to recommend notify for canceling because I want to recommend a different thing for canceling. But yes, you can use notify for canceling.
So notify could be used to bridge or wrap sync resources into an async world. That's also true,
right? So if you look at the method signatures for notify, you'll see that... So notified returns the notified and this is a future.
So this thing is not that useful in...
The thing you get back from Notified is just implements future. So this one is not in and of itself useful in a synchronous world.
But notify1 is synchronous. So you can use Notified to have an asynchronous wrapper for a synchronous resource.
So something synchronous is going to call notify1, which it can because notify1 is not an async function.
And your future is just going to await this Notified.
All right, so the last thing I want to talk about in Tokyo Sync is semaphore.
Usually you shouldn't need to reach for semaphore. Semaphore is a very low-level primitive, but it is useful to know what it does.
Semaphore is a way to have a number.
Think of it as a ticketing system. So you set some maximum number of tickets that are allowed to be active at any point in time.
And initially, let's say there are four tickets. A semaphore of size 4 means that someone can take a ticket
someone else can take a ticket, someone else can take a ticket, someone else can take a ticket, and then the next thing,
the next task, the next future that tries to take a ticket is going to return pending.
It will not get to proceed until someone who got a ticket drops that ticket and at that point sort of returns it rather to the pool,
then someone else can come in. So it is a way to guarantee that there are at most n things where n is configurable active at one point in time.
There are a bunch of places where this kind of thing goes up. For example, a mutex is a single-permit semaphore.
That is in fact how it's implemented, right? It is a single permit semaphore. There are other semaphores,
like for example, let's say you don't want there to be more than so and so many concurrent requests or so and so many concurrent connections you're handling in your web server.
You could have a semaphore that is the number of connections you allow, and every time you're about to accept a connection,
you first check whether you can grab a semaphore. And if you can't, then you just do not accept any more connections until someone gives the ticket back,
meaning some other connection has been dropped. Or if you don't allow more than N concurrent requests on a single connection,
you have a semaphore with that limit and before you read a new request from the client over that connection,
you try to grab a ticket from the semaphore and only if you get a ticket do you allow it to send another request.
Otherwise it has to wait for one of its pending requests to finish. So this is a really useful low-level primitive,
and you'll find that it's actually used by a bunch of high-level primitives. So mutex being an example. Another example of a place where it's used is in the...
concurrent request filter in tower, I believe also uses Semaphore because it can,
right? This is the kind of pattern that Semaphore enables you to do, but its interface is very low level.
So you'll see here, when you create it, you say how many permits there are, how many tickets there are.
You can add permits if you want. There's an asyncfn acquire, which you try to take a ticket and you see whether you get it or not.
It's async, so you can await it and you'll be notified when you can. And similarly,
you can also, so with the Semaphore permit, you get back the,
where's my...
Yeah, so if no remaining permits are available, acquire waits for an outstanding permit to be dropped.
So the type that you get back, why is the text so long? The type that you get back from acquire the semaphore permit here holds that ticket until it gets dropped.
When it gets dropped, the ticket sort of gets returned to the pool and then you allow someone who's waiting on acquire to resume.
And this is also how you get something like a read or writer lock, right? So a read or writer lock is actually one that has N permit.
and taking out the read side of the lock just takes one semaphore.
And if you are a writer, you have to try to acquire all of the semaphores at once. That is how you know that you have exclusive access.
So yeah, semaphore, useful thing to know about.
Not a thing that you should generally need to use very often,
but it is useful to know that it exists.
Okay, moving on from Tokyo Sync.

# tokio::task::JoinSet
The next thing we have is Joinset. So Joinset...
So Joinset... is a collection of tasks that are running at the Tokyo runtime.
And the idea here is that let's say you...
What's a good example of this? Let's say you're writing a web crawler. So you have a bunch of URLs that you want to crawl from this website.
And so you really want to have those crawlers run kind of in parallel. And so you want to spawn each one.
But after you've initiated all of these tasks that are all going to be downloading a different URL,
you want to wait for all of them to finish. But how do you know when they've all finished?
Well, you could have a vector, and then like every spawn, you stick the join handle into the vector, and then after you've spawned all of them,
you loop over the vector and you wait each one. That does work, but it's not really what you want to do.
You can instead use a join set. So the idea of a join set is you construct one, and then you call spawn and you pass it a future that it will then call Tokyo spawn on,
and it will take the returning join handle and sort of stick it inside itself, inside of the join set.
And then, crucially, joinset has a joinNext method that is asynchronous.
You can call it on the loop in order to get the output results of each spawn future as they resolved.
And importantly, this happens in any order. So, imagine again that you have this setup where you have these,
let's say you're crawling 10,000 URLs. You would end up with a vector that's 10,000 join handles long.
If you then write your for loop, what will actually happen is you wait for the first URL that you spawned to finish,
then you went for the second URL you spawned to finish, then the third, then the fourth, etc. But it could be that URL number 18 finished way before
URL number one. And With this vector approach, you wouldn't know that, you wouldn't get to print anything until the first one is finished,
even if that might be the slowest one to crawl, it's like some giant website or something.
With join set, when you call join next, what that means is, give me the output of the futures that I have spawned in whatever order they become available in.
So this can be a super useful way to keep track of large numbers of futures, and then make sure that you wait for them or collect the results after the fact.
That is all it really does, and it's useful to know about.
Do notice though that it specifically is for spawning. So this will use Tokyo spawn behind the scenes.
It will run them as parallel tasks. It is not for sort of local concurrency.
So what I mean by that is it's not for if you have a, you already have a top level task and you don't want to spawn them.
You just want to run them concurrently inside of the current task. Then a join set does not let you do that.
Unless I have missed something. No.
For that, I don't know. I don't think Tokyo has an equivalent to this.
Let me see just in case they've added it and I'm lying.
No, it does not. So. I'm not actually going to recommend that you use this because it has a bunch of problems.
But there is this type called futures unordered that does not spawn and instead just locally looks at them in the current task
So this is a thing you can use instead, but it does have a bunch of problems I'm expecting the one day Tokyo will sort of get its own version of this that isn't broken quite a severe way but actually getting this version right like the version that doesn't spawn right is
Quite complicated and if you look at the the github issues for this type you'll see that there are a bunch of discussions about
How should this actually work? And in fact, that's not what I meant to do at all.
Let me fix that real quick. Aha.
If we look at without boats again, you'll see here, futures are ordered in the order of futures.
That talks about this exact type and why it's actually pretty complicated to get right. So I'd recommend you read that if you really want to dig into it.
But know the join set is very often the thing that you want, specifically because usually you do want to spawn these so that they can execute in parallel and not just concurrently.
What I mean by in parallel is that the different tasks that you spawn get to work on different worker threads at the same time.
Whereas if you don't spawn them, if you just sort of join them, then they're only executing on a single worker thread at a time.
Okay, questions on join set before I move on?
Yeah, so there's also here in, in fact, I'll show it in task.
You'll see that under task, there is my line.
I am lying. In the root of Tokyo, there's a macro called join and there's a macro called try join.
And these, you actually give a list of futures and it will wait for all of them to resolve.
And then it will give you back a tuple of the results in the same order you pass them in. So this is a way for you to run multiple futures concurrently,
but not in parallel. So within the context of a single task without using any Tokyo spawn.
But the downside of using join here is that it requires that you have a static number.
You know at compile time how many different futures you're joining from,
but they are allowed to have different return values, different output values. Try join is the same,
just they're allowed to be fallible.
If you don't spawn them, doesn't Tokyo still work still if needed? No. So if you use join here,
or futures unordered, that means you're not spawning them. If you're not spawning them,
that means they're just nested futures inside of whatever future you're currently in. So they're nested futures,
they're not top level tasks. And because Tokyo only gets to schedule top level tasks,
it cannot work steal one of these sort of embedded futures down here. And so if you use join or futures unordered without spawning,
Those futures will execute concurrently within this task, but they will not get to execute in parallel,
they will not get to be work stolen.
Does spawning a Tokyo task allocate a new stack like go or beam instead of using memory as needed?
Yes, you can think of that as roughly right.
Rust futures are kind of stackless, like they don't truly have a stack. They're more like state machines.
But when you spawn a task, it does get its own heap allocation for the task and for the state machine.
What's a good way to pause all tasks on join all temporarily until something is done for it to be resumed?
You'd want to use join plus select. I'll talk about select in a moment.
Okay, moving on to Select, in fact.


# tokio::select! and cancellation
All right, so Select is one of the coolest features of Async and also the easiest one to burn yourself on.
The idea behind Select is that you have a bunch of different features.
and you want to wait for one of them to complete. This could be sort of the first one to complete.
Another name for select that you might think about in your head is race. So the idea is that you have multiple futures and they're racing,
and you just want whichever one finished first, you want to do something when that one finishes, and then forget about the other ones.
Common examples of these are things like wait for a TCP packet or the user to press control C.
Or wait for an input on this TCP channel or on standard in.
Or wait for a new message on this channel or for this write to complete.
Or, you know, wait for input on this channel or for this notify to complete that tells me that I should shut down early.
Select embodies all of these. When you write a select, the idea is that you write a bunch of arms inside of the select.
I wish they had an actual example here. Ah, they do. So when you write a select,
you write a bunch of arms for the select. Each arm has something that needs to be a future.
So in this case, doStuffAsync is some method that runs a future. And moreAsyncWork is also a function that produces a future.
And... When you use select in this way, what's going to happen under the hood is you construct each of those features.
They're embedded features, so they're not top-level tasks. Embedded features, they get to run concurrently.
And whichever one finishes first, we're going to execute the block under that one.
And then we're going to drop all the other ones. All the other ones, we're just going to drop them, not do anything more with them.
So let's say this one finishes first, then we're going to go into this arm, we're going to execute this code, and then we're going to continue from after the select.
So for example, here you can see we are selecting over two different streams,
and if you go into this one, then the V will be the V we bound back from this next call,
and you get to access that in there, and similarly here.
And you can do the same over things like a sleep, right? So this is one way to implement a timeout,
is that you call stream.next in a select, and the other arm is you're waiting on some sleep future,
right? And so if the sleep finishes first, then you go into this arm, and if the stream.next finishes first,
you go into this arm. And so what that will be is a timeout.
So, select is useful for all sorts of things, anything where you want to wait for multiple different kinds of things and do different kinds of things in return or in response to them finishing.
Before I talk about the complications around select and about cancellation, does the basic premise of select make sense?
Large select blocks are a pain because the LSP basically doesn't work inside them. One of the pains of select is that it's a giant macro.
And that means that... Like, IDE integration often doesn't understand the syntax because it's,
as far as it can see, it's just arbitrary macro input code. So it doesn't actually know what it turns into.
So large select macros tend to be pretty annoying. Usually you'll want to keep the select pretty slim and then move the actual logic into separate functions.
Why is there no await on the async functions inside of the select? How does Rust run them without the await? So the answer to that,
why is there not a.await here? And the answer to that is because Tokyo's select macro expects that everything that's passed here is a future.
And so the expanded version of the macro will have a.await in there.
But you don't write the await yourself. And the reason for that is because if you wrote the await yourself,
you could in theory pass in something that wasn't a future. And then all of this would get really weird.
The slightly more complicated answer is that it doesn't actually call.08 for you. It doesn't add a.08.
It has to deal with the underlying pull calls for various reasons.
But the net way to think about it is the things you passed in must be futures. Therefore,
you Because Tokyo knows they must be futures, you don't have to write the.08 yourself.
What's the behavior of SELECT when multiple futures are ready with the result? Yeah, so this can happen.
Usually this happens because multiple of the futures were actually ready before the SELECT even started.
But it could also be that during the execution, multiple of them happened to technically be ready,
but because this is all single-threaded, like there's no actual parallelism, it's just concurrency,
what SELECT is really doing is it's calling pull. on all of the futures over and over.
It's not actually that dumb, it has optimization so it doesn't do it unnecessarily, but you can think of it as, it calls poll on the first future.
If that's not ready, it calls poll on the second future. If that's not ready, it calls poll on the third future. If the third future is ready,
then it goes into that arm, executes that code, and then exits from the select. Otherwise,
it goes to check whether the fourth, polling the fourth future is ready, etc. And then if it gets to the bottom,
it goes back to the first one again. That's sort of how you can think of the implementation of select, even though in reality it is smarter than that.
But in terms of fairness, it's a good question. Like, what if multiple futures are in fact ready? Which one does it choose?
And the answer to that is, you'll see here, there's a section on fairness. And by default, select randomly chooses which branch to execute first.
So you get a random branch that is ready. So that if you call select in a root,
for example, you don't always end up with the first branch being the one that's taken. There are ways to override this.
Like you can make a branch be biased to say, always choose this arm if it's ready.
Otherwise, just do fair among the remaining ones. But this is like...
There are some subtleties involved here, and if you actually care about exactly which arm gets executed when multiple are ready,
you should read this paragraph, or this sort of little section, to figure out what's right for you.
Is Tokyo Select any different from Future Select? I don't remember the exact details of Future Select.
I think they're fairly similar, but there are a couple of ways in which they're different,
such as I think Future Select requires a weight, whereas the Tokyo one does not.
But they're mostly similar. You can think of them as mostly similar.
Bias can only be added once to the top of the macro, not per arm. Ah, sorry,
yes. Biased is, prefer them in the order they're listed. So you set it once for the whole select,
you don't get to choose which arms are biased. Totally right. Okay.
So that's the reasons why select is nice. Then we get to the reason why select is complicated.
And that has to do with something called cancellation safety. So,
Let's imagine that we have the following.
Let's do a loop. We have a select.
And inside of the select we have one arm that is
Read a certain value. Does something. We have another that is
Tokyo time sleep.
Okay, so let's say I have this. Let's assume that
I... Yeah, that's fine. I'll just write this.
So we have read a certijsonvalue,
fn, and it returns a... Everyone knows that a certijsonvalue is actually just a string.
And this is a to-do. Okay, so let's say that we have this use of the select macro in a loop.
The idea here, the intent of the loop, right, is that I want to be reading stuff into a certi.json value,
but every second I want to write zzz. I just want to do that and then I want to keep going around until I get a certi.json value out,
and it's only when I actually get the value out that I'm ready to print something.
In fact, let's make this a little bit more interesting and make it be a...
No, string is fine. So now the question becomes,
what goes in here? Let me write something that's not cancellation safe to show you what can go wrong.
I have a buff here that is 1024,
and I do something like, let's say I have a TCP here.
Yeah. TCP stream connect await unwrap.
So here this takes a new TCP stream.
And down here I do tcp.read. mute buff,
oh, wait.
unwrap. And then what I actually want to do here is loop.
Actually,
let me just do s is string near
like this and then I do s dot push stir or push buff n and then if s
dot len is greater than 100 then I return s
0 u h and bring me a Tokyo TCP stream
And bring me read async redext.
And that's fine.
We'll just assert that this is UTF-8. Why not?
All right. So now I've written a readAssertedJsonValue that just returns a string.
I guess assertedJsonValue here is a lie. This is really readAstring. Read a string of 100 charis.
That's really what I've written, so let's name it like that. So this all looks fine,
right? Sort of naively, right? This asynchronous function creates a string,
it has a buffer, it reads into the buffer, it keeps pushing onto the string,
I don't actually need the n here, no, I do need the n here. It keeps reading bytes out of the TCP stream,
parsing them as a string, pushing them onto the string it's accumulating, and once the string is greater than 100 characters,
then it returns us. Looks great, right? The problem here is what happens if this connection is slow.
So imagine that we go through this loop, so we enter the loop first, we go into the select, and then the select tries to execute this function,
this function runs, it allocates a buffer, it does a tcp read, and let's say it gets,
I don't know, it gets 10 bytes, 10 characters. So it takes 10 characters, it pushes it onto the string,
the length is not greater than 100, so it doesn't return, it loops, it goes back here, and now it tries to do a tcp read again,
and remembering now, this now has 10 characters in it. right? But this await realizes there's no more data to read at the moment.
So it goes to sleep, it yields. Okay. So that means that we end up back here up at the select,
the select tried to execute this, but this future isn't ready yet. So it then goes to await the next future.
Okay. So that's a sleep. Great. So it now tries to await the sleep. So now we have two futures that are,
and you know, it hasn't, a second hasn't passed yet. So now it's two futures. Both of those futures are not runnable.
And so now the select kind of just waits for one of them to become runnable. So far, so good.
Now let's imagine that... one second passes.
So this future now becomes runnable, like the IO event queue or like the thing that drives timers rather,
now has a full second expired. It realizes that this particular sleep future should now be awoken.
It calls wake on the waker for this future. That moves that future from non-runnable to runnable.
The select picks up on that. And so it goes around again. It might even try to pull this future again,
but it still says I don't have any more bytes. So it goes to this one. This one now resolves. So we enter this block.
Now that we've entered this block, we do this print. And then we finish the select. And when we finish the select,
what happens? All the other futures that didn't finish get dropped. That means that this future gets dropped.
And then we go back around the loop and then we do it again and we construct a new future for this and a new future for that.
The problem is these 10 characters that we've already read. Because they were in this future the first time around the loop.
And then when we dropped this future at the point of finishing this block,
then those 10 characters are dropped because they were contained within the future.
And so once that happened, then we drop those characters, they're gone forever.
So when we go back around the loop, and then we create a new instance of this future,
that means we call this function afresh. It starts a new string with no characters in it, and then it starts reading into this.
So we've lost, we've dropped on the floor, 10 characters that we're never getting back.
And so we might actually, if this connection is slow enough, we might never get into this branch, because we keep reading a couple of characters,
but then we keep hitting the timeout, which then causes us to exit the select, which causes us to drop the future,
which causes us to drop all the characters we read, and then we loop back around. This is an example of the problem of cancellation.
The problem of cancellation when you're using selects is that you have a future that got canceled, it got aborted, it got dropped,
and it had some state that was actually important that went away. These 10 bytes are just as if they were never read.
And this can lead you to really problematic situations like, imagine that this was not reading just a string of characters,
it was actually reading a JSON object. Maybe you now dropped the opening curly bracket.
And so now as you keep going around, you actually never end up with a valid JSON object because you never got the opening bracket,
or rather you got it, but then you dropped it. This leads to enormous problems down the line where you just silently drop state of futures.
And this is the really big thing to be aware of whenever you use selects, is that either your futures need to be what's known as cancellation safe,
meaning that they can be resumed without having lost any state. Or,
you need to write your select so that it doesn't matter, like these futures are never attempted to be reused.
But cancellation safety is usually the thing that you want. There are two ways to achieve cancellation safety.
The first of them is that you'll see on a bunch of Tokyo types, like if you look at
MPSC receiver, for instance, you'll see that the receive method says under cancel safety,
this method is cancel save. If receive is used as an event in Tokyo's select statement and some other branch completes first,
it is guaranteed that no messages were received on this channel. The implication being that there are no messages that might be received and then get lost forever,
as a result of that future of receive being dropped. So receive for an MPC channel is safe to use in this kind of context.
This asynchronous future is not cancellation safe. If you drop its future and then try to resume,
you will have lost state, or you may have lost state. You know, if it never did any read, of course you haven't lost anything.
So, in general, you want to read through anything that you use in a select and make sure that they're all cancellation safe.
And the way to make something cancellation safe if it isn't is to construct the future outside of the loop,
like this, and then in the select,
use a mutable reference to the future instead. Because that way you're never dropping the future when you get to the select because the future was never owned here in the first place.
And so that way, when you go around, you're actually resuming the same future. This doesn't quite work because of pinning.
So the actual way that you need to do this, and I think there's an example of this down here. Oh, yeah, these are the things that are cancellation safe.
These are not cancellation safe. You need to be careful. But there's an example here of how you can avoid this problem.
Uh. where you pin it outside of the loop,
and then you take a mutable reference. This pinning is kind of annoying, right? You just,
but it won't compile without it. So at least there's that. Pin.
And I think actually we can use std future pin here.
std task pin. std pin pin.
Okay, great. Std pin pin.
And then you can pass a mutable reference to it here instead. But you do need to pin it in order to use this trick.
But once you do, then now you don't have to worry about the cancellation safety because you're not ever canceling that future.
So this is what we mean by cancellation safety. I have more to say on this,
but I'll pause here for questions first.
It's just only because selected is in a loop.
If select is not in a loop, this tends to not really be a problem because it usually means that you don't care about the other features being dropped.
Because they were constructed in here and you're never really resuming them because you're not running that line of code again.
So it only really tends to come up as a problem in a loop. You could probably write some code where this problem arises even though you're not using a loop,
but that is much less common.
What does pinning do under the hood? Does it change the type of fut? Yeah, so the future type here actually becomes a pin mutable reference to the future that comes in here.
So it turns this into it's no longer just this future. It is actually a pin,
a pinned mutable reference to that future.
Does this mean that ideally we need to make sure that all the processes in the HTTP handler is cancel safe?
I'm not sure what you mean by HTTP handler. This only applies to things that are awaited on in a select,
in the arms, the arm definitions of the selects.
Yeah, pain was a good Freudian slip here, I agree.
Yeah, so the behavior is different now, which is totally correctly pointed out here that here,
we will now only ever read one of these, but that's kind of what we intended, right? Because we had a break here.
So we intended for the loop not to continue anyway, the moment this resolves.
Select is more like a future than a loop, right? So select is.
A future that internally loops over all of its arms? So kind of both?
You should think of select as being a future. It isn't, but that is sort of what it is.
It is also internally kind of like a loop.
In this scenario the future is never cancelled and is just ignored. Um...
Well, in the pin case, the future is never cancelled. In the old case,
when we say cancellation safety and when we say a future is cancelled, what we mean is that it is dropped before it got to finish.
So it got cancelled, it got aborted, right? It started doing work, it accumulated some state,
and then we dropped it rather than letting it complete. In other words, it was cancelled. So in this case we did cancel it.
In the pin case it does not get cancelled because dropping it here does not mean that it doesn't get to do any more work the next time around the loop.
If we pin the future, once it resolves, won't that make it a one-shot future? I'm not sure I understand.
So one-shots are different from this. One-shot is a mechanism for having two different futures communicate with each other,
so that one can send a value to the other. It has nothing to do with us future-only resolving ones.
All futures only ever resolve ones.
This versus calling timeout on Feud in a loop. Not sure I follow.
Could we have made the future cancellation safe by passing in the string that we write to? Yeah, so that is another way that you could solve this problem.
So instead of pinning here, you could do this.
And then do mute S.
And then have this like. I guess not return, and instead it'll print s here.
This will take an s, which is a mutable reference to a string.
This no longer gives s here, like so. Yeah, so this is an example of something that would also make it be cancellation safe,
but that assumes that that's only true if read is cancellation safe.
And we can check whether read is cancellation safe, so if we go to read, async read x,
cancellation safety. This method is cancel safe. If you use it as an event in Tokyo select and some other branch completes first,
then it's guaranteed that no data was read. So only because of this clause is this safe.
But given that clause, this is cancellation safe. And usually I tend to tell people always document the cancellation safety of anything that you call from a select.
If I have some work after the loop that does not depend on the foot, we need to manually drop it to avoid unnecessary background processes.
Okay, so this is in the pin case.
So if we have this, then yes, down here, you might want to drop Vue.
But it doesn't actually matter because remember, this is not an actively running thing.
This is just the state of the future. And so you're not really wasting system resources.
You're wasting a little bit of stack memory. You're not really like wasting compute resources because this future,
if it's not being awaited or run in a select, it doesn't do anything. So you're not wasting compute.
Is whatever Tokyo Sync MPSC uses for cancellation safety the same or related to this mute technique? No.
So the way that you can implement cancellation safety internally in some kind of resource is just by having it do atomic things.
I don't necessarily mean CPU atomics, although that can be the way to do it. So that...
You basically don't have any await points between when you consume a value or like consume a resource like reading bytes from a file socket and when you return them.
So you know that if you consume the resource, then you definitely also returned it. That is the way that you guarantee cancellation safety.
If you consume some resources like you read some bytes off of the wire and then you have some await in there before you return that value,
then you're not cancellation safe. Because you could be dropped. At any await point you could be dropped.
It's funny how cancellation on safe code is still safe rust. So safe rust is a very particular meaning, right?
Safe rust means that it cannot have memory on safety.
Things like one type being treated as another or have undefined behavior.
There's no undefined behavior here. You just dropped a bunch of data, right? So it's wrong,
but it's not memory on safety. And that's what we mean by safe and rust. So this is more of a definitional question of what does safe mean.
And when we say that something is unsafe, do we mean the formal sense of rust on safety?
Or do we mean unsafe as in general, like this could lead to buggy behavior? And those are two different meanings of the word.
What I mean by one shot is that it can't be reused in the select. This is only useful in the case where you only want to use this kind of future once.
Well, so sort of, right? So you could here, if you end up in this branch,
you could then say... You could reassign to Futir.
You couldn't actually do it this way because of the pin. But you could imagine that you reconstruct the future in here.
In fact, I think you would use... Is it... Dot replace?
I can never quite remember what...
Pin... Set. Set.
So you could do this in order to... Once you've read a thing,
if you actually want to keep looping around, then yeah, you would replace it with a new future, but inside of the pin so that it itself remains cancellation safe.
So if you want something to happen in one select arm on interval, then also the interval should be defined outside the loop, right?
Yes, so that's also true. So this thing will not happen every second. It will happen a second after the start of every select.
So if you actually wanted something to happen on an interval, well, you would probably use the interval. So Tokyo has a...
I'm sorry. inside of the time module.
I haven't really talked about time because it's fairly straightforward. The only thing to know about here is that there's a minimum limit on how long intervals can be in Tokyo.
Like it doesn't really support, I think intervals shorter than... 100 milliseconds or something.
But you can have an interval that you can construct. And then what you would call inside of here is.tick on that interval.
But you would construct the interval outside the loop precisely for this reason. And in fact, if you look at tick, cancel safety,
this method is cancellation saved. So make sure to always check that.
I wonder if a different select API could exist that makes it less likely to make a mistake here. You are not the first person to wonder that,
and we haven't really come up with one yet. If you have a good idea, then absolutely, but it's actually surprisingly hard to figure out what should happen here.
One of the questions is, should futures be cancelled on drop? What should the drop behavior be for a future?
Currently it means cancel, but it's not clear whether that should be the case. This also ties into the question of asynchronous drop.
So currently, if you have a future and it implements drop, then your implementation of drop is not async.
Drop is a synchronous method. That means you can't do asynchronous things in your drop, but trying to fix that is all sorts of complicated for other reasons.
So this one's tricky.
All right, so that is cancellation safety,
which mostly comes up in the context of select.
Last thing then to talk about in terms of utilities is uh

# tokio-util and CancellationToken
is Tokyo util so I've talked a little bit about this already so codec is the way that you connect between async read and async write and stream and sync so that one we've already talked about I'm not going to dive into how it works
I think if you need that you can go look at it this is after after all Tokyo decrusted not the Tokyo ecosystem decrusted
Compat is a way to have interoperability between the async-read and async-write traits from Tokyo and the async-read and async-write traits from the futures IO crate,
which is used by some other asynchronous runtime in some other crates.
There are a couple of useful helpers here inside IO and net for things like...
Things like bridging between synchronous and asynchronous I.O. Things like copying things in and out of bytes.
Things like having...
I don't even know what this thing does. But under sync, I think there's some interesting stuff.
I'll talk about cancellation token in a second. Delay queues. Basically,
things that are... bigger and also a little bit more unstable constructions on top of the Tokyo primitives.
I mean, Tokyo primitives are usually sort of lower level things, building blocks that you can use,
whereas Tokyo Utils tries to provide higher level abstractions on top of that.
But there are a couple of things that I want to call out from here.
And they're both unknown. One is under task and the other is under.
Where is the other one? Where is task tracker?
Ha, it's even deeper. Okay. So the first one of these is cancellation token.
Cancellation token is unrelated to cancellation in terms of select.
What cancellation token is for is specifically imagine you have this like giant system, you have a bunch of Tokyo spawns,
like you have a bunch of different background tasks running. And at some point, like the user presses control C.
and you want to have graceful shutdown. You want your entire program to like, you want all of those different tasks that you have to start exiting.
One of the ways you could do this is you could, as we talked about on the runtime call shutdown.
And when you do that, the next time every task yields, it gets dropped. And then eventually all the tasks are dropped.
And so eventually your shutdown returns because all the tasks are gone and now you can shut down.
But in reality, you didn't really let those futures finish up their work. You just kind of dropped them when they were in some kind of pending state.
One way that you can improve on this situation is to use a cancellation token. What this type does is that you can,
it's kind of like a notify and in fact I think it's implemented using notify internally, but you construct one,
you can clone them and give them out to all of your different tasks that you spawn. And the idea with it is every cancellation token has a cancelled method that is an asynchronous function that will only return the moment someone calls cancel on the cancellation token.
But all of the clones of a cancellation token are connected. So if you call cancel on any clone of the cancellation token,
then the canceled future on all the cancellation tokens will suddenly yield.
So the idea here is that for all of the tasks that you spawn in various places, you have them do a select over the main work that they do and canceled.
And so that way, whenever they get this cancelled signal, then they can choose what to do in order to clean up and then they can choose to exit.
So this is a way for you to have one place in your program where you say, okay, time to shut down.
And then at that point, all of the threads start hitting this clause of their select.
At that point, they can choose what to do. Maybe they exit their loop. Maybe they do some cleanup, whatever. But eventually,
when they get that signal, they should know that it's time to wrap up, stop reading from channels,
stop reading from TCP sockets, whatever it might be, and just sort of finish up and then eventually return.
And... Hopefully, at some point after that, your runtime should become idle. There should no longer be any runnable tasks because they should all have sort of reacted to either this cancellation signal or cascading signals from that.
Like they were reading from a channel and the sender end of the channel they were reading from has now been closed because that thing saw the cancellation signal.
So eventually it should propagate all the way down until all of your tasks have gracefully exited. And the cancellation token is just a handy means to have the trigger to start a shutdown and the means in which to react to that shutdown signal.
There is also a stream cancel. There's an equivalent type specifically for streams that has a sort of this tripwire thing where you have,
you can take a stream and you can wrap it with a thing that will close the stream, like have the stream start to yield none the moment someone calls cancel on this trigger.
And so it's the same kind of construction. It just forcibly makes a stream start returning none with the same ultimate purpose.
So this can be a really nice way to give your application the ability to do graceful shutdown,
but it does require that you thread this tripwire or this cancellation token all the way through your program so that every task has a way to know that,
okay, it's time to shut down now.
Okay.
let's see can you keep reading from a channel after the center has been dropped so it depends um if you look at
the Tokyo MPSC channel, right? It is a multi-producer single consumer.
And so the rule is that when all the sender handles have been dropped, then it's no longer possible to send values into the channel.
This is considered the termination of the stream. And so then poll will start to return none. But if one sender is dropped,
then that doesn't mean that the channel is closed and you will still block waiting for more values.
There is a way to specifically say Or is it...
No, there's not. I'm like... But yeah, so the answer to this is like,
a channel, if all the senders have been dropped, then the receiver will still receive whatever's maybe in the channel,
but after there is no more, it'll get a none, and then at that point, no to stop reading. Similar to an iterator,
really.
What if you're writing a library, like an actor library, then it's hard to share the cancellation tokens as the user might create their own one.
It's true. And this is one of the ways in which it would actually be handy if cancellation tokens was in the standard library so that everyone could agree on how to signal it's time to finish now.
Until that happens, and I don't know whether it will, it would probably go inside the context maybe,
until we get something like that, If you're a library author, you could have sort of two APIs,
right? One that takes a cancellation token and one that does not. And if you don't get one, you just do whatever you normally do,
which is you block forever or your awaits might take forever during shutdown.
You might just get dropped. But if the user provides you with a cancellation token, then you're willing to respect it.
That, of course, would mean that you would take a public dependency on the Tokyo util crate, which you might not be willing to do.
But you're sort of caught between a rock and a hard place here. There's no great solution for having libraries that support being gracefully shut down,
sadly. Unless you just write your own type that wraps this that you tell people to write in,
and then they need to figure out how to bridge between them. It's unfortunate.
Because token is not mutable, can I use it as a static to avoid needing to pass it to every function that's going to depend on it?
You know, that is an interesting question.
I think you can, actually, because I think it is, yeah, it's both send and sync.
So yeah, I mean, I think in principle you could. You could have a, you, it couldn't actually be a normal static because the,
unless the constructor is const. Yeah, the constructor is not const. And so you would need to use something like a,
a once lock in a static. But if you did, then yeah,
you could use a static for this. It might get really annoying for things like tests, right?
Because all of your tests will use the same static. So if you go cancel in any test, all of the tests will exit.
And this is the perpetual problem with statics, right? Is that they truly are global.
But if you don't have tests, then you could probably do this. I don't know that I would recommend it.
Will async drop replace cancellation tokens? No, even if we had async drop,
that would be convenient, but it wouldn't replace cancellation tokens because async drop is just to allow you to do asynchronous operations when you are dropped.
But cancellation tokens are the way that you learn that you should exit and should start to drop yourself.
Yeah, often the way you would do is you would do a cancellation in libraries,
you would have like, give me something that implements future, and I will treat that as my cancellation.
So you just take anything that implements future with the output type as unit, and someone could pass in a cancellation token or whatever their own means are.
It doesn't really matter, right? So they could just take call canceled and just pass this type in,
because as long as you're generic over what you take, you're okay. If you pants to cancel token,
every function does not behave just like go context. Yeah, in some ways it does.
Um, all right. Uh,
there's also the type called task tracker that can be useful to know about in Tokyo util. Um, it's sort of a combination with cancellation token.
Uh, you can use it to get graceful shutdown. I'm not actually going to talk about this in detail, but there's a decent,
uh, blog post on this on the Tokyo blog on Graceful Shutdown that I recommend you read.
It talks a lot about exactly this and how to wire it all together correctly and how you can use Task Tracker to make your life a little bit simpler.
All right. We're on the home stretch. I think those are all the utilities I wanted to talk about.

# common errors: tokio::spawn
Now, the only thing that remains is a couple of like foot guns that are particularly common that people run into all the time.
And I'll go through these pretty rapidly because they're not huge topics. They're just things that you just need to be aware of so that you don't shoot yourself in the foot with them.
Um, okay, so number one. Tokyo spawn.
When you spawn something,
and I think I mentioned this earlier briefly, but I want to reiterate it. The join handle that you get back,
when you drop it, you're not waiting for that task. That task continues to run in the background.
And there are two implications of this. The first one is when you drop the join handle,
you're not guaranteed that that task is finished doing the work. But also, it continues to run.
It continues to use resources. And so because it spawned, because it's running in the background,
just dropping it is not sufficient if you want your system to not be wasting resources over time.
So if you truly want it to stop. then you need to do something like call abort so that Tokyo knows that the next time that future yields,
it shouldn't resume it, it should drop it and it should cancel it instead. So that's the first aspect of this that's important.
The second aspect of this is when you return from an asynchronous function that happened to run Tokyo spawn,
you now have this thing that's running in the background that might be doing something important. But from the outside,
from the caller's perspective, of the function that internally did a Tokyo spawn,
the caller doesn't know that there's now this background thing. Which means crucially, it doesn't know that it might have to wait for it.
So imagine that you have a background thread that's like, I don't know, you Tokyo spawned doing a, uh...
a file copy. Well, if your asynchronous main function calls a thing that calls a thing that calls a thing that ultimately spawns this thing that copies the files,
but nothing awaits the join handle, then what's going to happen is the thing that spawned the file copy returns,
and then we return, return, return, return all the way back to main. Main also doesn't know about this background spawn that's going on,
so main finishes. And at that point, all outstanding tasks are dropped because main exited.
And at that point, your file copy stops in the middle. And so it's really important to remember...
whether you want to do something whenever you do a spawn. Think about the fact,
or consider every time you do a spawn, assign it to a variable and decide what to do with that variable.
If you truly want it to be dropped, then you can drop it, that's fine, but you should make a conscious decision about whether dropping is the correct choice,
or whether it should actually be awaited, or whether it should actually be communicated further up, or whether it should actually be aborted.
But make a conscious decision rather than just letting the default behavior come back to bite you later.
Yeah, it's not as though a background task is necessarily useless, right? It could be you actually wanted to run in the background. You don't want it to be aborted.
You don't want it to be awaited. Like it's a, I don't know, it's your logger or it's forwarding something from a channel or reading something from a channel.
So it could be the correct decision is to sort of forget the handle, like drop the handle and just let the background task running.
It's more that it should be a conscious choice what you want to happen. I think the default behavior is actually reasonable as a default.
It's more that. You should really think about whether that default applies to you.

# common errors: concurrency vs parallelism
Okay, second thing to be aware of is that tasks are single-threaded.
So when you do a Tokyo spawn or whatever you do in your asyncfn main is a single task,
right? We've talked about what tasks are. It's a single task, which means that it's being executed by a single worker pool thread.
If you do a bunch of work in different futures, but they're all in a single top-level task,
that means all of that work is only being executed by a single thread. If you want work to be spread across multiple threads,
then you need to spawn those futures so they become top-level tasks so that they can be executed in parallel.
If you just use something like select or join or well not a join set because it spawns for you for exactly this reason but if you use a join or a select
or a futures unordered for that matter, what you end up with is a bunch of futures that are executing concurrently but not in parallel.
So they're all being executed by this one thread, but only one of the futures are being run at any given point in time.
That may or may not be what you want. Sometimes you do actually want to multiplex them on one thread, but you should be aware of the fact that futures embedded in futures are always single-threaded.
You need to spawn them and turn them into tasks in order to get parallelism.

# common errors: mpsc fan-in
And then the final thing I want to talk about is a foot gun that I see pretty often in an over-reliance on MPSC channels.
So we see this in particular with the actor model actually, where you spin up something that's an actor,
it owns, let's say, a TCP connection. And what you do is you keep sending messages to the actor for things like,
write these bytes, write these bytes, write these bytes, or handle this request, handle this request. The actor is again a single top-level task.
As a result, it only has a single thread. And as a result, that single thread has to handle all of those requests one at a time.
Now that's not inherently a problem, right? It might be that one at a time is the fastest you can write out on this TCP stream anyway.
And if you had multiple tasks all trying to write out to the same TCP stream, they would need a mutex or something.
So that's not inherently bad. The problem you run into is that when you have a sort of fan-in pattern into a single actor,
a single task that needs to do something about them, it needs to pay a cost for every channel received.
And again, if you think of these receives as being like, write these bytes or handle these requests, sometimes that's reasonable,
but very often the loop on the actor is probably going to look something like,
you know, while, uh, while let sum equals channel dot receive,
and then, you know, do a bunch of work with that message. That means that by the time it comes around,
depending on how much work there's in the loop, by the time it comes back around to read from the channel again, there might be five new things in that channel.
But it needs to read them one at a time, even though there might be more efficient ways to handle those incoming requests in batch.
How you choose to do that batching There are a bunch of different ways to do so.
You could use things like a... Well, watch is not really appropriate here, but you could imagine, for example,
that instead of having a channel to this actor, the input to this actor is actually a mutex over a vec rate.
And so if you want to send something to the actor, what you do is you take the mutex, you extend the vector with the bytes you want it to write,
and then you drop the mutex. And then the loop on the actor's side no longer looks like,
get, you know, one small set of bytes and then go again and get a small set of bytes.
Instead it takes the mutex, it takes the entire vector out and leaves an empty vector behind,
and then it writes all of those bytes out at once. That might be way more efficient.
and allow it to get to much higher bandwidth because it allows it to batch the cost across multiple calls into the actor.
This pattern doesn't always work, it doesn't always fit what the actor needs to do, but in general there's a tendency I see to over-rely on MPSC channels,
especially for actors, where it's often the right call for the simplicity of actors,
especially at lower throughput, but when you get to systems that strain the resources more,
you might want to start to think about these kinds of optimizations to amortize the cost of communication and synchronization here.

# Follow-up questions and outro
I think, I think those are all the things I want to say about Tokyo.
But I'll spend some time in chat and see if there are more things. Again, doesn't have to be about the sort of foot guns I just talked about,
but across the whole stream, let's do a sort of quick, I don't want to say Q&A, but a quick, everything I talked about at Tokyo,
are there things that you still get confused by? You want more detail on things that have bit you in the past to sort of round off the stream here.
It sounds a bit like a ring buffer. Yeah, it is kind of like a ring buffer. In fact, you can use a ring buffer for this if you're willing to copy out of it rather than steal.
What things in Tokyo are actually parallel, not just concurrent? It's only Tokyo spawn,
and of course runtime spawn, which is the same. Spawn blocking is parallel, because it runs on a different thread.
Join set is parallel, because when you give a future to a join set,
it spawns them. So basically the answer is only things that are spawned are parallel.
And the IO event loop is parallel.
When doing batching, usually you want to parameterize it with two inputs, the max batch size and the max wait time for a batch to fill.
Is there something in the Tokyo ecosystem that implements such a pattern? Not that I know of.
But we're talking like five lines of code. So I don't think so.
But maybe.
Request says it wants a Tokyo 1.x runtime when running with another runtime. Is Tokyo doing something special that request needs?
Well, so… Well, so…
This gets at the fact that in order for Tokyo to know when futures need to move from the non-runnable queue to the runnable queue,
it needs to know which things those futures are waiting for. Which means that when those futures use,
for example, a Tokyo TCP stream, that's how Tokyo knows. It's because Tokyo implemented the TCP stream,
so it knows what to wait for. So it knows when to move them and basically how to implement wake.
If you try to use a non-Tokyo runtime, but you're trying to use the Tokyo resources,
you run into a problem because the Tokyo resources, like a TCP stream, when you call poll on it and it realizes,
like, let's say you call poll read on a TCP stream, it knows that, oh, I didn't have any bytes to read.
I need to register myself somewhere so that the runtime... So that I can notify the runtime when I need to resume,
basically I need to register myself with the I-O event loop. So it looks for an I-O event loop,
specifically the Tokyo I-O event loop, and it doesn't find one. Then it doesn't know where to put its waker and its file descriptor for something to be picked up later.
And so that's why when you use the Tokyo I-O resources, you need to also use the Tokyo runtime because the I-O resources are tied to the I-O event loop.
If you use, let's say, the async stood runtime or something, but you're trying to use a Tokyo resource,
then when a pull read happens, it doesn't have anywhere to put its state that it can rely on will call wake later.
And so request, I believe, uses Tokyo resources all the way down.
Like it uses Tokyo TCP streams, for example. And you can't run that in a non-Tokyo runtime because that connection with the IO event loop would be lost.
Does Tokyo not do MPMC? No, Tokyo does not have a multi-producer, multi-consumer channel.
If this hasn't been asked already, why does the async writeExt have implementations for each individual type, such as writeU8,
instead of a generic time parameter? I think that one is because it's not clear what it would be generic over.
Let's say you had a write number, and it's generic over t.
Where t is what? What is the bound that allows it to take a number and turn it into a sequence of bytes?
And it gets complicated, right? If you get a U128, the order in which you write the individual bytes of a
U128 out to the writer depends on the endianness, for example.
And so there's no generic bound to really express that. And so that's why you end up with individual methods.
Can you talk about broadcast and the overhead that comes with having a bunch of receivers and senders?
So there shouldn't be that much overhead when using broadcast with multiple receivers.
I don't think broadcast allows multiple sender. Is it sender?
Oh yeah, send takes a reference to self. Okay. So there's not,
I think, a huge overhead to having multiple. The main problem you run into with broadcasts is if you have slow readers,
they hold up all of the readers, or rather, it's not quite true. They don't hold up the readers,
but they either hold up the sender or they cause rights to start to get dropped and they start losing messages.
But that's just sort of an inherent problem here. This is the lagging problem that
Does TokyoSpawn create an OS thread or a green thread? It creates a green thread. What about keeping a mutex guard between awaits?
We already talked about that earlier. Is there a way to access the blocking thread pool of the runtime? Can I get a handle to it and send it somewhere else?
Sort of. So if you look at runtime, there is a method called handle that gives you a handle.
And on a handle, which is clone, there is a spawn blocking. And spawn blocking lets you run something on the pool and get an asynchronous future back.
So you can pass a handle around and use that to spawn things onto the blocking pool.
But that is your handle to the blocking pool.
Can you talk about how you deal with handling what would normally require async drop, like losing third party async libraries you would like to clean up on drop,
but those cleanups would be async? There's not a great answer to this. Async drop just, it sucks that we don't have it and it's really hard to get.
You will see that for a bunch of types, not close is a bad example of this, but what is a good example of this?
I guess actually maybe file.
No, file is also a bad example here.
I guess TCP stream, maybe. Or buff stream.
Maybe buff stream is the one I want to talk about.
Although maybe that doesn't solve the problem either. I guess maybe there is an example of this in Tokyo. So what I'm looking for is,
you'll see a bunch of asynchronous libraries have an asynchronous function called close or something like it.
And the idea is that if you know that you want to drop this type, then you should call this function in order to drop it.
So close would be an asynchronous function that consumes self. And so the idea is that if you have the ability to manually drop it,
like you know that you're going to drop this thing, rather than relying on the drop trait, like it just going out of scope,
call this method instead, and then it will actually get to do its asynchronous cleanup.
You also see this actually in synchronous methods where you might have a close function just because the close thing might error.
And so if you just let the thing be dropped, the error from the result is just silenced,
it's forgotten. So you are encouraged to call the close method if you can in order to recover that error or see that error and not have it just be silenced.
And you can, the same thing applies for async, that you can provide an async function close. That doesn't perfectly solve the problem,
right? Because there are cases where it's just going to be dropped and you don't really have control over logic that happens on drop,
except through the drop trade. When that happens, the best thing that I know of here is
So there's a handle current or try current that lets you get a handle to the current
Tokyo runtime. And then. When you get that handle,
you can then spawn code that would get to do your cleanup stuff.
It's not beautiful. It means, for example, that if you get dropped off of the runtime,
then now you don't get to execute anything to do that cleanup properly. But it is sort of a best effort asynchronous cleanup.
Ultimately, though, this is a problem and we just don't have a great solution for it.
Um...
Tokyo Sync broadcast is not a multi-producer, multi-consumer. No. Broadcast, so broadcast is specifically not a multi-producer,
multi-consumer queue. It is a broadcast queue. What that means is,
and the distinction is, an MPMC channel is any sender can send a thing and one of the consumers will get that thing.
A broadcast is any sender can send a thing, all consumers will get that thing.
So the semantics are different. Uh,
does the can-do the cancel safe futures or methods internally revert state?
How do they know when to do that if they can become canceled in the middle? Um, it depends actually. So some of them are just written in such a way that they are cancellation safe.
So they don't actually have to revert anything. They're just written in such a way that there's no a wait point between the-
the sort of commit point, like the point where they change externally visible state and when they return. And so therefore they're just naturally cancellation safe.
And some of them will actually have a sort of drop guard. So they'll construct a type internally that they create an instance of before they do some stuff.
And so if the whole thing gets dropped, the drop implementation of that instance will do some revert stuff.
That is a much more complicated pattern, but we do see it sometimes.
Do you think Tokyo will remain the most common asynchronous runtime in the future?
At least for the foreseeable future, I don't see any particularly relevant general purpose runtimes at least.
There are runtimes that are better suited for sort of specific domains, for example, like embedded,
but I don't know of any that are sort of compelling as a general purpose executor at the moment.
Does the send method on channels call wake? Yes.
That is exactly the way in which they allow the receiver to realize that,
oh, this future has now moved from non-runnable to runnable.
Why doesn't Tokyo implement Unix sockets of the type SOCKSECPACKET?
I don't know. I don't know what that- variant of Unix sockets are. There are a bunch of like utility crates around Tokyo that provide like alternate socket types,
for example. I think also there is, so this is a thing called asyncfd,
which is basically a wrapper type specifically on Linux on anything that has a file descriptor that implements the standard
Unix sort of read and write interface. So you might be able to use that, but...
Tokyo is never going to support like every single possible resource directly in the library.
That would just not be feasible. But the idea is that you should be able to write additional resources as other crates that people can use and they will interoperate nicely through things like the Waker.
Would changing an asynchronous function that is pubbed from cancellation safe to non-cancellation safe be considered a breaking change for SemVer?
That's a very good question. There's no well-defined answer to this.
I would claim that the answer is yes. Because you have changed the correctness contract of the function.
Right? Like, someone has been using your code correctly, and now you have made their code incorrect by changing your code,
and they were relying on something that was at least assuming the cancellation safety was publicly documented before,
then it would be a breaking change. If you never promised anything about cancellation safety, then no, I don't think it's a breaking change.
So this becomes a question of documentation, really. Did you ever promise that it was cancellation safe?
In general, when you call... third party libraries for things, you should be pessimistic about what they promise you.
So if they don't guarantee a thing, you're not allowed to assume that thing. Or if you assume it, you should also specifically check it because it's allowed to change.
Any plans on decrusting the small ecosystem? No. I don't think it's particularly relevant.
Toki is not really good for real-time applications. I mean, that's generally true for anything that doesn't use a real-time operating system.
But this sort of gets at what I was implying with embedded, right, is that there are runtimes that are better for specific kinds of use cases.
So I was more referring to a general-purpose asynchronous runtime.
Are there any workarounds for synchronous to asynchronous to synchronous to asynchronous call stacks? I mean, the answer is basically don't do it.
It will come back to bite you. The slightly more helpful answer is channels are usually the way that you do this,
whether one shot or MPSC. They are the best means we have for really bridging that gap.
Things like block in place are crutches where if you use them a lot, or similar like handle current,
if you use them a lot, you're going to run into really sad times as a result.
Because those nested patterns end up just performing really poorly, having really hard to debug runtime failures.
You don't generally want those sync async sandwiches.
Okay, I think I got to the end of chat. Oh, okay, so I hate this so much.
YouTube has this feature now, so I have the chat window up on the side and it defaults to top chat,
not live chat, top chat, which means that I might have missed a bunch of messages. Let me change that to live chat and see what I've missed.
Okay, it looks like I haven't really missed anything. Great. I am not an Elixir user, no.
What are good strategies for migrating from sync to async? I find that trying to write certain modules using a runtime and throwing handles around is difficult.
There's not a general purpose solution here for rewriting things from sync to async.
I think I've usually found that you kind of want to go outside in rather than inside out. Like trying to write,
wrap asynchronous things with synchronous things tends to be a bunch of pain because...
You now end up spawning multiple runtimes in each place where there's an interface. If you start from the top,
you can have one asynchronous runtime, and initially you're going to have things that block the runtime, and you can use block in place or spawn blocking.
And so that direction of the port tends to be easier, and then you just push async more and more down.
So that tends to work better.
The problem with real-time async is I don't think Rust has the right abstraction since real-time requires to some extent detail how to make a CPU...
How much CPU a process is going to consume. Yeah, it's unclear whether in real-time operating systems this is the right interface.
It might be, but it's much less clear.
The domain is without boots. Without dot boats.
Are you a heavy user of Tokyo? At Helsing? I mean, yeah. Tokyo is the asynchronous runtime.
For general purpose asynchronous code, I don't see a reason to use anything else.
Okay, I think that's it. I think we're done. And hey,
okay, I started at 2, 3 hours and 33 minutes and 33 seconds.
I think I guessed 3 to 3 and a half hours, so I'm pretty happy with that.
All right, great everyone, thank you for coming out, and I hope you found that interesting, and I'll see you for the next stream.

All

From the series
