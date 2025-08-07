use crate::board::{VIRTGPU_XRES, VIRTGPU_YRES};
use crate::drivers::bus::virtio::VirtioHal;
use crate::sync::UPIntrFreeCell;
use alloc::{sync::Arc, vec, vec::Vec};
use core::any::Any;
use core::cell::Cell;
use core::ops::{Deref, DerefMut};
use embedded_graphics::pixelcolor::Rgb888;
use log::{debug, info};
use tinybmp::Bmp;
use virtio_drivers::{VirtIOGpu, VirtIOHeader};

const VIRTIO7: usize = 0x10007000;
const CURSOR_SIZE: usize = 64;
pub trait GpuDevice: Send + Sync + Any {
    fn get_framebuffer(&self) -> &mut [u8];
    fn draw_soft_cursor(&self);
    fn flush(&self);
    fn update_cursor(&self, pos_x: u32, pos_y: u32);
    fn get_cursor_pos(&self) -> (u32, u32);
}

lazy_static::lazy_static!(
    pub static ref GPU_DEVICE: Arc<dyn GpuDevice> = Arc::new(VirtIOGpuWrapper::new(true));
);

pub struct VirtIOGpuWrapper {
    gpu: UPIntrFreeCell<VirtIOGpu<'static, VirtioHal>>,
    fb: &'static [u8],
    /// 光标位置的原始图像信息
    cursor_origin_buf: UPIntrFreeCell<Vec<u8>>,
    cursor_buf: Vec<u8>,
    cursor_pos: UPIntrFreeCell<(u32, u32)>,
    use_soft_cursor: bool,
    cursor_visible: bool,
}
static BMP_DATA: &[u8] = include_bytes!("../../assert/mouse.bmp");
impl VirtIOGpuWrapper {
    pub fn new(use_soft_cursor: bool) -> Self {
        unsafe {
            let mut virtio =
                VirtIOGpu::<VirtioHal>::new(&mut *(VIRTIO7 as *mut VirtIOHeader)).unwrap();

            let fbuffer = virtio.setup_framebuffer().unwrap();
            let len = fbuffer.len();
            let ptr = fbuffer.as_mut_ptr();
            let fb = core::slice::from_raw_parts_mut(ptr, len);

            let bmp = Bmp::<Rgb888>::from_slice(BMP_DATA).unwrap();
            let raw = bmp.as_raw();
            let mut b = Vec::new();
            for i in raw.image_data().chunks(3) {
                let mut v = i.to_vec();
                b.append(&mut v);
                if i == [255, 255, 255] {
                    b.push(0x00)
                } else {
                    b.push(0xff)
                }
            }
            info!("Setting up cursor, image size: {}", b.len());

            if !use_soft_cursor {
                virtio.setup_cursor(b.as_slice(), 50, 50, 40, 40).unwrap();
            }

            Self {
                gpu: UPIntrFreeCell::new(virtio),
                fb,
                cursor_origin_buf: UPIntrFreeCell::new(vec![0; b.len()]),
                cursor_buf: b,
                cursor_pos: UPIntrFreeCell::new((0, 0)),
                use_soft_cursor,
                cursor_visible: true,
            }
        }
    }
}

impl VirtIOGpuWrapper {
    fn recover_origin_cursor(&self) {
        debug!("cursor buf size: {:?}", self.cursor_buf.len());
        debug!(
            "cursor origin buf size: {:?}",
            self.cursor_origin_buf.exclusive_access().len()
        );
        let fb = GPU_DEVICE.get_framebuffer();
        let (pos_x, pos_y) = *self.cursor_pos.exclusive_access();
        // 遍历光标区域
        for y_offset in 0..CURSOR_SIZE as u32 {
            let fb_y = pos_y + y_offset;
            if fb_y >= VIRTGPU_YRES {
                // 假设屏幕高度为800，应该使用常量
                break;
            }

            for x_offset in 0..CURSOR_SIZE as u32 {
                let fb_x = pos_x + x_offset;
                if fb_x >= VIRTGPU_XRES {
                    break;
                }

                // 将 origin buf 内容设置到原先的光标位置
                // 计算在cursor_buf中的位置（4字节/像素）
                let cursor_idx = (y_offset as usize * CURSOR_SIZE + x_offset as usize) * 4;

                // 确保不会越界
                let cob = self.cursor_origin_buf.exclusive_access();
                if cursor_idx + 2 < cob.len() {
                    // 计算在framebuffer中的位置（4字节/像素）
                    let fb_idx = (fb_y as usize * VIRTGPU_XRES as usize + fb_x as usize) * 4;

                    // 确保不会越界
                    if fb_idx + 4 < fb.len() {
                        // 从cursor_buf读取RGB值并写入framebuffer
                        fb[fb_idx] = cob[cursor_idx]; // Blue
                        fb[fb_idx + 1] = cob[cursor_idx + 1]; // Green
                        fb[fb_idx + 2] = cob[cursor_idx + 2]; // Red
                        fb[fb_idx + 3] = cob[cursor_idx + 3]; // Alpha
                    }
                }
            }
        }
    }
}

