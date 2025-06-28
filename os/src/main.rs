#![no_main]
#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;
#[macro_use]
extern crate bitflags;

#[macro_use]
mod console;
mod boards;
mod config;
mod lang_items;
mod loader;
pub mod logging;
mod mm;
mod sbi;
mod sync;
pub mod syscall;
mod task;
mod timer;
pub mod trap;

use crate::mm::heap_allocator::{heap_test, init_heap};
use core::arch::global_asm;
use log::{debug, error, info, trace, warn};

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

fn clear_bss() {
    unsafe extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe {
        (a as *mut u8).write_volatile(0);
    })
}

fn print_boot_info() {
    unsafe extern "C" {
        fn stext(); // begin addr of text segment
        fn etext(); // end addr of text segment
        fn srodata(); // start addr of Read-Only data segment
        fn erodata(); // end addr of Read-Only data ssegment
        fn sdata(); // start addr of data segment
        fn edata(); // end addr of data segment
        fn sbss(); // start addr of BSS segment
        fn ebss(); // end addr of BSS segment
        fn boot_stack_lower_bound(); // stack lower bound
        fn boot_stack_top(); // stack top
    }
    println!("[kernel] Hello, RISC-V World!");
    println!("[kernel] Hello, world!");
    trace!(
        "[kernel] .text [{:#x}, {:#x})",
        stext as usize, etext as usize
    );
    debug!(
        "[kernel] .rodata [{:#x}, {:#x})",
        srodata as usize, erodata as usize
    );
    info!(
        "[kernel] .data [{:#x}, {:#x})",
        sdata as usize, edata as usize
    );
    warn!(
        "[kernel] boot_stack top=bottom={:#x}, lower_bound={:#x}",
        boot_stack_top as usize, boot_stack_lower_bound as usize
    );
    error!("[kernel] .bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
    info!("[kernel] -- Boot Info End --");
}

#[unsafe(no_mangle)]
fn rust_main() {
    clear_bss();
    logging::init();
    print_boot_info();
    trap::init();
    loader::load_apps();
    init_heap();
    heap_test();

    // 启动计时中断
    trap::enable_timer_interrupt();
    // 设置下一个计时中断触发
    timer::set_next_trigger();
    // 运行第一个任务
    task::run_first_task();
    panic!("Unreachable in rust_main!");
}
