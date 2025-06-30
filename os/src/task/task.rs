use log::error;
use super::TaskContext;
use crate::config::{TRAP_CONTEXT, kernel_stack_position};
use crate::mm::{KERNEL_SPACE, MapPermission, MemorySet, PhysPageNum, VirtAddr};
use crate::trap::{TrapContext, trap_handler};

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    #[allow(dead_code)]
    UnInit, // 未初始化
    Ready,   // 准备运行
    Running, // 正在运行
    Exited,  // 已退出
}

pub struct TaskControlBlock {
    pub task_status: TaskStatus,  // 任务状态
    pub task_cx: TaskContext,     // 任务信息
    pub task_start_time: usize,   // 任务开始时间
    pub task_end_time: usize,     // 任务结束时间
    pub memory_set: MemorySet,    // 任务的内存空间
    pub trap_cx_ppn: PhysPageNum, // TrapContext 在物理内存中的页号
    #[allow(unused)]
    pub base_size: usize, // 统计应用数据的大小
    pub heap_bottom: usize,
    pub program_brk: usize,
}

impl TaskControlBlock {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // 通过 ELF 数据创建应用内存空间
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        // 获取到 trap context 的物理页号
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // 设置应用状态为准备完成
        let task_status = TaskStatus::Ready;
        // 读取内核栈的起始和结束位置
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        // 在内核空间中分配内核栈
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        // 配置任务控制块
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            task_start_time: 0,
            task_end_time: 0,
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
        };
        // 为用户空间准备 TrapContext
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }
    /// 更改程序分隔的位置。如果失败，则返回 None。
    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
        let old_break = self.program_brk;
        let new_brk = self.program_brk as isize + size as isize;
        if new_brk < self.heap_bottom as isize {
            return None;
        }
        let result = if size < 0 {
            self.memory_set
                .shrink_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        } else {
            self.memory_set
                .append_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        };
        if result {
            self.program_brk = new_brk as usize;
            Some(old_break)
        } else {
            None
        }
    }
}
