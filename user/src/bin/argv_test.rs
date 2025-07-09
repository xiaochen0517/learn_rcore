#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;

#[macro_use]
extern crate user_lib;

#[unsafe(no_mangle)]
pub fn main(_argc: usize, _argv: &[&str]) -> i32 {
    println!("argv_test: argc = {}", _argc);
    for (i, arg) in _argv.iter().enumerate() {
        println!("argv_test: argv[{}] = {}", i, arg);
    }
    0
}
