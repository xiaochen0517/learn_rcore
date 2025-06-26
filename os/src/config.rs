// 应用栈大小
pub const USER_STACK_SIZE: usize = 4096 * 2;
// 内核栈大小
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
// 最大应用数量
pub const MAX_APP_NUM: usize = 4;
// 应用基地址
pub const APP_BASE_ADDRESS: usize = 0x80400000;
// 应用大小限制
pub const APP_SIZE_LIMIT: usize = 0x20000;
