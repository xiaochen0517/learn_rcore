use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};

/// 任务退出
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// 挂起当前任务并运行下一个任务
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}