//!['FrameAllocator'] 的实现，它控制作系统中的所有帧。

use super::{PhysAddr, PhysPageNum};
use crate::config::MEMORY_END;
use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::*;

/// 保存一个物理页的跟踪器
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        // 初始化时将物理页的内容清零
        let bytes_array = ppn.get_bytes_array();
        for i in bytes_array {
            *i = 0;
        }
        Self { ppn }
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

/// 释放帧跟踪器时，调用 `frame_dealloc` 释放物理页
impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

/// 帧分配器的 trait 定义
trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

/// 保存当前帧分配器的状态
pub struct StackFrameAllocator {
    /// 当前分配器已分配过的物理页号，会随着分配器的使用而增加不会减少
    current: usize,
    /// 分配器的结束物理页号，分配器只能分配到这个物理页号
    end: usize,
    /// 已回收的物理页号列表，优先从这里分配物理页
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    /// 使用 `l` 和 `r` 初始化分配器的范围
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
    }
}
impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    /// 分配一个物理页
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            // 如果有回收的物理页，优先从这里分配
            Some(ppn.into())
        } else if self.current == self.end {
            // 如果没有回收的物理页且当前分配器已分配到结束位置，返回 None
            None
        } else {
            // 否则分配一个新的物理页
            self.current += 1;
            Some((self.current - 1).into())
        }
    }

    /// 回收一个物理页
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        // 检查物理页号是否在分配器的范围内，并且已回收的物理页号列表中没有该物理页号
        if ppn >= self.current || self.recycled.iter().any(|&v| v == ppn) {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        // 回收物理页，将其添加到已回收的物理页号列表中
        self.recycled.push(ppn);
    }
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    /// 全局帧分配器实例
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> =
        unsafe { UPSafeCell::new(FrameAllocatorImpl::new()) };
}

/// 使用 'ekernel' 内核结束点和 'MEMORY_END' 内存结束点，为可分配范围启动帧分配器
pub fn init_frame_allocator() {
    unsafe extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}

/// 分配一个物理页
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new)
}

/// 释放一个物理页
fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

#[allow(unused)]
/// 测试帧分配器
pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    v.clear();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    drop(v);
    println!("frame_allocator_test passed!");
}
