use tokio::task::JoinError;

/// Runs synchronous filesystem and other blocking work on Tokio's blocking thread pool.
#[derive(Clone, Debug, Default)]
pub struct BlockingIoPool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockingIoError;

impl std::fmt::Display for BlockingIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "blocking I/O task failed")
    }
}

impl std::error::Error for BlockingIoError {}

impl BlockingIoPool {
    pub fn new() -> Self {
        Self
    }

    pub async fn run<F, T>(&self, f: F) -> Result<T, BlockingIoError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        tokio::task::spawn_blocking(f).await.map_err(join_error)
    }
}

fn join_error(_: JoinError) -> BlockingIoError {
    BlockingIoError
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::BlockingIoPool;

    #[tokio::test]
    async fn unrelated_async_work_is_not_stalled_by_blocking_io()
    -> Result<(), Box<dyn std::error::Error>> {
        let pool = BlockingIoPool::new();
        let pool_for_blocking = pool.clone();

        let blocking = tokio::spawn(async move {
            pool_for_blocking
                .run(|| {
                    std::thread::sleep(Duration::from_millis(400));
                })
                .await
        });

        let start = Instant::now();
        let responsive = tokio::time::timeout(Duration::from_millis(50), async {
            tokio::task::yield_now().await;
        })
        .await
        .is_ok();
        if !responsive {
            return Err(std::io::Error::other(
                "async scheduler should remain responsive while blocking I/O runs",
            )
            .into());
        }
        assert!(
            start.elapsed() < Duration::from_millis(200),
            "async worker appeared stalled by blocking filesystem work"
        );

        match blocking.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(std::io::Error::other("blocking I/O pool run failed").into()),
            Err(join_error) => Err(Box::new(join_error) as Box<dyn std::error::Error>),
        }
    }
}
