use crate::syscall::syscall;
use crate::task::suspend_current_and_run_next;
use crate::timer::set_next_trigger;
pub use crate::trap::context::TrapContext;
use core::arch::global_asm;
use log::{debug, error, info};
use riscv::register::scause::Interrupt;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap},
    sepc, sie, stval, stvec,
};

pub mod context;

global_asm!(include_str!("trap.S"));

pub fn init() {
    unsafe extern "C" {
        fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

#[unsafe(no_mangle)]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let spec = sepc::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            // info!("[kernel] CPU timer interrupt, run scheduler.");
            set_next_trigger();
            suspend_current_and_run_next();
        }
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            error!("[kernel] PageFault in application, kernel killed it.");
            panic!("PageFault in application, kernel killed it.");
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            error!("[kernel] IllegalInstruction in application, kernel killed it.");
            error!(
                "[kernel] scause = {:?}, sepc = {:#x}, stval = {:#x}",
                scause.cause(),
                spec,
                stval
            );
            panic!("IllegalInstruction in application, kernel killed it.");
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    cx
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}
