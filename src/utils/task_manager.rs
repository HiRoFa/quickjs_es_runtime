use futures::Future;
use log::trace;
use tokio::runtime::Runtime;
use tokio::task::JoinError;

pub struct TaskManager {
    runtime: Runtime,
}

impl TaskManager {
    pub fn new(thread_count: usize) -> Self {
        // start threads

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .max_blocking_threads(thread_count)
            .build()
            .expect("tokio rt failed");

        TaskManager { runtime }
    }

    pub fn add_task<T: FnOnce() + Send + 'static>(&self, task: T) {
        trace!("adding a task");
        self.runtime.spawn_blocking(task);
    }

    /// start an async task
    /// # Example
    /// ```rust
    /// use quickjs_runtime::utils::task_manager::TaskManager;
    /// let tm = TaskManager::new(2);
    /// let task = async {
    ///     println!("foo");
    /// };
    /// tm.add_task_async(task);
    /// ```
    pub fn add_task_async<R: Send + 'static, T: Future<Output = R> + Send + 'static>(
        &self,
        task: T,
    ) -> impl Future<Output = Result<R, JoinError>> {
        self.runtime.spawn(task)
    }

    #[allow(dead_code)]
    pub fn run_task_blocking<R: Send + 'static, T: FnOnce() -> R + Send + 'static>(
        &self,
        task: T,
    ) -> R {
        trace!("adding a sync task from thread {}", thread_id::get());
        // check if the current thread is not a worker thread, because that would be bad
        let join_handle = self.runtime.spawn_blocking(task);
        self.runtime.block_on(join_handle).expect("task failed")
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::task_manager::TaskManager;
    use log::trace;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test() {
        trace!("testing");

        let tm = TaskManager::new(1);
        for _x in 0..5 {
            tm.add_task(|| {
                thread::sleep(Duration::from_secs(1));
            })
        }

        let s = tm.run_task_blocking(|| {
            thread::sleep(Duration::from_secs(1));
            "res"
        });

        assert_eq!(s, "res");

        for _x in 0..10 {
            let s = tm.run_task_blocking(|| "res");

            assert_eq!(s, "res");
        }
    }
}
