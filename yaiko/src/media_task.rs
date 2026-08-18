//! Media-processing task orchestration over the task-result store.

use crate::media_processing::FfmpegJobSpec;
use crate::task_results::{TaskResultError, TaskResultStore, TaskState};
use std::future::Future;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaTaskError {
    Task(TaskResultError),
    Cancelled,
    ProcessFailed(String),
}

#[derive(Clone)]
pub struct MediaTask {
    id: String,
    spec: Arc<FfmpegJobSpec>,
    results: TaskResultStore,
}

impl MediaTask {
    pub fn create(results: TaskResultStore, spec: FfmpegJobSpec) -> Result<Self, MediaTaskError> {
        let record = results.create().map_err(MediaTaskError::Task)?;
        Ok(Self {
            id: record.id,
            spec: Arc::new(spec),
            results,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn spec(&self) -> &FfmpegJobSpec {
        &self.spec
    }
    pub fn start(&self) -> Result<(), MediaTaskError> {
        self.results.start(&self.id).map_err(MediaTaskError::Task)
    }
    pub fn progress(&self, percent: u8) -> Result<u8, MediaTaskError> {
        self.results
            .progress(&self.id, percent)
            .map_err(MediaTaskError::Task)
    }
    pub fn request_cancel(&self) -> Result<(), MediaTaskError> {
        self.results
            .request_cancel(&self.id)
            .map_err(MediaTaskError::Task)
    }
    pub fn cancellation_requested(&self) -> bool {
        self.results
            .get(&self.id)
            .map(|task| {
                task.state == TaskState::CancelRequested || task.state == TaskState::Cancelled
            })
            .unwrap_or(true)
    }
    pub fn result(&self) -> Option<crate::task_results::TaskResult> {
        self.results.get(&self.id)
    }
    pub async fn execute<F, Fut>(&self, runner: F) -> Result<Vec<u8>, MediaTaskError>
    where
        F: FnOnce(Arc<FfmpegJobSpec>) -> Fut,
        Fut: Future<Output = Result<Vec<u8>, String>> + Send,
    {
        if self.cancellation_requested() {
            return Err(MediaTaskError::Cancelled);
        }
        self.start()?;
        if self.cancellation_requested() {
            let _ = self.results.cancel(&self.id);
            return Err(MediaTaskError::Cancelled);
        }
        let output = runner(self.spec.clone()).await;
        if self.cancellation_requested() {
            let _ = self.results.cancel(&self.id);
            return Err(MediaTaskError::Cancelled);
        }
        match output {
            Ok(bytes) => {
                self.results
                    .succeed(&self.id, bytes.clone())
                    .map_err(MediaTaskError::Task)?;
                Ok(bytes)
            }
            Err(error) => {
                self.results
                    .fail(&self.id, error.clone())
                    .map_err(MediaTaskError::Task)?;
                Err(MediaTaskError::ProcessFailed(error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media_processing::FfmpegJobSpec;

    fn task() -> MediaTask {
        MediaTask::create(
            TaskResultStore::new(4),
            FfmpegJobSpec::new("input.mp4", "output.mp4").unwrap(),
        )
        .unwrap()
    }
    #[tokio::test]
    async fn maps_success_and_progress_to_task_result() {
        let task = task();
        task.progress(0).unwrap_err();
        let result = task
            .execute(|_| async { Ok::<_, String>(b"rendered".to_vec()) })
            .await
            .unwrap();
        assert_eq!(result, b"rendered");
        let record = task.result().unwrap();
        assert_eq!(record.state, TaskState::Succeeded);
        assert_eq!(record.progress_percent, 100);
    }
    #[tokio::test]
    async fn maps_process_failure() {
        let task = task();
        let error = task
            .execute(|_| async { Err::<Vec<u8>, _>("ffmpeg exit 1".into()) })
            .await
            .unwrap_err();
        assert_eq!(error, MediaTaskError::ProcessFailed("ffmpeg exit 1".into()));
        assert_eq!(task.result().unwrap().state, TaskState::Failed);
    }
    #[tokio::test]
    async fn honors_cancellation_before_execution() {
        let task = task();
        task.request_cancel().unwrap();
        let error = task
            .execute(|_| async { Ok::<_, String>(Vec::new()) })
            .await
            .unwrap_err();
        assert_eq!(error, MediaTaskError::Cancelled);
        assert_eq!(task.result().unwrap().state, TaskState::Cancelled);
    }
    #[tokio::test]
    async fn reports_progress_during_running_work() {
        let task = task();
        task.start().unwrap();
        assert_eq!(task.progress(45).unwrap(), 45);
        assert_eq!(task.result().unwrap().progress_percent, 45);
    }
}
