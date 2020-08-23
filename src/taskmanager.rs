use log::trace;

use rayon::{ThreadPool, ThreadPoolBuilder};

pub struct TaskManager {
    thread_pool: ThreadPool,
}

impl TaskManager {
    #[allow(dead_code)]
    pub fn new(thread_count: usize) -> Self {
        // start threads

        let thread_pool = ThreadPoolBuilder::new()
            .thread_name(|id| format!("TaskManager_thread_{}", id))
            .num_threads(thread_count)
            .build()
            .unwrap();

        TaskManager { thread_pool }
    }

    #[allow(dead_code)]
    pub fn add_task<T: FnOnce() + Send + 'static>(&self, task: T) {
        trace!("adding a task");

        self.thread_pool.spawn(task);
    }

    #[allow(dead_code)]
    pub fn run_task_blocking<R: Send, T: FnOnce() -> R + Send>(&self, task: T) -> R {
        trace!("adding a sync task from thread {}", thread_id::get());
        // check if the current thread is not a worker thread, because that would be bad
        assert!(self.thread_pool.current_thread_index().is_none());
        self.thread_pool.install(task)
    }
}

#[cfg(test)]
mod tests {
    use crate::taskmanager::TaskManager;
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

        for _x in 0..1000 {
            let s = tm.run_task_blocking(|| "res");

            assert_eq!(s, "res");
        }
    }
}
