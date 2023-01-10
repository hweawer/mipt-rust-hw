#![forbid(unsafe_code)]

use crossbeam::channel::{self, Receiver, Sender};

use std::any::Any;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    thread,
};

////////////////////////////////////////////////////////////////////////////////

type Job = Box<dyn FnOnce() + Send>;

enum Message {
    Task(Job),
    Terminate,
}

pub struct ThreadPool {
    tasks: Arc<AtomicI32>,
    threads: Vec<JoinHandle<()>>,
    sender: Sender<Message>,
}

impl ThreadPool {
    pub fn new(thread_count: usize, queue_size: usize) -> Self {
        assert!(thread_count > 0);
        assert!(queue_size > 0);
        let (sender, receiver) = channel::bounded(queue_size);
        let mut threads = Vec::with_capacity(thread_count);
        let tasks_in_pool = Arc::new(AtomicI32::new(0));
        for _ in 0..thread_count {
            let receiver: Receiver<Message> = receiver.clone();
            let tasks = Arc::clone(&tasks_in_pool);
            thread::spawn(move || loop {
                for mes in &receiver {
                    match mes {
                        Message::Task(job) => {
                            let _ = catch_unwind(AssertUnwindSafe(job));
                            tasks.fetch_add(-1, Relaxed);
                        }
                        Message::Terminate => {
                            break;
                        }
                    }
                }
            });
        }
        Self {
            threads,
            tasks: tasks_in_pool,
            sender,
        }
    }

    pub fn spawn<F: Send + 'static, T: Send + 'static>(&self, task: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T,
    {
        self.tasks.fetch_add(1, Relaxed);
        let (sender, receiver) = channel::bounded(1);
        let job = Box::new(move || {
            let res = task();
            let _ = sender.send(res);
        });
        let _ = self.sender.send(Message::Task(job));
        JoinHandle { receiver }
    }

    pub fn shutdown(self) {
        loop {
            if self.tasks.load(Relaxed) == 0 {
                break;
            }
        }
        for _ in 0..self.threads.len() {
            self.sender.send(Message::Terminate).unwrap();
        }

        for thread in self.threads {
            thread.join().unwrap();
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct JoinHandle<T> {
    receiver: Receiver<T>,
}

#[derive(Debug)]
pub struct JoinError {
    inner: Box<dyn Any + Send + 'static>,
}

impl<T> JoinHandle<T> {
    pub fn join(self) -> Result<T, JoinError> {
        self.receiver
            .recv()
            .map_err(|e| JoinError { inner: Box::new(e) })
    }
}
