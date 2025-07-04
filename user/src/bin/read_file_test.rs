#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use user_lib::{OpenFlags, open, read};

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let file = open("test.txt\0", OpenFlags::RDONLY);
    println!("File open with descriptor: {:?}", file);
    if file < 0 {
        panic!("Failed to open file: {}", file);
    }
    let mut buffer = [0u8; 1024];
    let read_result = read(file as usize, &mut buffer);
    println!("Read result: {:?}", read_result);
    if read_result < 0 {
        panic!("Failed to write to file: {}", read_result);
    }
    println!(
        "Read data: {:?}",
        core::str::from_utf8(&buffer[..read_result as usize]).unwrap()
    );
    0
}
