#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::force_shutdown;

#[unsafe(no_mangle)]
fn main() -> i32 {
    force_shutdown();
    0
}
