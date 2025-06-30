use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next};
use crate::timer::get_time_ms;

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

/// # 获取当前开机时间（毫秒）
///
/// 从系统定时器获取当前开机时间，单位为毫秒。
pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// # 获取当前进程的 PID
///
pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    // Forking is not implemented in this simple kernel.
    // In a real kernel, this would create a new process.
    todo!("Fork syscall is not implemented!");
}

pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    todo!("Waitpid syscall is not implemented!");
}

pub fn sys_exec(path: *const u8) -> isize {
    todo!("Exec syscall is not implemented!");
}
