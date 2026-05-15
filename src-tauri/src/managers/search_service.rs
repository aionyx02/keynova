use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

struct WorkItem {
    task: Box<dyn FnOnce() + Send + 'static>,
    cancel: Arc<AtomicBool>,
}

enum Slot {
    Empty,
    Pending(WorkItem),
}

/// Single-worker coordinator for background file-search execution.
///
/// Guarantees: at most one task runs at a time; at most one task is pending.
/// Submitting a new task atomically cancels the previous pending or running
/// task by setting its cancel token to `true` before the new task is slotted.
pub(crate) struct SearchService {
    slot: Mutex<Slot>,
    condvar: Condvar,
    active_cancel: Mutex<Arc<AtomicBool>>,
}

impl SearchService {
    pub fn new() -> Arc<Self> {
        let svc = Arc::new(Self {
            slot: Mutex::new(Slot::Empty),
            condvar: Condvar::new(),
            active_cancel: Mutex::new(Arc::new(AtomicBool::new(false))),
        });
        let worker = Arc::clone(&svc);
        thread::Builder::new()
            .name("keynova-search-worker".into())
            .spawn(move || Self::worker_loop(worker))
            .expect("search worker thread failed to start");
        svc
    }

    fn worker_loop(svc: Arc<Self>) {
        loop {
            let item = {
                let mut guard = svc.slot.lock().unwrap();
                loop {
                    match std::mem::replace(&mut *guard, Slot::Empty) {
                        Slot::Pending(item) => break item,
                        Slot::Empty => {
                            guard = svc.condvar.wait(guard).unwrap();
                        }
                    }
                }
            };
            if !item.cancel.load(Ordering::Relaxed) {
                (item.task)();
            }
        }
    }

    /// Submit a task. Cancels any previously pending or running task by
    /// setting its cancel token to `true`.
    ///
    /// The `cancel` token passed here is the same one the task should poll
    /// internally for cooperative early exit. It will be set to `true` when
    /// the next task is submitted.
    pub fn submit(&self, cancel: Arc<AtomicBool>, task: impl FnOnce() + Send + 'static) {
        {
            let mut active = self.active_cancel.lock().unwrap();
            active.store(true, Ordering::Relaxed);
            *active = Arc::clone(&cancel);
        }
        {
            let mut guard = self.slot.lock().unwrap();
            if let Slot::Pending(old) = &*guard {
                old.cancel.store(true, Ordering::Relaxed);
            }
            *guard = Slot::Pending(WorkItem { task: Box::new(task), cancel });
        }
        self.condvar.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

    #[test]
    fn submit_sets_cancel_on_previous_task() {
        let svc = SearchService::new();
        let cancel_a = Arc::new(AtomicBool::new(false));
        svc.submit(Arc::clone(&cancel_a), || {});
        let cancel_b = Arc::new(AtomicBool::new(false));
        svc.submit(Arc::clone(&cancel_b), || {});
        assert!(cancel_a.load(Ordering::Relaxed), "A should be cancelled when B is submitted");
    }

    #[test]
    fn submit_chain_cancels_all_previous() {
        let svc = SearchService::new();
        let cancel_a = Arc::new(AtomicBool::new(false));
        svc.submit(Arc::clone(&cancel_a), || {});
        let cancel_b = Arc::new(AtomicBool::new(false));
        svc.submit(Arc::clone(&cancel_b), || {});
        let cancel_c = Arc::new(AtomicBool::new(false));
        svc.submit(Arc::clone(&cancel_c), || {});
        assert!(cancel_a.load(Ordering::Relaxed));
        assert!(cancel_b.load(Ordering::Relaxed));
        assert!(!cancel_c.load(Ordering::Relaxed), "C is active, must not be cancelled yet");
    }

    #[test]
    fn worker_runs_latest_slotted_task() {
        let svc = SearchService::new();
        let ran = Arc::new(AtomicU64::new(0));

        // Block the worker so we can fill the slot with multiple tasks.
        let (unblock_tx, unblock_rx) = std::sync::mpsc::channel::<()>();
        svc.submit(Arc::new(AtomicBool::new(false)), move || {
            let _ = unblock_rx.recv_timeout(Duration::from_millis(500));
        });

        // Submit three tasks while worker is busy — only the last should run.
        for _ in 0_u64..2 {
            svc.submit(Arc::new(AtomicBool::new(false)), move || {});
        }
        let ran_clone = Arc::clone(&ran);
        svc.submit(Arc::new(AtomicBool::new(false)), move || {
            ran_clone.fetch_add(1, Ordering::Relaxed);
        });

        // Unblock the worker and wait for it to finish.
        let _ = unblock_tx.send(());
        thread::sleep(Duration::from_millis(200));

        // Exactly the last submitted task must have run.
        assert_eq!(ran.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn cancelled_task_is_skipped() {
        let svc = SearchService::new();
        let ran = Arc::new(AtomicBool::new(false));

        let cancel = Arc::new(AtomicBool::new(false));
        let ran_clone = Arc::clone(&ran);
        svc.submit(Arc::clone(&cancel), move || {
            ran_clone.store(true, Ordering::Relaxed);
        });

        // Cancel the task before the worker picks it up.
        cancel.store(true, Ordering::Relaxed);

        thread::sleep(Duration::from_millis(100));
        assert!(!ran.load(Ordering::Relaxed), "task with cancel=true must not run");
    }
}