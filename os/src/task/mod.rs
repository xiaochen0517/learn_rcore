mod context;
mod switch;

#[allow(clippy::module_inception)]
mod task;

use crate::config::MAX_APP_NUM;
use crate::loader::{get_num_app, init_app_cx};
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

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
    tasks: [TaskControlBlock; MAX_APP_NUM],
    /// 当前运行的任务索引
    current_task: usize,
}

lazy_static! {
    /// Global variable: TASK_MANAGER
    pub static ref TASK_MANAGER: TaskManager = {
        // 获取应用数量
        let num_app = get_num_app();
        // 初始化任务列表
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];
        // 初始化每个任务的上下文
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }
        info!("TaskManager created with {} tasks.", num_app);
        // 创建任务管理器
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
        // 获取任务管理器的可变引用
        let mut inner = self.inner.exclusive_access();
        // 获取到第一个任务，并将任务状态设置为 `Running`
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        // 获取当前第一个任务的上下文指针
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        // 释放对任务管理器的可变引用
        drop(inner);
        // 对当前的任务上下文进行初始化
        let mut _unused = TaskContext::zero_init();
        info!("Running first task with context");
        // 需要对上下文进行切换，当前任务使用一个空任务，接下来要执行的任务使用第一个任务的上下文。
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
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
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
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

    /// 运行下一个任务
    ///
    /// 如果没有下一个任务，则打印信息并关闭系统。
    fn run_next_task(&self) {
        // 查找下一个任务，有可能当前任务已经是最后一个任务了
        if let Some(next) = self.find_next_task() {
            // 获取任务管理器的可变引用
            let mut inner = self.inner.exclusive_access();
            // 获取到当前任务的索引
            let current = inner.current_task;
            // 将下一个任务的状态设置为 `Running`，并更新当前执行任务索引为下一个任务的索引
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            // 获取到当前任务和下一个任务的上下文指针
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // 切换上下文，从当前任务切换到下一个任务
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // 返回到用户模式
        } else {
            println!("All applications completed!");
            shutdown(false);
        }
    }
}

/// run first task
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// rust next task
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// suspend current task
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// exit current task
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// suspend current task, then run next task
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// exit current task,  then run next task
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}
