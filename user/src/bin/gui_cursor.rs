#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{RgbColor, Size};
use user_lib::{
    cursor_pos_get, cursor_update, event_get, Display, VIRTGPU_XRES, VIRTGPU_YRES,
};

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let mut display = Display::new(Size::new(VIRTGPU_XRES, VIRTGPU_YRES));
    display.clear(Rgb888::BLACK).unwrap();
    display.flush();
    loop {
        match event_get() {
            Some(event) => {
                println!("event type: {}", event.event_type);
                println!("event code: {}", event.code);
                println!("event value: {}", event.value);

                // 处理鼠标事件
                let mut has_mouse_movement = false;
                let mut x_delta = 0;
                let mut y_delta = 0;
                // 鼠标事件类型通常为0x02 (EV_REL)
                if event.event_type == 0x02 {
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

                // 如果有鼠标移动，更新光标位置
                if has_mouse_movement {
                    // 获取当前位置并计算新位置
                    let current_pos = cursor_pos_get();
                    let (mut new_x, mut new_y) = (
                        current_pos.0 as i32 + x_delta,
                        current_pos.1 as i32 + y_delta,
                    );
                    println!("cursor moved: ({}, {})", new_x, new_y);

                    // 确保光标在屏幕范围内
                    if new_x < 0 {
                        new_x = 0;
                    }
                    if new_y < 0 {
                        new_y = 0;
                    }
                    if new_x >= VIRTGPU_XRES as i32 {
                        new_x = VIRTGPU_XRES as i32 - 1;
                    }
                    if new_y >= VIRTGPU_YRES as i32 {
                        new_y = VIRTGPU_YRES as i32 - 1;
                    }
                    // 更新光标位置
                    cursor_update(new_x as usize, new_y as usize);
                }
            }
            None => {}
        }
    }
    0
}
