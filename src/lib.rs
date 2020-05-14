use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::{
    sync::mpsc,
    thread::{self, JoinHandle},
};

#[derive(Debug)]
pub enum PoolCreationError {
    ThreadsNum,
}

struct Worker {
    id: usize,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn new(id: usize, jobs: Arc<Mutex<Receiver<Message>>>) -> Self {
        println!("👨‍💻 spawning thread pool worker n:{}", id);
        let thread = thread::spawn(move || loop {
            // By using loop instead and acquiring the lock without assigning to a variable, the
            // temporary MutexGuard returned from the lock method is dropped as soon as the let job
            // statement ends.
            // This ensures that the lock is held during the call to recv, but it is released before
            // the call to job(), allowing multiple requests to be serviced concurrently.
            match jobs.lock().unwrap().recv().unwrap() {
                Message::NewJob(job) => {
                    println!("worker {} got a job; executing.", id);
                    job();
                }

                Message::Terminate => {
                    println!("worker {} terminating", id);
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> Self {
        ThreadPool::new_with_result(size).expect("")
    }

    pub fn new_with_result(size: usize) -> Result<ThreadPool, PoolCreationError> {
        match size {
            0 => Err(PoolCreationError::ThreadsNum),
            _ => {
                let (sender, receiver) = mpsc::channel();
                let receiver = Arc::new(Mutex::new(receiver));
                let workers: Vec<Worker> = (0..size)
                    .map(move |i| Worker::new(i, Arc::clone(&receiver)))
                    .collect();

                Ok(ThreadPool { workers, sender })
            }
        }
    }

    pub fn execute<T: FnOnce() + Send + 'static>(&self, job: T) {
        let msg = Message::NewJob(Box::new(job));
        self.sender.send(msg).unwrap()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Sending terminate message to all workers.");

        // If we tried to send a message and join immediately in the same loop, we couldn’t
        // guarantee that the worker in the current iteration would be the one to get the message
        // from the channel.
        // To better understand why we need two separate loops, imagine a scenario with two workers.
        // If we used a single loop to iterate through each worker, on the first iteration a
        // terminate message would be sent down the channel and join called on the first worker’s
        // thread.
        // If that first worker was busy processing a request at that moment, the second worker
        // would pick up the terminate message from the channel and shut down. We would be left
        // waiting on the first worker to shut down, but it never would because the second thread
        // picked up the terminate message. Deadlock!
        // To prevent this scenario, we first put all of our Terminate messages on the channel in
        // one loop; then we join on all the threads in another loop. Each worker will stop
        // receiving requests on the channel once it gets a terminate message. So, we can be sure
        // that if we send the same number of terminate messages as there are workers, each worker
        // will receive a terminate message before join is called on its thread.
        for _ in &self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        println!("Shutting down all workers.");

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
