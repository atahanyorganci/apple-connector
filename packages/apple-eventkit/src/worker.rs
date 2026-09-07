//! Serial owner for an Objective-C resource.
//!
//! The resource is built on, and never leaves, a dedicated OS thread. Callers submit boxed
//! closures that borrow it for the duration of one job; only `Send` values cross the boundary in
//! either direction. That gives three properties the rest of the crate relies on:
//!
//! - exactly one owner, so no raw pointer is ever detached from Rust ownership;
//! - a single-consumer job channel, so framework calls are strictly serial;
//! - a timeout that abandons a *result*, not a borrow — a job that outruns its budget keeps
//!   running against a resource the worker still owns, so nothing can dangle.

use std::{sync::mpsc, thread, time::Duration};

use thiserror::Error;
use tokio::sync::oneshot;

/// One unit of framework work, erased so the channel does not need the result type.
type Job<R> = Box<dyn FnOnce(&R) + Send + 'static>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum WorkerError {
    /// The worker thread could not be started, or stopped before the job ran.
    #[error("framework worker is not running")]
    Stopped,
    /// The worker dropped the job without reporting a result.
    #[error("framework worker dropped the job")]
    Lost,
    /// The job did not report a result within its budget.
    #[error("framework job timed out")]
    TimedOut,
}

/// Handle to the thread that owns `R`.
///
/// Dropping the handle closes the job channel, which ends the worker loop and releases the
/// resource on its own thread. The handle deliberately does not join: dropping it from an async
/// context must not block the runtime.
pub(crate) struct Worker<R> {
    jobs: mpsc::Sender<Job<R>>,
}

impl<R: 'static> Worker<R> {
    /// Starts the worker and returns it alongside whatever `build` reported about the resource.
    ///
    /// `build` runs on the worker thread, so `R` itself never has to be `Send`.
    pub(crate) fn spawn<F, I>(name: &str, build: F) -> Result<(Self, I), WorkerError>
    where
        F: FnOnce() -> (R, I) + Send + 'static,
        I: Send + 'static,
    {
        let (jobs, requests) = mpsc::channel::<Job<R>>();
        let (ready, started) = mpsc::sync_channel::<I>(1);

        thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                let (resource, report) = build();
                if ready.send(report).is_err() {
                    return;
                }
                while let Ok(job) = requests.recv() {
                    job(&resource);
                }
            })
            .map_err(|_| WorkerError::Stopped)?;

        let report = started.recv().map_err(|_| WorkerError::Stopped)?;
        Ok((Self { jobs }, report))
    }

    /// Runs `f` on the worker thread and waits up to `budget` for its result.
    pub(crate) async fn run<F, T>(&self, budget: Duration, f: F) -> Result<T, WorkerError>
    where
        F: FnOnce(&R) -> T + Send + 'static,
        T: Send + 'static,
    {
        let (result, completion) = oneshot::channel::<T>();
        self.jobs
            .send(Box::new(move |resource| {
                let _ = result.send(f(resource));
            }))
            .map_err(|_| WorkerError::Stopped)?;

        match tokio::time::timeout(budget, completion).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(_)) => Err(WorkerError::Lost),
            Err(_) => Err(WorkerError::TimedOut),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::{Worker, WorkerError};

    #[derive(Debug, PartialEq, Eq)]
    enum Mark {
        Enter(u32),
        Exit(u32),
    }

    fn runtime() -> Result<tokio::runtime::Runtime, Box<dyn std::error::Error>> {
        Ok(tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_time()
            .build()?)
    }

    /// `RefCell` is neither `Send` nor `Sync`; that this compiles is itself the proof that the
    /// resource never leaves the worker thread.
    #[test]
    fn jobs_never_overlap() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = runtime()?;
        let (worker, ()) = Worker::spawn("test-overlap", || (RefCell::new(Vec::new()), ()))?;
        let worker = std::sync::Arc::new(worker);

        runtime.block_on(async {
            let mut handles = Vec::new();
            for index in 0..8_u32 {
                let worker = std::sync::Arc::clone(&worker);
                handles.push(tokio::spawn(async move {
                    worker
                        .run(
                            std::time::Duration::from_secs(5),
                            move |log: &RefCell<Vec<Mark>>| {
                                log.borrow_mut().push(Mark::Enter(index));
                                std::thread::sleep(std::time::Duration::from_millis(2));
                                log.borrow_mut().push(Mark::Exit(index));
                            },
                        )
                        .await
                }));
            }
            for handle in handles {
                let _ = handle.await;
            }
        });

        let marks = runtime.block_on(worker.run(
            std::time::Duration::from_secs(5),
            |log: &RefCell<Vec<Mark>>| {
                log.borrow()
                    .iter()
                    .map(|mark| match mark {
                        Mark::Enter(index) => Mark::Enter(*index),
                        Mark::Exit(index) => Mark::Exit(*index),
                    })
                    .collect::<Vec<_>>()
            },
        ))?;

        assert_eq!(marks.len(), 16, "every job should record entry and exit");
        for pair in marks.chunks(2) {
            match pair {
                [Mark::Enter(entered), Mark::Exit(exited)] => assert_eq!(
                    entered, exited,
                    "a second job entered before the previous one exited"
                ),
                other => return Err(format!("jobs interleaved: {other:?}").into()),
            }
        }
        Ok(())
    }

    #[test]
    fn jobs_run_in_submission_order() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = runtime()?;
        let (worker, ()) = Worker::spawn("test-order", || (RefCell::new(Vec::new()), ()))?;

        runtime.block_on(async {
            for index in 0..6_u32 {
                worker
                    .run(
                        std::time::Duration::from_secs(5),
                        move |log: &RefCell<Vec<u32>>| {
                            log.borrow_mut().push(index);
                        },
                    )
                    .await?;
            }
            Ok::<(), WorkerError>(())
        })?;

        let seen = runtime.block_on(worker.run(
            std::time::Duration::from_secs(5),
            |log: &RefCell<Vec<u32>>| log.borrow().clone(),
        ))?;
        assert_eq!(seen, vec![0, 1, 2, 3, 4, 5]);
        Ok(())
    }

    #[test]
    fn timeout_abandons_the_result_and_leaves_the_worker_usable()
    -> Result<(), Box<dyn std::error::Error>> {
        let runtime = runtime()?;
        let (worker, ()) = Worker::spawn("test-timeout", || (Cell::new(0_u32), ()))?;

        let timed_out = runtime.block_on(worker.run(
            std::time::Duration::from_millis(20),
            |counter: &Cell<u32>| {
                std::thread::sleep(std::time::Duration::from_millis(200));
                counter.set(7);
            },
        ));
        assert_eq!(timed_out, Err(WorkerError::TimedOut));

        // The abandoned job still owns nothing of the caller's, so it completes safely and the
        // next job observes its effect on a resource the worker never stopped owning.
        let observed = runtime.block_on(
            worker.run(std::time::Duration::from_secs(5), |counter: &Cell<u32>| {
                counter.get()
            }),
        )?;
        assert_eq!(observed, 7);
        Ok(())
    }

    #[test]
    fn spawn_reports_a_worker_that_never_started() {
        let failed: Result<(Worker<()>, ()), WorkerError> =
            Worker::spawn("test-dead", || panic!("resource construction failed"));
        assert_eq!(failed.err(), Some(WorkerError::Stopped));
    }
}
