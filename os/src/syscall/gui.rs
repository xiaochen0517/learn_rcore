use crate::drivers::GPU_DEVICE;
use crate::mm::{MapArea, MapPermission, MapType, PhysAddr, VirtAddr};
use crate::task::current_process;

const FB_VADDR: usize = 0x10000000;

pub fn sys_framebuffer() -> isize {
    let fb = GPU_DEVICE.get_framebuffer();
    let len = fb.len();
    // println!("[kernel] FrameBuffer: addr 0x{:X}, len {}", fb.as_ptr() as usize , len);
    let fb_start_pa = PhysAddr::from(fb.as_ptr() as usize);
    assert!(fb_start_pa.aligned());
    let fb_start_ppn = fb_start_pa.floor();
    let fb_start_vpn = VirtAddr::from(FB_VADDR).floor();
    let pn_offset = fb_start_ppn.0 as isize - fb_start_vpn.0 as isize;

    let current_process = current_process();
    let mut inner = current_process.inner_exclusive_access();
    inner.memory_set.push(
        MapArea::new(
            (FB_VADDR as usize).into(),
            (FB_VADDR + len as usize).into(),
            MapType::Linear(pn_offset),
            MapPermission::R | MapPermission::W | MapPermission::U,
        ),
        None,
    );
    FB_VADDR as isize
}

pub fn sys_framebuffer_flush() -> isize {
    GPU_DEVICE.flush();
    0
}

pub fn sys_cursor_update(pos_x: u32, pos_y: u32) -> isize {
    GPU_DEVICE.update_cursor(pos_x, pos_y);
    0
}

pub fn sys_cursor_pos_get() -> isize {
    let cursor_pos = GPU_DEVICE.get_cursor_pos();
    // 将两个 u32 合并为 u64
    let pos = (cursor_pos.0 as u64) << 32 | cursor_pos.1 as u64;
    pos as isize
}
