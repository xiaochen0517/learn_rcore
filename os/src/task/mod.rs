mod action;
mod context;
mod manager;
mod pid;
mod processor;
mod signal;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::fs::{OpenFlags, open_file};
use crate::sbi::shutdown;
use alloc::sync::Arc;
pub use context::TaskContext;
use lazy_static::*;
use manager::fetch_task;
use manager::remove_from_pid2task;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub use action::{SignalAction, SignalActions};
pub use manager::{add_task, pid2task};
pub use pid::{KernelStack, PidHandle, pid_alloc};
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
};
pub use signal::{MAX_SIG, SignalFlags};

pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();

    let pid = task.getpid();
    if pid == IDLE_PID {
        println!(
            "[kernel] Idle process exit with exit_code {} ...",
            exit_code
        );
        if exit_code != 0 {
            //crate::sbi::shutdown(255); //255 == -1 for err hint
            shutdown(true)
        } else {
            //crate::sbi::shutdown(0); //0 for success hint
            shutdown(false)
        }
    }

    // remove from pid2task
    remove_from_pid2task(task.getpid());
    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    // drop file descriptors
    inner.fd_table.clear();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        TaskControlBlock::new(v.as_slice())
    });
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}

/// # 检查当前任务中是否存在错误或终止类型的信号
pub fn check_signals_error_of_current() -> Option<(i32, &'static str)> {
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();
    // println!(
    //     "[K] check_signals_error_of_current {:?}",
    //     task_inner.signals
    // );
    task_inner.signals.check_error()
}

pub fn current_add_signal(signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.signals |= signal;
    // println!(
    //     "[K] current_add_signal:: current task sigflag {:?}",
    //     task_inner.signals
    // );
}

/// # 调用内核信号处理函数
/// 
fn call_kernel_signal_handler(signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    match signal {
        SignalFlags::SIGSTOP => {
            // 冻结程序，并移除已经处理的信号
            task_inner.frozen = true;
            task_inner.signals ^= SignalFlags::SIGSTOP;
        }
        SignalFlags::SIGCONT => {
            // 解除冻结状态，并移除已经处理的信号
            if task_inner.signals.contains(SignalFlags::SIGCONT) {
                task_inner.signals ^= SignalFlags::SIGCONT;
                task_inner.frozen = false;
            }
        }
        _ => {
            // println!(
            //     "[K] call_kernel_signal_handler:: current task sigflag {:?}",
            //     task_inner.signals
            // );
            // 终止任务
            task_inner.killed = true;
        }
    }
}

/// # 调用用户信号处理函数
fn call_user_signal_handler(sig: usize, signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    // 获取到对应的用户信号处理函数入口地址
    let handler = task_inner.signal_actions.table[sig].handler;
    // 如果用户程序定义了信号处理函数，才会执行
    if handler != 0 {
        // 将正在处理的信号类型设置为当前信号，并将信号从待处理列表中移除
        task_inner.handling_sig = sig as isize;
        task_inner.signals ^= signal;

        // 将当前任务的 trap context 备份
        let trap_ctx = task_inner.get_trap_cx();
        task_inner.trap_ctx_backup = Some(*trap_ctx);

        // 将 trap context 设置为用户信号处理函数的上下文
        trap_ctx.sepc = handler;

        // 定义信号处理函数的参数
        trap_ctx.x[10] = sig;
    } else {
        // default action
        println!("[K] task/call_user_signal_handler: default action: ignore it or kill process");
    }
    // 最后执行完成后，需要用户程序调用 `sys_sigreturn` 来恢复上下文
}

fn check_pending_signals() {
    // 遍历所有信号的类型
    for sig in 0..(MAX_SIG + 1) {
        // 获取当前任务控制块
        let task = current_task().unwrap();
        let task_inner = task.inner_exclusive_access();
        // 转换当前检查的信号类型为 SignalFlags
        let signal = SignalFlags::from_bits(1 << sig).unwrap();
        // 需要当前任务待处理列表中存在此信号，并且此信号没有被屏蔽
        if task_inner.signals.contains(signal) && (!task_inner.signal_mask.contains(signal)) {
            let mut masked = true;
            // 判断当前任务是否是在处理信号
            let handling_sig = task_inner.handling_sig;
            if handling_sig == -1 {
                // 没有正在处理的信号
                masked = false;
            } else {
                // 正在处理信号，获取到处理的信号类型
                let handling_sig = handling_sig as usize;
                // 如果当前处理的信号类型没有被屏蔽
                if !task_inner.signal_actions.table[handling_sig]
                    .mask
                    .contains(signal)
                {
                    masked = false;
                }
            }
            if !masked {
                // 移除引用
                drop(task_inner);
                drop(task);
                // 如果信号是内核信号，则调用内核信号处理函数
                if signal == SignalFlags::SIGKILL
                    || signal == SignalFlags::SIGSTOP
                    || signal == SignalFlags::SIGCONT
                    || signal == SignalFlags::SIGDEF
                {
                    // 内核信号处理
                    call_kernel_signal_handler(signal);
                } else {
                    // 用户信号处理
                    call_user_signal_handler(sig, signal);
                    return;
                }
            }
        }
    }
}

/// # 处理当前任务的信号
pub fn handle_signals() {
    loop {
        // 检查信号并进行处理
        check_pending_signals();
        // 检查当前任务是否被冻结或被杀死
        let (frozen, killed) = {
            let task = current_task().unwrap();
            let task_inner = task.inner_exclusive_access();
            (task_inner.frozen, task_inner.killed)
        };
        // 如果当前任务没有被冻结或者已经被杀死，则退出循环
        if !frozen || killed {
            break;
        }
        // 若为其他情况，程序需要暂停当前任务，运行下一个任务
        suspend_current_and_run_next();
    }
}
