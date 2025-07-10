#[allow(unused)]

// 应用栈大小
pub const USER_STACK_SIZE: usize = 4096 * 2;
// 内核栈大小
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
// 内核堆大小
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;
// 内存分页大小
pub const PAGE_SIZE: usize = 0x1000;
// 页号位数
pub const PAGE_SIZE_BITS: usize = 0xc;

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

pub use crate::boards::{CLOCK_FREQ, MEMORY_END, MMIO};
