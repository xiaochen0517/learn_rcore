use super::{
    block_cache_sync_all, get_block_cache, Bitmap, BlockDevice, DiskInode, DiskInodeType, Inode,
    SuperBlock,
};
use crate::BLOCK_SZ;
use alloc::sync::Arc;
use log::debug;
use spin::Mutex;
/// 简易文件系统
pub struct EasyFileSystem {
    /// 硬盘设备
    pub block_device: Arc<dyn BlockDevice>,
    /// 索引节点位图
    pub inode_bitmap: Bitmap,
    /// 数据区域位图
    pub data_bitmap: Bitmap,
    /// 索引节点区域起始块编号
    inode_area_start_block: u32,
    /// 数据区域起始块编号
    data_area_start_block: u32,
}

/// 单个块的大小
type DataBlock = [u8; BLOCK_SZ];

/// An easy fs over a block device
impl EasyFileSystem {
    /// A data block of block size
    pub fn create(
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
    ) -> Arc<Mutex<Self>> {
        // 创建索引节点位图
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize);
        // 位图大小
        let inode_num = inode_bitmap.maximum();
        // 索引区域大小
        let inode_area_blocks =
            ((inode_num * core::mem::size_of::<DiskInode>() + BLOCK_SZ - 1) / BLOCK_SZ) as u32;
        // 索引总大小
        let inode_total_blocks = inode_bitmap_blocks + inode_area_blocks;
        // 剩余部分为数据区域总大小
        let data_total_blocks = total_blocks - 1 - inode_total_blocks;
        // 当前数据区域的位图大小为剩余块除以 4097 (每 4096 个数据块需要一个位图块)
        let data_bitmap_blocks = (data_total_blocks + 4096) / 4097;
        // 将剩余的数据总块大小减去数据位图块占用，剩余为数据区域块大小
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;
        // 创建数据区域位图
        let data_bitmap = Bitmap::new(
            (1 + inode_total_blocks) as usize,
            data_bitmap_blocks as usize,
        );
        // 创建 EasyFileSystem 实例
        let mut efs = Self {
            block_device: Arc::clone(&block_device),
            inode_bitmap,
            data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: 1 + inode_total_blocks + data_bitmap_blocks,
        };
        // 清理所有的块，将所有块初始化为 0
        for i in 0..total_blocks {
            get_block_cache(i as usize, Arc::clone(&block_device))
                .lock()
                .modify(0, |data_block: &mut DataBlock| {
                    for byte in data_block.iter_mut() {
                        *byte = 0;
                    }
                });
        }
        // 写入超级块
        get_block_cache(0, Arc::clone(&block_device)).lock().modify(
            0,
            |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks,
                    inode_bitmap_blocks,
                    inode_area_blocks,
                    data_bitmap_blocks,
                    data_area_blocks,
                );
            },
        );
        // 写入第一个文件夹索引块数据
        // create a inode for root node "/"
        assert_eq!(efs.alloc_inode(), 0);
        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);
        get_block_cache(root_inode_block_id as usize, Arc::clone(&block_device))
            .lock()
            .modify(root_inode_offset, |disk_inode: &mut DiskInode| {
                disk_inode.initialize(DiskInodeType::Directory);
            });
        // 将输入同步到硬盘
        block_cache_sync_all();
        Arc::new(Mutex::new(efs))
    }
    /// Open a block device as a filesystem
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        debug!("Opening BlockDevice as EasyFileSystem");
        // read SuperBlock
        get_block_cache(0, Arc::clone(&block_device))
            .lock()
            .read(0, |super_block: &SuperBlock| {
                assert!(super_block.is_valid(), "Error loading EFS!");
                let inode_total_blocks =
                    super_block.inode_bitmap_blocks + super_block.inode_area_blocks;
                let efs = Self {
                    block_device,
                    inode_bitmap: Bitmap::new(1, super_block.inode_bitmap_blocks as usize),
                    data_bitmap: Bitmap::new(
                        (1 + inode_total_blocks) as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks,
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks,
                };
                Arc::new(Mutex::new(efs))
            })
    }
    /// Get the root inode of the filesystem
    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        let block_device = Arc::clone(&efs.lock().block_device);
        // acquire efs lock temporarily
        let (block_id, block_offset) = efs.lock().get_disk_inode_pos(0);
        // release efs lock
        Inode::new(block_id, block_offset, Arc::clone(efs), block_device)
    }
    /// Get inode by id
    pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {
        let inode_size = core::mem::size_of::<DiskInode>();
        let inodes_per_block = (BLOCK_SZ / inode_size) as u32;
        let block_id = self.inode_area_start_block + inode_id / inodes_per_block;
        (
            block_id,
            (inode_id % inodes_per_block) as usize * inode_size,
        )
    }
    /// Get data block by id
    pub fn get_data_block_id(&self, data_block_id: u32) -> u32 {
        self.data_area_start_block + data_block_id
    }
    /// Allocate a new inode
    pub fn alloc_inode(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }

    /// Allocate a data block
    pub fn alloc_data(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block
    }
    /// Deallocate a data block
    pub fn dealloc_data(&mut self, block_id: u32) {
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                data_block.iter_mut().for_each(|p| {
                    *p = 0;
                })
            });
        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_area_start_block) as usize,
        )
    }
}
