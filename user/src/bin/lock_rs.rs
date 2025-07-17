#![feature(sync_unsafe_cell)]
#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::{SyncUnsafeCell, UnsafeCell};
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use user_lib::{exit, sleep, thread_create, waittid, yield_};

pub struct Mutex<T> {
    flag: AtomicBool,
    data: SyncUnsafeCell<T>,
}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // 解锁
        self.mutex.unlock();
        // println!("mutex unlocked")
    }
}

impl<T> core::ops::Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> core::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T> Mutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            flag: AtomicBool::new(false),
            data: SyncUnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        while self
            .flag
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            yield_();
        }
        // println!("mutex locked");
        MutexGuard { mutex: self }
    }

    pub fn unlock(&self) {
        self.flag.store(false, Ordering::Relaxed);
    }
}

// static

lazy_static! {
    pub static ref COUNT: Mutex<i32> = Mutex::new(0);
}

fn add_process(i: usize) -> ! {
    {
        println!("start process {}", i);
        let mut guard = COUNT.lock();
        println!("locked process {}", i);
        for _ in 0..1000 {
            *guard += 1;
            sleep(2);
        }
        println!("done process {}", i);
    } // 添加此区域用于自动 Drop 锁，调用 exit 之后会直接结束线程
    exit(0)
}

#[unsafe(no_mangle)]
pub unsafe fn main() -> i32 {
    println!("run lock_rs");
    let mut tid_vec = Vec::new();
    {
        let guard = COUNT.lock();
        println!("Init Count: {}", *guard);
    }
    for i in 0..5 {
        tid_vec.push(thread_create(add_process as usize, i as usize) as usize);
    }
    println!("thread created");
    for tid in tid_vec {
        waittid(tid);
    }
    {
        let guard = COUNT.lock();
        println!("count size {}", *guard);
    }
    0
}
