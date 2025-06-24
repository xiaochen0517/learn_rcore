#![no_main]
#![no_std]
mod console;
mod lang_items;
mod sbi;

use core::arch::global_asm;
use crate::sbi::shutdown;

global_asm!(include_str!("entry.asm"));

#[unsafe(no_mangle)]
fn rust_main() {
    clear_bss();
    println!("Hello, RISC-V World!");

    // panic!("Shutdown machine!");
    shutdown(false);
    // loop {}
}

fn clear_bss() {
    unsafe extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe {
        (a as *mut u8).write_volatile(0);
    })
}
