#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;

#[macro_use]
extern crate user_lib;

use user_lib::console::getchar;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    loop {
        let c = getchar();
        if c == 0 {
            break;
        }
        if c == '\n' as u8 {
            println!("");
        } else {
            print!("{}", c as char);
        }
    }
    println!("");
    0
}
