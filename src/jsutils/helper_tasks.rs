use futures::Future;
use hirofa_utils::task_manager::TaskManager;
use std::sync::OnceLock;
use tokio::runtime::Handle;
use tokio::task::JoinError;

static HELPER_TASKS: OnceLock<TaskManager> = OnceLock::new();

fn get_helper_tasks() -> &'static TaskManager {
    HELPER_TASKS.get_or_init(|| TaskManager::new(std::cmp::max(2, num_cpus::get())))
}

/// initialize the helper tasks with a specific tokio handle
pub fn init_helper_tasks_with_handle(handle: Handle) {
    let _ = HELPER_TASKS.set(TaskManager::from_handle(handle));
}

/// initialize the helper tasks with a specific thread count
pub fn init_helper_tasks(thread_count: usize) {
    let _ = HELPER_TASKS.set(TaskManager::new(thread_count));
}

/// add a task the the "helper" thread pool
pub fn add_helper_task<T>(task: T)
where
    T: FnOnce() + Send + 'static,
{
    log::trace!("adding a helper task");
    get_helper_tasks().add_task(task);
}

/// add an async task the the "helper" thread pool
pub fn add_helper_task_async<R: Send + 'static, T: Future<Output = R> + Send + 'static>(
    task: T,
) -> impl Future<Output = Result<R, JoinError>> {
    log::trace!("adding an async helper task");
    get_helper_tasks().add_task_async(task)
}
