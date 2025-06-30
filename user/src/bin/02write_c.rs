#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::yield_;

const WIDTH: usize = 80;
const HEIGHT: usize = 100;

#[unsafe(no_mangle)]
fn main() -> i32 {
    println!("Test write_c started!");
    for i in 0..HEIGHT {
        for _ in 0..WIDTH {
            print!("C");
        }
        println!(" [{}/{}]", i + 1, HEIGHT);
        // yield_();
    }
    println!("Test write_c OK!");
    0
}
