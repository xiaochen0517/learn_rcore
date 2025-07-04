//! Implementation of [`PageTableEntry`] and [`PageTable`].

use alloc::string::String;
use super::{FrameTracker, PhysPageNum, StepByOne, VirtAddr, VirtPageNum, frame_alloc, PhysAddr};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;
use log::error;

bitflags! {
    /// page table entry flags
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// page table entry structure
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// page table structure
pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    /// 临时用于从用户空间获取参数。
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    /// 查找页表项，如果不存在则创建
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        // 将虚拟页号转为三个索引
        let idxs = vpn.indexes();
        // 从根页表开始查找
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        // 遍历索引
        for (i, idx) in idxs.iter().enumerate() {
            // 获取当前层级的页表项
            let pte = &mut ppn.get_pte_array()[*idx];
            // 如果是最后一级页表项，记录结果
            if i == 2 {
                result = Some(pte);
                break;
            }
            // 如果当前页表项无效，则分配新的页框
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            // 没有找到对应的页表，继续向下查找
            ppn = pte.ppn();
        }
        result
    }

    /// 查找页表项，如果存在则返回，否则返回 `None`
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }

    /// 将虚拟页号与对应的物理页号映射到一起
    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        if pte.is_valid() {
            error!("Mapping vpn {:?} to ppn {:?}", vpn, ppn);
        }
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    /// 将虚拟页号解除映射
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    /// 获取到指定虚拟页号的页表项，如果不存在则返回 `None`
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte| {
            //println!("translate_va:va = {:?}", va);
            let aligned_pa: PhysAddr = pte.ppn().into();
            //println!("translate_va:pa_align = {:?}", aligned_pa);
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }

    /// 获取地址空间标识
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

/// 通过页表将指针转换为可变的 u8 Vec
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    // 将 token 转换为 PageTable
    let page_table = PageTable::from_token(token);
    // 获取到开始的虚拟地址
    let mut start = ptr as usize;
    // 计算结束的虚拟地址
    let end = start + len;
    // 保存返回的 Vec
    let mut v = Vec::new();
    // 循环获取每个虚拟页的内容
    while start < end {
        // 将开始虚拟地址转换为 VirtAddr 类型
        let start_va = VirtAddr::from(start);
        // 将开始虚拟地址转换为虚拟页号
        let mut vpn = start_va.floor();
        // 通过虚拟页号获取到对应的物理页号
        let ppn = page_table.translate(vpn).unwrap().ppn();
        // 自增到下一个虚拟页
        vpn.step();
        // 将本循环中自增后的虚拟页号作为结束虚拟页号，将其转换为虚拟地址
        let mut end_va: VirtAddr = vpn.into();
        // 将本循环的结束虚拟地址与整体结束虚拟地址进行比较，取最小值避免超出单个页的范围
        end_va = end_va.min(VirtAddr::from(end));
        // 获取到结束地址在物理页中的偏移，如果结束虚拟地址的偏移为 0，则表示是页的开始
        if end_va.page_offset() == 0 {
            // 直接将整个物理页的内容添加到 Vec 中
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            // 否则，需要从开始虚拟地址的偏移到结束虚拟地址的偏移之间的内容
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        // 更新开始虚拟地址为结束虚拟地址
        start = end_va.into();
    }
    v
}
/// translate a pointer to a mutable u8 Vec end with `\0` through page table to a `String`
pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8 = *(page_table
            .translate_va(VirtAddr::from(va))
            .unwrap()
            .get_mut());
        if ch == 0 {
            break;
        } else {
            string.push(ch as char);
            va += 1;
        }
    }
    string
}
///translate a generic through page table and return a mutable reference
pub fn translated_refmut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    //println!("into translated_refmut!");
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    //println!("translated_refmut: before translate_va");
    page_table
        .translate_va(VirtAddr::from(va))
        .unwrap()
        .get_mut()
}


///Array of u8 slice that user communicate with os
pub struct UserBuffer {
    ///U8 vec
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    ///Create a `UserBuffer` by parameter
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }
    ///Length of `UserBuffer`
    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for b in self.buffers.iter() {
            total += b.len();
        }
        total
    }
}

impl IntoIterator for UserBuffer {
    type Item = *mut u8;
    type IntoIter = UserBufferIterator;
    fn into_iter(self) -> Self::IntoIter {
        UserBufferIterator {
            buffers: self.buffers,
            current_buffer: 0,
            current_idx: 0,
        }
    }
}
/// Iterator of `UserBuffer`
pub struct UserBufferIterator {
    buffers: Vec<&'static mut [u8]>,
    current_buffer: usize,
    current_idx: usize,
}

impl Iterator for UserBufferIterator {
    type Item = *mut u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_buffer >= self.buffers.len() {
            None
        } else {
            let r = &mut self.buffers[self.current_buffer][self.current_idx] as *mut _;
            if self.current_idx + 1 == self.buffers[self.current_buffer].len() {
                self.current_idx = 0;
                self.current_buffer += 1;
            } else {
                self.current_idx += 1;
            }
            Some(r)
        }
    }
}
