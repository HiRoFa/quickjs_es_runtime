use crate::utils::auto_id_map::AutoIdMap;
use crate::utils::debug_mutex::DebugMutex;
use futures::task::Waker;
use log::trace;
use std::cell::RefCell;
use std::future::Future;
use std::mem::replace;
use std::ops::Add;
use std::pin::Pin;
use std::sync::mpsc::{channel, Receiver, SendError, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use uuid::Uuid;

type LocalJob = dyn FnOnce() + 'static;
type Job = dyn Fn() + 'static;

struct ScheduledJob {
    job: Box<Job>,
    interval: Option<Duration>,
    next_run: Instant,
}

thread_local!(
    static LOCAL_JOBS: RefCell<Vec<Box<LocalJob>>> = RefCell::new(vec![]);
    static SCHEDULED_LOCAL_JOBS: RefCell<AutoIdMap<ScheduledJob>> =
        RefCell::new(AutoIdMap::new_with_max_size(i32::max_value() as usize));
);

///
/// the EsEventQueue is a single threaded thread pool which is used to act as the only thread
/// using an instance of a script runtime
/// besides being able to add tasks from any thread by add_task
/// a running task can add jobs to the current thread by calling add_task_from_worker
/// those tasks need not impl the Send trait and there is no locking happening to add
/// the task to the queue
pub struct SingleThreadedEventQueue {
    jobs: DebugMutex<Vec<Box<dyn FnOnce() + Send + 'static>>>,
    empty_cond: Condvar,
    worker_thread_name: String,
    shutdown_switch: Mutex<Sender<bool>>,
    join_handle: Mutex<Option<JoinHandle<()>>>,
}

pub struct TaskFutureResolver<R> {
    sender: Mutex<Sender<R>>,
    waker: Mutex<Option<Waker>>,
}

impl<R> TaskFutureResolver<R> {
    pub fn new(tx: Sender<R>) -> Self {
        Self {
            sender: Mutex::new(tx),
            waker: Mutex::new(None),
        }
    }
    pub fn resolve(&self, resolution: R) -> Result<(), SendError<R>> {
        log::trace!("TaskFutureResolver.resolve");
        let lck = self.sender.lock().unwrap();
        let sender = &*lck;
        sender.send(resolution)?;
        drop(lck);

        let waker_opt = &mut *self.waker.lock().unwrap();
        if let Some(waker) = waker_opt.take() {
            waker.wake();
        }
        Ok(())
    }
}

pub struct TaskFuture<R> {
    result: Receiver<R>,
    resolver: Arc<TaskFutureResolver<R>>,
}
impl<R> TaskFuture<R> {
    pub fn new() -> Self {
        let (tx, rx) = channel();

        Self {
            result: rx,
            resolver: Arc::new(TaskFutureResolver {
                sender: Mutex::new(tx),
                waker: Mutex::new(None),
            }),
        }
    }
    pub fn get_resolver(&self) -> Arc<TaskFutureResolver<R>> {
        self.resolver.clone()
    }
}
impl<R> Default for TaskFuture<R> {
    fn default() -> Self {
        Self::new()
    }
}
impl<R> Future for TaskFuture<R> {
    type Output = R;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        log::trace!("TaskFuture::poll");
        match self.result.try_recv() {
            Ok(res) => {
                log::trace!("TaskFuture::poll -> Ready");
                Poll::Ready(res)
            }
            Err(_) => {
                log::trace!("TaskFuture::poll -> Pending");
                let mtx = &self.resolver.waker;
                let waker_opt = &mut *mtx.lock().unwrap();
                let _ = waker_opt.replace(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl SingleThreadedEventQueue {
    pub fn new() -> Arc<Self> {
        let uuid = format!("eseq_wt_{}", Uuid::new_v4());

        let (shutdown_switch, shutdown_receiver) = channel();

        let task_manager = SingleThreadedEventQueue {
            jobs: DebugMutex::new(vec![], "EsEventQueue::jobs"),
            empty_cond: Condvar::new(),
            worker_thread_name: uuid.clone(),
            shutdown_switch: Mutex::new(shutdown_switch),
            join_handle: Mutex::new(None),
        };
        let ret = Arc::new(task_manager);
        let arc = ret.clone();

        let join_handle = thread::Builder::new()
            .name(uuid)
            .spawn(move || loop {
                arc.worker_loop();
                if shutdown_receiver.try_recv().is_ok() {
                    trace!("Shutdown was called, stopping worker thread");
                    arc.shutdown_cleanup();
                    break;
                }
            })
            .unwrap();

        ret.join_handle.lock().unwrap().replace(join_handle);

        ret
    }

    pub fn shutdown(&self) {
        // todo use a channel
        self.shutdown_switch
            .lock()
            .unwrap()
            .send(true)
            .expect("could not send shutdown");
        // wake up
        self.add_task(|| {});
        let jh_opt = &mut *self.join_handle.lock().unwrap();
        let jh = jh_opt.take().expect("no join handle set");
        jh.join().expect("join failed");
    }

    fn shutdown_cleanup(&self) {
        LOCAL_JOBS.with(|rc| {
            let lj = &mut *rc.borrow_mut();
            lj.clear();
        });
        SCHEDULED_LOCAL_JOBS.with(|rc| {
            let slj = &mut *rc.borrow_mut();
            slj.clear();
        })
    }

    /// add a task which will run asynchronously
    pub fn add_task<T: FnOnce() + Send + 'static>(&self, task: T) {
        // todo block on max size
        trace!("EsEventQueue::add_task");
        {
            let mut lck = self.jobs.lock("add_task").unwrap();
            let jobs = &mut *lck;
            jobs.push(Box::new(task));
        }
        trace!("EsEventQueue::add_task / notify");
        self.empty_cond.notify_all();
    }

    pub fn async_task<T: FnOnce() -> R + Send + 'static, R: Send + 'static>(
        &self,
        task: T,
    ) -> impl Future<Output = R> {
        trace!("EsEventQueue::async_task");
        let fut = TaskFuture::new();
        let tx = fut.get_resolver();
        self.add_task(move || {
            let res = task();
            match tx.resolve(res) {
                Ok(_) => {}
                Err(e) => {
                    log::error!(
                        "resolving TaskFuture failed, TaskFuture was probably dropped: {}",
                        e
                    );
                }
            }
        });
        fut
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

    pub fn schedule_task_from_worker<T: Fn() + 'static>(
        &self,
        task: T,
        interval: Option<Duration>,
        delay: Duration,
    ) -> i32 {
        trace!(
            "SingleThreadedEventQueue.schedule_task_from_worker interval:{:?} delay:{:?}",
            interval,
            delay
        );

        self.assert_is_worker_thread();

        let task = ScheduledJob {
            job: Box::new(task),
            interval,
            next_run: Instant::now().add(delay),
        };

        // return the id
        SCHEDULED_LOCAL_JOBS.with(|rc| {
            let jobs = &mut *rc.borrow_mut();
            jobs.insert(task) as i32
        })
    }

    pub fn remove_schedule_task_from_worker(&self, id: i32) {
        self.assert_is_worker_thread();
        SCHEDULED_LOCAL_JOBS.with(|rc| {
            let jobs = &mut *rc.borrow_mut();
            let id = &(id as usize);
            if jobs.contains_key(id) {
                jobs.remove(id);
            }
        });
    }

    pub fn todo_count(&self) -> usize {
        let jobs_lck = self.jobs.lock("todo_count").unwrap();
        jobs_lck.len()
    }

    fn has_local_jobs(&self) -> bool {
        LOCAL_JOBS.with(|rc| {
            let local_jobs = &*rc.borrow();
            !local_jobs.is_empty()
        })
    }

    fn is_worker_thread(&self) -> bool {
        let handle = thread::current();
        if let Some(handle_name) = handle.name() {
            self.worker_thread_name.as_str().eq(handle_name)
        } else {
            false
        }
    }

    pub fn assert_is_worker_thread(&self) {
        debug_assert_eq!(
            thread::current().name(),
            Some(self.worker_thread_name.as_str())
        );
    }

    fn worker_loop(&self) {
        let wait_dur: Duration = run_sched_jobs();

        let jobs: Vec<Box<dyn FnOnce() + Send + 'static>>;
        {
            let mut jobs_lck = self.jobs.lock("worker_loop").unwrap();

            if jobs_lck.is_empty() && !self.has_local_jobs() {
                jobs_lck = self
                    .empty_cond
                    .wait_timeout(jobs_lck, wait_dur)
                    .ok()
                    .unwrap()
                    .0;
            }

            jobs = replace(&mut *jobs_lck, vec![]);
        }

        for job in jobs {
            job();
        }

        run_local_jobs();
    }
}

fn run_sched_jobs() -> Duration {
    // NB prevent double borrow mut, so first get removable jobs
    let now = Instant::now();
    SCHEDULED_LOCAL_JOBS.with(|rc| {
        let mut wait_dur = Duration::from_millis(10000);
        {
            // this block is so we don;t a a mutable borrow while running a job, a job might add another job and then there might already be a mutable borrow. which would be bad

            let removable_jobs;
            {
                let jobs = &mut *rc.borrow_mut();
                removable_jobs =
                    jobs.remove_values(|job| job.next_run.lt(&now) && job.interval.is_none());
            }

            // run those
            for job in &removable_jobs {
                let j = &job.job;
                j();
            }

            trace!("SingleThreadedEventQueue.run_sched_jobs done");

            // update re-scheds
            // haha this effing sucks, i need descent iter/map/collect code in AutoIdMap
            // also figure out a way to dynamically get wait delay for empty condition
            let re_sched_ids: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(vec![]));
            {
                let jobs = &*rc.borrow();
                jobs.foreach(|k, v| {
                    if v.next_run.lt(&now) && v.interval.is_some() {
                        re_sched_ids.lock().unwrap().push(*k);
                        let j = &v.job;
                        j();
                    }
                });
            }

            // re sched jobs
            {
                let jobs = &mut *rc.borrow_mut();
                for k in &*re_sched_ids.lock().unwrap() {
                    let job = jobs.get_mut(k).unwrap();
                    job.next_run = now.add(job.interval.unwrap());
                }

                for job in jobs.map.values() {
                    let wait_opt = job.next_run.duration_since(now);
                    if wait_opt.lt(&wait_dur) {
                        wait_dur = wait_opt;
                    }
                }
            }
        }
        wait_dur
    })
}

fn run_local_jobs() {
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

impl Drop for SingleThreadedEventQueue {
    fn drop(&mut self) {
        trace!("drop EsEventQueue");
    }
}

#[cfg(test)]
mod tests {

    use crate::utils::single_threaded_event_queue::SingleThreadedEventQueue;
    use futures::executor::block_on;
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
            let sttm = SingleThreadedEventQueue::new();

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
            sttm.shutdown();
        }
        debug!("EsEventQueue should drop now");
        thread::sleep(Duration::from_secs(1));
        debug!("EsEventQueue should be dropped now");
    }

    #[test]
    fn t2() {
        //simple_logging::log_to_stderr(LevelFilter::Trace);

        let sttm = SingleThreadedEventQueue::new();
        let sttm2 = sttm.clone();
        sttm.add_task(move || {
            sttm2.schedule_task_from_worker(
                || {
                    log::info!("st 1 > after 1 sec");
                },
                None,
                Duration::from_millis(1000),
            );
            sttm2.schedule_task_from_worker(
                || {
                    log::info!("st 2 > after 2 secs");
                },
                None,
                Duration::from_millis(2000),
            );
            sttm2.schedule_task_from_worker(
                || {
                    log::info!("st 3 > after 7 secs");
                },
                None,
                Duration::from_secs(7),
            );
            sttm2.schedule_task_from_worker(
                || {
                    log::info!("int 1 > after 2 secs, every 2 secs");
                },
                Some(Duration::from_secs(2)),
                Duration::from_secs(2),
            );
            sttm2.schedule_task_from_worker(
                || {
                    log::info!("int 2 > after 2 secs, every 5 secs");
                },
                Some(Duration::from_secs(5)),
                Duration::from_secs(2),
            );
        });

        std::thread::sleep(Duration::from_secs(13));
        sttm.shutdown();
    }

    async fn test_async1(sttm: &SingleThreadedEventQueue) {
        let f1 = sttm.async_task(|| {
            thread::sleep(Duration::from_secs(1));
            "blurb1"
        });
        let f2 = sttm.async_task(|| {
            thread::sleep(Duration::from_secs(1));
            "blurb2"
        });

        let two = f2.await;
        let one = f1.await;

        assert_eq!(one, "blurb1");
        assert_eq!(two, "blurb2");
    }

    #[test]
    fn test_async() {
        let sttm = SingleThreadedEventQueue::new();
        let fut = test_async1(&sttm);
        block_on(fut);
        sttm.shutdown();
    }
}
