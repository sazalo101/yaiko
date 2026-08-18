//! Background job processing for Yaiko applications.
//!
//! Provides an async job queue with bounded retries, optional timeouts, and
//! inspectable dead-letter records.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify};

pub type JobFn =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadLetter {
    pub name: String,
    pub retries: u32,
    pub error: String,
}

pub struct JobQueue {
    jobs: Arc<Mutex<VecDeque<Job>>>,
    dead_letters: Arc<Mutex<Vec<DeadLetter>>>,
    notify: Arc<Notify>,
    running: Arc<Mutex<bool>>,
}

pub struct Job {
    pub name: String,
    pub retries: u32,
    pub max_retries: u32,
    timeout: Option<Duration>,
    retry_backoff: Duration,
    task: JobFn,
}

impl Job {
    pub fn new<F, Fut>(name: &str, task: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        Self {
            name: name.to_string(),
            retries: 0,
            max_retries: 3,
            timeout: None,
            retry_backoff: Duration::from_secs(2),
            task: Box::new(move || Box::pin(task())),
        }
    }

    pub fn max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn retry_backoff(mut self, backoff: Duration) -> Self {
        self.retry_backoff = backoff;
        self
    }
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(VecDeque::new())),
            dead_letters: Arc::new(Mutex::new(Vec::new())),
            notify: Arc::new(Notify::new()),
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn enqueue(&self, job: Job) {
        self.jobs.lock().await.push_back(job);
        self.notify.notify_one();
    }

    pub async fn add<F, Fut>(&self, name: &str, task: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.enqueue(Job::new(name, task)).await;
    }

    pub async fn pending_count(&self) -> usize {
        self.jobs.lock().await.len()
    }

    pub async fn dead_letters(&self) -> Vec<DeadLetter> {
        self.dead_letters.lock().await.clone()
    }

    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            *self.running.lock().await = true;

            loop {
                tokio::select! {
                    _ = self.notify.notified() => {},
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {},
                }

                if !*self.running.lock().await {
                    break;
                }

                let job = self.jobs.lock().await.pop_front();
                if let Some(mut job) = job {
                    tracing::info!(job_name = %job.name, "Processing job");
                    let result = if let Some(timeout) = job.timeout {
                        match tokio::time::timeout(timeout, (job.task)()).await {
                            Ok(result) => result,
                            Err(_) => {
                                Err(format!("job timed out after {} seconds", timeout.as_secs()))
                            }
                        }
                    } else {
                        (job.task)().await
                    };

                    match result {
                        Ok(()) => {
                            tracing::info!(job_name = %job.name, "Job completed successfully")
                        }
                        Err(error) => {
                            job.retries += 1;
                            if job.retries < job.max_retries {
                                tracing::warn!(job_name = %job.name, error = %error, retry = job.retries, "Job failed, retrying");
                                tokio::time::sleep(job.retry_backoff).await;
                                self.jobs.lock().await.push_back(job);
                                self.notify.notify_one();
                            } else {
                                tracing::error!(job_name = %job.name, error = %error, "Job moved to dead letter queue");
                                self.dead_letters.lock().await.push(DeadLetter {
                                    name: job.name,
                                    retries: job.retries,
                                    error,
                                });
                            }
                        }
                    }
                }
            }
        })
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn failed_jobs_are_retained_as_dead_letters() {
        let queue = Arc::new(JobQueue::new());
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_job = attempts.clone();
        queue
            .enqueue(
                Job::new("always-fails", move || {
                    let attempts = attempts_for_job.clone();
                    async move {
                        attempts.fetch_add(1, Ordering::SeqCst);
                        Err("boom".to_string())
                    }
                })
                .max_retries(1)
                .retry_backoff(Duration::from_millis(1)),
            )
            .await;

        let worker = queue.clone().start();
        tokio::time::sleep(Duration::from_millis(30)).await;
        queue.stop().await;
        worker.await.unwrap();

        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert_eq!(queue.dead_letters().await[0].name, "always-fails");
    }
}
