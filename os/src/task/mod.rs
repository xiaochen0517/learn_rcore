mod context;
mod switch;

#[allow(clippy::module_inception)]
mod task;

use crate::loader::{get_app_data, get_num_app};
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

use crate::timer::get_time_ms;
use crate::trap::TrapContext;
pub use context::TaskContext;
use log::info;

/// 任务管理器
pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

/// 任务管理器单例
struct TaskManagerInner {
    /// 任务列表
    tasks: Vec<TaskControlBlock>,
    /// 当前运行的任务索引
    current_task: usize,
}

lazy_static! {
    /// Global variable: TASK_MANAGER
    pub static ref TASK_MANAGER: TaskManager = {
        info!("[kernel] init TASK_MANAGER");
        let num_app = get_num_app();
        info!("[kernel] num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        info!("[kernel] TaskControlBlock created.");
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// 运行任务列表中的第一个任务。
    fn run_first_task(&self) -> ! {
        info!("[kernel] Running first task...");
        let mut inner = self.inner.exclusive_access();
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        next_task.task_start_time = get_time_ms();
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        info!("[kernel] Running first task: {:?}", next_task_cx_ptr);
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// 将当前 `Running` 任务的状态标记为 `Ready`，即准备就绪。
    fn mark_current_suspended(&self) {
        // 获取任务管理器的可变引用
        let mut inner = self.inner.exclusive_access();
        // 获取当前任务的索引
        let current = inner.current_task;
        // 将当前任务的状态设置为 `Ready`
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    /// 将当前 `Running` 任务的状态标记为 `Exited`，即已退出。
    fn mark_current_exited(&self) {
        // 获取任务管理器的可变引用
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let current_task = &mut inner.tasks[current];
        // 将当前任务的状态设置为 `Exited`，并记录任务结束时间
        current_task.task_status = TaskStatus::Exited;
        current_task.task_end_time = get_time_ms();
        // 计算任务的持续时间，并打印相关信息
        let duration = current_task
            .task_end_time
            .saturating_sub(current_task.task_start_time);
        if current_task.task_end_time < current_task.task_start_time {
            info!("[kernel] Task {} exited with negative duration!", current);
        }
        info!(
            "[kernel] Task {} exited, duration: {} ms",
            current, duration
        );
    }

    /// 发现下一个 `Ready` 任务
    ///
    /// 如果没有找到 `Ready` 任务，则返回 `None`。
    fn find_next_task(&self) -> Option<usize> {
        // 获取到任务管理器
        let inner = self.inner.exclusive_access();
        // 获取当前任务
        let current = inner.current_task;
        // 从当前任务的下一个开始，循环查找下一个 `Ready` 任务
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.tasks[next].task_start_time = get_time_ms();
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            println!("All applications completed!");
            shutdown(false);
        }
    }
}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Get the current 'Running' task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// Get the current 'Running' task's trap contexts.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// Change the current 'Running' task's program break
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}
