pub mod address;
pub mod frame_allocator;
pub mod heap_allocator;
pub mod memory_set;
pub mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
pub use address::{StepByOne, VPNRange};
pub use frame_allocator::{FrameTracker, frame_alloc, frame_dealloc};
pub use memory_set::{KERNEL_SPACE, MapPermission, MemorySet, kernel_token, remap_test};
pub use page_table::{
    PTEFlags, PageTable, PageTableEntry, UserBuffer, translated_byte_buffer, translated_refmut,
    translated_str, translated_ref
};

/// initiate heap allocator, frame allocator and kernel space
pub fn init() {
    // 初始化内核堆分配器
    heap_allocator::init_heap();
    // 初始化物理内存帧分配器
    frame_allocator::init_frame_allocator();
    // 启动 SV39 多级页表
    KERNEL_SPACE.exclusive_access().activate();
}
