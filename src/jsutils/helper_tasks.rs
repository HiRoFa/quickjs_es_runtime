use hirofa_utils::task_manager::TaskManager;
use futures::Future;
use lazy_static::lazy_static;
use tokio::task::JoinError;

lazy_static! {
    /// a static Multithreaded task manager used to run rust ops async and multithreaded ( in at least 2 threads)
    static ref HELPER_TASKS: TaskManager = TaskManager::new(std::cmp::max(2, num_cpus::get()));
}

/// add a task the the "helper" thread pool
pub fn add_helper_task<T>(task: T)
    where
        T: FnOnce() + Send + 'static,
{
    log::trace!("adding a helper task");
    HELPER_TASKS.add_task(task);
}

/// add an async task the the "helper" thread pool
pub fn add_helper_task_async<R: Send + 'static, T: Future<Output = R> + Send + 'static>(
    task: T,
) -> impl Future<Output = Result<R, JoinError>> {
    log::trace!("adding an async helper task");
    HELPER_TASKS.add_task_async(task)
}