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


/// Return (bottom, top) of a kernel stack in kernel space.
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

pub use crate::boards::{CLOCK_FREQ, MEMORY_END, MMIO};

// 最大应用数量
pub const MAX_APP_NUM: usize = 4;
// 应用基地址
pub const APP_BASE_ADDRESS: usize = 0x80400000;
// 应用大小限制
pub const APP_SIZE_LIMIT: usize = 0x20000;