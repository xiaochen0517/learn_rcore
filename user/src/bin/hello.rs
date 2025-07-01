#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;

#[macro_use]
extern crate user_lib;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Hello World!!!");
    0
}
