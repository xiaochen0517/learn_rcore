#![no_std]
#![no_main]

extern crate alloc;

#[macro_use]
extern crate user_lib;

use user_lib::{OpenFlags, close, open, write};

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let file = open("test.txt\0", OpenFlags::CREATE);
    println!("File created with descriptor: {:?}", file);
    if file < 0 {
        panic!("Failed to create file: {}", file);
    }
    let write_result = write(file as usize, "Hello, World!\0".as_ref());
    println!("Write result: {:?}", write_result);
    if write_result < 0 {
        panic!("Failed to write to file: {}", write_result);
    }
    let close_result = close(file as usize);
    println!("Close result: {:?}", close_result);
    if close_result < 0 {
        panic!("Failed to close file: {}", close_result);
    }
    0
}
