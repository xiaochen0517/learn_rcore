#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

extern crate alloc;

use alloc::vec;
use user_lib::exit;
use user_lib::{semaphore_create, semaphore_down, semaphore_up};
use user_lib::{sleep, thread_create, waittid};

const SEM_SYNC: usize = 0;

unsafe fn first() -> ! {
    println!("first work ---- start ----");
    sleep(10);
    println!("first work ---- sleep 10 done ----");
    semaphore_up(SEM_SYNC);
    println!("first work ---- up ----");
    println!("first work ---- done ----");
    exit(0)
}

unsafe fn second() -> ! {
    println!("second work ---- start ----");
    semaphore_down(SEM_SYNC);
    println!("second work ---- down ----");
    println!("second work ---- done ----");
    exit(0)
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    // create semaphores
    assert_eq!(semaphore_create(0) as usize, SEM_SYNC);
    // create threads
    let threads = vec![
        thread_create(first as usize, 0),
        thread_create(second as usize, 0),
    ];
    // wait for all threads to complete
    for thread in threads.iter() {
        waittid(*thread as usize);
    }
    println!("sync_sem passed!");
    0
}
