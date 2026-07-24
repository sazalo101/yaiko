//! Background job processing for Yaiko applications
//!
//! Provides a simple async job queue for background task execution.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

/// A boxed async function that can be stored in the queue
pub type JobFn = Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> + Send + Sync>;

/// Background job processor
pub struct JobQueue {
    jobs: Arc<Mutex<VecDeque<Job>>>,
    notify: Arc<Notify>,
    running: Arc<Mutex<bool>>,
}

/// A job in the queue
pub struct Job {
    pub name: String,
    pub retries: u32,
    pub max_retries: u32,
    task: JobFn,
}

impl Job {
    /// Create a new job
    pub fn new<F, Fut>(name: &str, task: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        Self {
            name: name.to_string(),
            retries: 0,
            max_retries: 3,
            task: Box::new(move || Box::pin(task())),
        }
    }

    /// Set max retries
    pub fn max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }
}

impl JobQueue {
    /// Create a new job queue
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Add a job to the queue
    pub async fn enqueue(&self, job: Job) {
        self.jobs.lock().await.push_back(job);
        self.notify.notify_one();
    }

    /// Add a simple job by name and closure
    pub async fn add<F, Fut>(&self, name: &str, task: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.enqueue(Job::new(name, task)).await;
    }

    /// Get the number of pending jobs
    pub async fn pending_count(&self) -> usize {
        self.jobs.lock().await.len()
    }

    /// Start processing jobs in the background
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            *self.running.lock().await = true;
            
            loop {
                // Wait for notification or timeout
                tokio::select! {
                    _ = self.notify.notified() => {},
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {},
                }

                // Check if still running
                if !*self.running.lock().await {
                    break;
                }

                // Process next job
                let job = self.jobs.lock().await.pop_front();
                if let Some(mut job) = job {
                    tracing::info!(job_name = %job.name, "Processing job");
                    
                    let result = (job.task)().await;
                    
                    match result {
                        Ok(()) => {
                            tracing::info!(job_name = %job.name, "Job completed successfully");
                        }
                        Err(e) => {
                            job.retries += 1;
                            if job.retries < job.max_retries {
                                tracing::warn!(
                                    job_name = %job.name, 
                                    error = %e,
                                    retry = job.retries,
                                    "Job failed, retrying"
                                );
                                // Exponential backoff: 2^retries seconds
                                let backoff = tokio::time::Duration::from_secs(2u64.pow(job.retries));
                                tokio::time::sleep(backoff).await;
                                // Re-queue the job back
                                self.jobs.lock().await.push_back(job);
                                self.notify.notify_one();
                            } else {
                                tracing::error!(
                                    job_name = %job.name,
                                    error = %e,
                                    "Job failed after max retries"
                                );
                            }
                        }
                    }
                }
            }
        })
    }

    /// Stop processing jobs
    pub async fn stop(&self) {
        *self.running.lock().await = false;
        self.notify.notify_one();
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}
