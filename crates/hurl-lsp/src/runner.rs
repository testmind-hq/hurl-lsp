use dashmap::DashMap;
use std::{
    io,
    process::ExitStatus,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{io::AsyncReadExt, process::Command, sync::watch};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunOutcome {
    Completed,
    Cancelled,
    TimedOut,
}

#[derive(Debug)]
pub struct RunnerOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub outcome: RunOutcome,
    pub process_ms: u64,
}

#[derive(Clone, Default)]
pub struct RunTaskRegistry {
    active: Arc<DashMap<String, watch::Sender<bool>>>,
    sequence: Arc<AtomicU64>,
}

impl RunTaskRegistry {
    pub fn next_id(&self) -> String {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        format!("run-{millis}-{sequence}")
    }

    pub fn register(&self, task_id: &str) -> watch::Receiver<bool> {
        let (sender, receiver) = watch::channel(false);
        self.active.insert(task_id.to_string(), sender);
        receiver
    }

    pub fn cancel(&self, task_id: &str) -> bool {
        self.active
            .get(task_id)
            .is_some_and(|sender| sender.send(true).is_ok())
    }

    pub fn finish(&self, task_id: &str) {
        self.active.remove(task_id);
    }

    #[cfg(test)]
    fn contains(&self, task_id: &str) -> bool {
        self.active.contains_key(task_id)
    }
}

pub async fn execute(
    command: &mut Command,
    mut cancellation: watch::Receiver<bool>,
    timeout: Option<Duration>,
) -> io::Result<RunnerOutput> {
    command.kill_on_drop(true);
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    let started = Instant::now();
    let mut child = command.spawn()?;
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let stdout_reader = tokio::spawn(async move {
        let mut bytes = Vec::new();
        if let Some(stream) = stdout.as_mut() {
            stream.read_to_end(&mut bytes).await?;
        }
        Ok::<_, io::Error>(bytes)
    });
    let stderr_reader = tokio::spawn(async move {
        let mut bytes = Vec::new();
        if let Some(stream) = stderr.as_mut() {
            stream.read_to_end(&mut bytes).await?;
        }
        Ok::<_, io::Error>(bytes)
    });

    let sleep_duration = timeout.unwrap_or(Duration::from_secs(365 * 24 * 60 * 60));
    let outcome;
    let status;
    tokio::select! {
        result = child.wait() => {
            status = result?;
            outcome = RunOutcome::Completed;
        }
        changed = cancellation.changed() => {
            if changed.is_ok() && *cancellation.borrow() {
                let _ = child.kill().await;
                status = child.wait().await?;
                outcome = RunOutcome::Cancelled;
            } else {
                status = child.wait().await?;
                outcome = RunOutcome::Completed;
            }
        }
        _ = tokio::time::sleep(sleep_duration), if timeout.is_some() => {
            let _ = child.kill().await;
            status = child.wait().await?;
            outcome = RunOutcome::TimedOut;
        }
    }
    let stdout = stdout_reader
        .await
        .map_err(|error| io::Error::other(error.to_string()))??;
    let stderr = stderr_reader
        .await
        .map_err(|error| io::Error::other(error.to_string()))??;
    Ok(RunnerOutput {
        status,
        stdout,
        stderr,
        outcome,
        process_ms: started.elapsed().as_millis() as u64,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn timeout_terminates_the_child_and_marks_task_timed_out() {
        let registry = RunTaskRegistry::default();
        let receiver = registry.register("timeout");
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 2"]);
        let output = execute(&mut command, receiver, Some(Duration::from_millis(20)))
            .await
            .expect("runner");
        registry.finish("timeout");
        assert_eq!(output.outcome, RunOutcome::TimedOut);
        assert!(!registry.contains("timeout"));
    }

    #[tokio::test]
    async fn cancellation_terminates_the_child() {
        let registry = RunTaskRegistry::default();
        let receiver = registry.register("cancel");
        let cancelling = registry.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            assert!(cancelling.cancel("cancel"));
        });
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 2"]);
        let output = execute(&mut command, receiver, Some(Duration::from_secs(1)))
            .await
            .expect("runner");
        registry.finish("cancel");
        assert_eq!(output.outcome, RunOutcome::Cancelled);
        assert!(!registry.contains("cancel"));
    }
}
