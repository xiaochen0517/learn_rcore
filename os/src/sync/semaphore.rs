use crate::sync::UPSafeCell;
use crate::task::{TaskControlBlock, block_current_and_run_next, current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc};
use log::debug;

pub struct Semaphore {
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    pub fn new(res_count: usize) -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    pub fn up(&self) {
        let mut inner = self.inner.exclusive_access();
        // 信号量 +1
        inner.count += 1;
        debug!("semaphore up count: {}", inner.count);
        // 如果当前已有的信号量小于等于零，说明在这之前已经有线程进入到了等待状态。
        // 需要获取到一个等待队列中的线程并唤醒。
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                wakeup_task(task);
            }
        }
    }

    pub fn down(&self) {
        let mut inner = self.inner.exclusive_access();
        // 信号量 -1
        inner.count -= 1;
        debug!("semaphore down count: {}", inner.count);
        // 在进行信号量 -1 操作时，有可能信号量已经是 0 。
        // 如果操作后信号量小于一，则需要将当前线程添加到等待队列中，并阻塞当前线程。
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        }
    }
}
