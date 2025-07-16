#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

#[macro_use]
extern crate user_lib;

extern crate alloc;

use alloc::vec::Vec;
use user_lib::exit;
use user_lib::{semaphore_create, semaphore_down, semaphore_up};
use user_lib::{thread_create, waittid};

const SEM_MUTEX: usize = 0;
const SEM_EMPTY: usize = 1;
const SEM_AVAIL: usize = 2;
const BUFFER_SIZE: usize = 8;
/// 环形缓冲区
static mut BUFFER: [usize; BUFFER_SIZE] = [0; BUFFER_SIZE];
static mut FRONT: usize = 0;
static mut TAIL: usize = 0;
const PRODUCER_COUNT: usize = 4;
const NUMBER_PER_PRODUCER: usize = 100;

fn producer(id: *const usize) -> ! {
    unsafe {
        let id = *id;
        // 每次生产的数量
        for _ in 0..NUMBER_PER_PRODUCER {
            // 减少缓冲区信号量空间
            semaphore_down(SEM_EMPTY);
            // 获取到互斥锁
            semaphore_down(SEM_MUTEX);
            // 将数据填充到缓冲区中
            BUFFER[TAIL] = id;
            // 更新尾标记值
            TAIL = (TAIL + 1) % BUFFER_SIZE;
            // 释放互斥锁
            semaphore_up(SEM_MUTEX);
            // 增加可用资源数量
            semaphore_up(SEM_AVAIL);
        }
    }
    exit(0)
}

fn consumer() -> ! {
    unsafe {
        // 消耗所有生产者线程锁生产的内容
        for _ in 0..PRODUCER_COUNT * NUMBER_PER_PRODUCER {
            // 首先减少可用资源数量，（如果没有可用的资源会阻塞当前线程）
            semaphore_down(SEM_AVAIL);
            // 获取到互斥锁
            semaphore_down(SEM_MUTEX);
            // 开始消费，更新头标记的值
            print!("{} ", BUFFER[FRONT]);
            FRONT = (FRONT + 1) % BUFFER_SIZE;
            // 释放互斥锁
            semaphore_up(SEM_MUTEX);
            // 将可用空间增大
            semaphore_up(SEM_EMPTY);
        }
    }
    println!("");
    exit(0)
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    // 创建互斥锁信号量
    assert_eq!(semaphore_create(1) as usize, SEM_MUTEX);
    // 创建缓存区大小信号量
    assert_eq!(semaphore_create(BUFFER_SIZE) as usize, SEM_EMPTY);
    // 创建是否可用信号量
    assert_eq!(semaphore_create(0) as usize, SEM_AVAIL);
    // 创建生产者线程
    let ids: Vec<_> = (0..PRODUCER_COUNT).collect();
    let mut threads = Vec::new();
    for i in 0..PRODUCER_COUNT {
        threads.push(thread_create(
            producer as usize,
            &ids.as_slice()[i] as *const _ as usize,
        ));
    }
    // 创建消费者线程
    threads.push(thread_create(consumer as usize, 0));
    // 等待所有线程完成
    for thread in threads.iter() {
        waittid(*thread as usize);
    }
    println!("mpsc_sem passed!");
    0
}
