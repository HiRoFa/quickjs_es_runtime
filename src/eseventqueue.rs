use crate::debugmutex::DebugMutex;
use log::trace;
use std::cell::RefCell;
use std::mem::replace;
use std::sync::mpsc::channel;
use std::sync::{Arc, Condvar};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

type LocalJob = dyn FnOnce() + 'static;

thread_local!(
    pub static LOCAL_JOBS: RefCell<Vec<Box<LocalJob>>> = RefCell::new(vec![]);
);
///
/// the EsEventQueue is a single threaded thread pool which is used to act as the only thread
/// using an instance of the spidermonkey runtime
/// besides being able to add tasks from any thread by add_task
/// a running task can add jobs to the current thread by calling add_task_from_worker
/// those tasks need not impl the Send trait and there is no locking happening to add
/// the task to the queue
pub struct EsEventQueue {
    jobs: DebugMutex<Vec<Box<dyn FnOnce() + Send + 'static>>>,
    empty_cond: Condvar,
    worker_thread_name: String,
}

impl EsEventQueue {
    pub fn new() -> Arc<Self> {
        let uuid = format!("eseq_wt_{}", Uuid::new_v4());
        let task_manager = EsEventQueue {
            jobs: DebugMutex::new(vec![], "EsEventQueue::jobs"),
            empty_cond: Condvar::new(),
            worker_thread_name: uuid.clone(),
        };
        let rc = Arc::new(task_manager);

        let wrc = Arc::downgrade(&rc);

        thread::Builder::new()
            .name(uuid)
            .spawn(move || loop {
                let rcc = wrc.upgrade();
                if let Some(rc) = rcc {
                    rc.worker_loop();
                } else {
                    trace!("Arc to EsEventQueue was dropped, stopping worker thread");
                    break;
                }
            })
            .unwrap();

        rc
    }

    /// add a task which will run asynchronously
    pub fn add_task<T: FnOnce() + Send + 'static>(&self, task: T) {
        trace!("EsEventQueue::add_task");
        {
            let mut lck = self.jobs.lock("add_task").unwrap();
            let jobs = &mut *lck;
            jobs.push(Box::new(task));
        }
        trace!("EsEventQueue::add_task / notify");
        self.empty_cond.notify_all();
    }

    /// execute a task synchronously in the worker thread
    pub fn exe_task<R: Send + 'static, T: FnOnce() -> R + Send + 'static>(&self, task: T) -> R {
        trace!("EsEventQueue::exe_task");

        if self.is_worker_thread() {
            // don;t block from worker threads
            trace!("EsEventQueue::exe_task, is worker, just run");
            return task();
        }

        // create a channel, put sender in job, wait for receiver here

        trace!("EsEventQueue::exe_task / create channel");

        let (sender, receiver) = channel();

        let job = move || {
            trace!("EsEventQueue::exe_task / job");
            let res: R = task();
            trace!("EsEventQueue::exe_task / send");
            sender.send(res).unwrap();
        };
        self.add_task(job);
        trace!("EsEventQueue::exe_task / receive");
        let res = receiver.recv();
        trace!("EsEventQueue::exe_task / received");
        match res {
            Ok(ret) => ret,
            Err(e) => {
                panic!("task failed: {}", e);
            }
        }
    }

    /// method for adding tasks from worker, these do not need to impl Send
    /// also there is no locks we need to wait for
    #[allow(dead_code)]
    pub fn add_task_from_worker<T: FnOnce() + 'static>(&self, task: T) {
        // assert current thread is worker thread
        // add to a thread_local st_tasks list
        // plan a job to run a single task from that list
        // this way the list does not need to be locked
        self.assert_is_worker_thread();

        LOCAL_JOBS.with(move |rc| {
            let vec = &mut *rc.borrow_mut();
            vec.push(Box::new(task));
        });
        self.empty_cond.notify_all();
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        !self.has_local_jobs() && !self.has_jobs()
    }

    #[allow(dead_code)]
    fn has_jobs(&self) -> bool {
        let jobs_lck = self.jobs.lock("has_jobs").unwrap();
        !jobs_lck.is_empty()
    }

    fn has_local_jobs(&self) -> bool {
        LOCAL_JOBS.with(|rc| {
            let local_jobs = &*rc.borrow();
            !local_jobs.is_empty()
        })
    }

    #[allow(dead_code)]
    pub fn looks_like_eventqueue_thread() -> bool {
        let handle = thread::current();
        if let Some(handle_name) = handle.name() {
            handle_name.starts_with("eseq_wt_")
        } else {
            false
        }
    }

    pub fn is_worker_thread(&self) -> bool {
        let handle = thread::current();
        if let Some(handle_name) = handle.name() {
            self.worker_thread_name.as_str().eq(handle_name)
        } else {
            false
        }
    }

    pub fn assert_is_worker_thread(&self) {
        let handle = thread::current();
        assert_eq!(handle.name(), Some(self.worker_thread_name.as_str()));
    }

    fn worker_loop(&self) {
        let jobs: Vec<Box<dyn FnOnce() + Send + 'static>>;
        {
            let mut jobs_lck = self.jobs.lock("worker_loop").unwrap();

            if jobs_lck.is_empty() && !self.has_local_jobs() {
                let dur = Duration::from_secs(5);
                jobs_lck = self.empty_cond.wait_timeout(jobs_lck, dur).ok().unwrap().0;
            }

            jobs = replace(&mut *jobs_lck, vec![]);
        }

        for job in jobs {
            job();
        }

        LOCAL_JOBS.with(|rc| {
            let mut local_todos = vec![];
            {
                let local_jobs = &mut *rc.borrow_mut();
                while !local_jobs.is_empty() {
                    let local_job = local_jobs.remove(0);
                    local_todos.push(local_job);
                }
            }
            for local_todo in local_todos {
                local_todo();
            }
        });
    }
}

impl Drop for EsEventQueue {
    fn drop(&mut self) {
        trace!("drop EsEventQueue");
    }
}

#[cfg(test)]
mod tests {
    use crate::eseventqueue::EsEventQueue;
    use log::debug;

    use std::thread;
    use std::time::Duration;
    #[test]
    fn t() {
        thread::spawn(|| {
            t1();
        })
        .join()
        .ok()
        .unwrap();
    }

    fn t1() {
        {
            let sttm = EsEventQueue::new();

            let sttm2 = sttm.clone();
            let sttm3 = sttm.clone();
            let sttm4 = sttm.clone();

            let j = thread::spawn(move || {
                debug!("add t1 to EsEventQueue");
                sttm3.add_task(|| {
                    debug!("t1");
                });
                debug!("add t2 to EsEventQueue4");
                sttm4.add_task(|| {
                    debug!("t2");
                });
                debug!("dropping EsEventQueue3 and 4");
            });

            sttm.add_task(|| {
                debug!("sp something");
            });
            sttm.add_task(|| {
                debug!("sp something");
            });
            sttm.add_task(move || {
                debug!("sp something");
                sttm2.add_task_from_worker(|| {
                    debug!("sp something from worker");
                });
                sttm2.add_task_from_worker(|| {
                    debug!("sp something from worker");
                });
            });
            sttm.add_task(|| {
                debug!("sp something");
            });

            let a = sttm.exe_task(|| 12);
            assert_eq!(a, 12);

            std::thread::sleep(Duration::from_secs(2));
            j.join().ok().unwrap();
            debug!("done");
        }
        debug!("EsEventQueue should drop now");
        thread::sleep(Duration::from_secs(1));
        debug!("EsEventQueue should be dropped now");
    }
}