impl GpuDevice for VirtIOGpuWrapper {
    fn draw_soft_cursor(&self) {
        if !self.use_soft_cursor || !self.cursor_visible {
            return;
        }
        let fb = GPU_DEVICE.get_framebuffer();
        let mut cob = self.cursor_origin_buf.exclusive_access();
        let (pos_x, pos_y) = *self.cursor_pos.exclusive_access();

        // 遍历光标区域
        for y_offset in 0..CURSOR_SIZE as u32 {
            let fb_y = pos_y + y_offset;
            if fb_y >= VIRTGPU_YRES {
                // 假设屏幕高度为800，应该使用常量
                break;
            }

            for x_offset in 0..CURSOR_SIZE as u32 {
                let fb_x = pos_x + x_offset;
                if fb_x >= VIRTGPU_XRES {
                    break;
                }

                // 计算在cursor_buf中的位置（4字节/像素）
                let cursor_idx = (y_offset as usize * CURSOR_SIZE + x_offset as usize) * 4;

                // 确保不会越界
                if cursor_idx + 2 < self.cursor_buf.len() {
                    // 计算在framebuffer中的位置（4字节/像素）
                    let fb_idx = (fb_y as usize * VIRTGPU_XRES as usize + fb_x as usize) * 4;
                    // 确保不会越界
                    if fb_idx + 4 < fb.len() {
                        // 将原始位置的内容进行保存
                        cob[cursor_idx] = fb[fb_idx];
                        cob[cursor_idx + 1] = fb[fb_idx + 1];
                        cob[cursor_idx + 2] = fb[fb_idx + 2];
                        cob[cursor_idx + 3] = fb[fb_idx + 3];
                        if self.cursor_buf[cursor_idx + 3] <= 0x0 {
                            continue;
                        }
                        // 从cursor_buf读取RGB值并写入framebuffer
                        fb[fb_idx] = self.cursor_buf[cursor_idx]; // Blue
                        fb[fb_idx + 1] = self.cursor_buf[cursor_idx + 1]; // Green
                        fb[fb_idx + 2] = self.cursor_buf[cursor_idx + 2]; // Red
                        fb[fb_idx + 3] = self.cursor_buf[cursor_idx + 3]; // Alpha
                    }
                }
            }
        }
    }
    fn flush(&self) {
        self.draw_soft_cursor();
        self.gpu.exclusive_access().flush().unwrap();
    }
    fn get_framebuffer(&self) -> &mut [u8] {
        unsafe {
            let ptr = self.fb.as_ptr() as *const _ as *mut u8;
            core::slice::from_raw_parts_mut(ptr, self.fb.len())
        }
    }
    fn update_cursor(&self, pos_x: u32, pos_y: u32) {
        if self.use_soft_cursor {
            // 复原光标位置图像
            self.recover_origin_cursor();
            // 修改光标位置
            *self.cursor_pos.exclusive_access() = (pos_x, pos_y);
            // 重新刷新
            self.flush();
        } else {
            self.gpu
                .exclusive_access()
                .move_cursor(pos_x, pos_y)
                .unwrap();
        }
    }
    fn get_cursor_pos(&self) -> (u32, u32) {
        *self.cursor_pos.exclusive_access()
    }
}
