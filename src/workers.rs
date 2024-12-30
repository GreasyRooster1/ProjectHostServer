use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};
use std::fmt::format;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = (Box<dyn FnOnce() + Send + 'static>,usize);

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = (Box::new(f) as Box<dyn FnOnce() + Send>,0usize);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let name = format!("worker-{id}");
        let builder = thread::Builder::new().name(name);

        let thread = builder.spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("worker {id} got a job, executing...");

                    let (func,id)=job;
                    func();
                }
                Err(_) => {
                    println!("worker {id} got disconnected");
                    break;
                }
            }
            println!("worker {id} finished job, awaiting");
        }).unwrap();
        Worker {
            id,
            thread: Some(thread),
        }
    }
    pub fn id(&self)->usize{
        self.id
    }
}