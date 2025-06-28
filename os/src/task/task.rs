use super::TaskContext;

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit,  // 未初始化
    Ready,   // 准备运行
    Running, // 正在运行
    Exited,  // 已退出
}

#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus, // 任务状态
    pub task_cx: TaskContext,    // 任务信息
    pub task_start_time: usize,  // 任务开始时间
    pub task_end_time: usize,    // 任务结束时间
}
