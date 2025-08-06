use crate::drivers::GPU_DEVICE;
use crate::drivers::bus::virtio::VirtioHal;
use crate::sync::{Condvar, UPIntrFreeCell};
use crate::task::schedule;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::any::Any;
use log::debug;
use virtio_drivers::{VirtIOHeader, VirtIOInput};

const VIRTIO5: usize = 0x10005000;
const VIRTIO6: usize = 0x10006000;

struct VirtIOInputInner {
    virtio_input: VirtIOInput<'static, VirtioHal>,
    events: VecDeque<u64>,
}

struct VirtIOInputWrapper {
    inner: UPIntrFreeCell<VirtIOInputInner>,
    condvar: Condvar,
}

pub trait InputDevice: Send + Sync + Any {
    fn read_event(&self) -> u64;
    fn handle_irq(&self);
    fn is_empty(&self) -> bool;
}

lazy_static::lazy_static!(
    pub static ref KEYBOARD_DEVICE: Arc<dyn InputDevice> = Arc::new(VirtIOInputWrapper::new(VIRTIO5));
    pub static ref MOUSE_DEVICE: Arc<dyn InputDevice> = Arc::new(VirtIOInputWrapper::new(VIRTIO6));
);

impl VirtIOInputWrapper {
    pub fn new(addr: usize) -> Self {
        let inner = VirtIOInputInner {
            virtio_input: unsafe {
                VirtIOInput::<VirtioHal>::new(&mut *(addr as *mut VirtIOHeader)).unwrap()
            },
            events: VecDeque::new(),
        };
        Self {
            inner: unsafe { UPIntrFreeCell::new(inner) },
            condvar: Condvar::new(),
        }
    }
}

impl InputDevice for VirtIOInputWrapper {
    fn read_event(&self) -> u64 {
        loop {
            let mut inner = self.inner.exclusive_access();
            if let Some(event) = inner.events.pop_front() {
                return event;
            } else {
                let task_cx_ptr = self.condvar.wait_no_sched();
                drop(inner);
                schedule(task_cx_ptr);
            }
        }
    }

    fn handle_irq(&self) {
        debug!("input irq handle active");
        let mut count = 0;
        let mut result = 0;

        // 跟踪鼠标位置变化
        let mut x_delta = 0;
        let mut y_delta = 0;
        let mut has_mouse_movement = false;

        self.inner.exclusive_session(|inner| {
            inner.virtio_input.ack_interrupt();
            while let Some(event) = inner.virtio_input.pop_pending_event() {
                count += 1;
                result = (event.event_type as u64) << 48
                    | (event.code as u64) << 32
                    | (event.value) as u64;

                debug!("input irq handle : {:?}", result);

                // 处理鼠标事件
                // 鼠标事件类型通常为0x02 (EV_REL)
                if event.event_type == 0x03 {
                    match event.code {
                        // 鼠标X轴移动 (REL_X)
                        0x00 => {
                            x_delta = event.value as i32;
                            has_mouse_movement = true;
                        }
                        // 鼠标Y轴移动 (REL_Y)
                        0x01 => {
                            y_delta = event.value as i32;
                            has_mouse_movement = true;
                        }
                        _ => {}
                    }
                }

                inner.events.push_back(result);
            }
        });

        // 如果有鼠标移动，更新光标位置
        if has_mouse_movement {
            // 获取当前位置并计算新位置
            let current_pos = GPU_DEVICE.get_cursor_pos();
            let (mut new_x, mut new_y) = (
                current_pos.0 as i32 + x_delta,
                current_pos.1 as i32 + y_delta,
            );
            debug!("cursor moved: ({}, {})", new_x, new_y);

            // 确保光标在屏幕范围内
            if new_x < 0 {
                new_x = 0;
            }
            if new_y < 0 {
                new_y = 0;
            }
            if new_x >= crate::board::VIRTGPU_XRES as i32 {
                new_x = crate::board::VIRTGPU_XRES as i32 - 1;
            }
            if new_y >= crate::board::VIRTGPU_YRES as i32 {
                new_y = crate::board::VIRTGPU_YRES as i32 - 1;
            }
            // 更新光标位置
            GPU_DEVICE.update_cursor(new_x as u32, new_y as u32);
        }

        if count > 0 {
            self.condvar.signal();
        };
    }

    fn is_empty(&self) -> bool {
        self.inner.exclusive_access().events.is_empty()
    }
}
