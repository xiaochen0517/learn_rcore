//! File system in os
mod inode;
mod pipe;
mod stdio;

use crate::mm::UserBuffer;
/// File trait
pub trait File: Send + Sync {
    /// If readable
    fn readable(&self) -> bool;
    /// If writable
    fn writable(&self) -> bool;
    /// Read file to `UserBuffer`
    fn read(&self, buf: UserBuffer) -> usize;
    /// Write `UserBuffer` to file
    fn write(&self, buf: UserBuffer) -> usize;
}

pub use inode::{OpenFlags, list_apps, open_file};
pub use pipe::make_pipe;
pub use stdio::{Stdin, Stdout};
