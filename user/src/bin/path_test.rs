#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use user_lib::{syscall_test, OpenFlags};

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Starting path test...");
    let file = syscall_test(
        "/test.txt\0".as_ptr() as usize,
        OpenFlags::CREATE.bits() as usize,
        0,
    );
    println!("File created with descriptor: {:?}", file);
    if file < 0 {
        panic!("Failed to create file: {}", file);
    }
    0
}
