#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{fork, yield_};

#[unsafe(no_mangle)]
fn main() -> i32 {
    println!("Enter test proc -----------");
    fork();
    // 下面的代码会被执行两次
    println!("Test proc fork -----------");
    loop {
        yield_();
    }
    0
}
