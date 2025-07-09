#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;

#[macro_use]
extern crate user_lib;

use user_lib::{OpenFlags, open, read};

#[unsafe(no_mangle)]
pub fn main(_argc: usize, _argv: &[&str]) -> i32 {
    if _argc != 2 {
        println!("Usage: cat <file>");
        return 1;
    }
    let file_name = _argv[1];
    let fd = open(file_name, OpenFlags::RDONLY);
    if fd < 0 {
        println!(
            "cat: cannot open '{}': No such file or directory",
            file_name
        );
        return 1;
    }
    let mut buf = [0u8; 1024];
    loop {
        let size = read(fd as usize, &mut buf);
        if size < 0 {
            println!("read error: {}", size);
        }
        if size == 0 {
            break; // EOF
        }
        if let Ok(s) = core::str::from_utf8(&buf[..size as usize]) {
            print!("{}", s);
        } else {
            println!("cat: invalid UTF-8 sequence in file '{}'", file_name);
            return 1;
        }
    }
    println!("");
    0
}
