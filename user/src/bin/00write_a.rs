#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::sleep;

#[unsafe(no_mangle)]
fn main() -> i32 {
    println!("Test write_a started!");
    sleep(2000);
    println!("Test write_a OK!");
    0
}
