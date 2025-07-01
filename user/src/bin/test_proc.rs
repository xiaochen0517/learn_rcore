#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::fork;

#[unsafe(no_mangle)]
fn main() -> i32 {
    println!("Enter test proc -----------");
    fork();
    0
}
