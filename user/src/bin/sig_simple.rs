#![no_std]
#![no_main]

extern crate alloc;
extern crate user_lib;

use core::arch::asm;
use user_lib::*;

fn func(sig: usize) {
    println!("user_sig_test passed: {}", sig);
    sigreturn();
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let mut new = SignalAction::default();
    let mut old = SignalAction::default();
    new.handler = func as usize;

    println!("signal_simple: sigaction");
    if sigaction(SIGUSR1, Some(&new), Some(&mut old)) < 0 {
        panic!("Sigaction failed!");
    }
    println!("signal_simple: kill");
    if kill(getpid() as usize, SIGUSR1) < 0 {
        println!("Kill failed!");
        exit(1);
    }
    let mut sig: usize = 0;
    unsafe {
        // 使用汇编读取 a0 寄存器
        asm!("mv {}, a0", out(reg) sig);
    }
    println!("signal_simple: sig {}", sig);
    0
}
