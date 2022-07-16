// use std::println;

use crate::BLOCK_SZ;

use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};

/// Virtual filesystem layer over easy-fs
pub struct Inode {
    pub block_id: usize,
    pub block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a vfs inode
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }
    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
    /// Find inode under a disk inode by name
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_number() as u32);
            }
        }
        None
    }
    pub fn find_inode_id_by_name(&self, name: &str) -> Option<u32> {
        self.read_disk_inode(|disk_inode| self.find_inode_id(name, disk_inode))
    }
    pub fn get_inode_number(&self) -> usize{
        let inode_size = core::mem::size_of::<DiskInode>();
        let inodes_per_block = (BLOCK_SZ / inode_size) as u32;
        let tem1 = self.block_id - self.fs.lock().inode_area_start_block as usize;
        
        return tem1 * (inodes_per_block as usize) + self.block_offset/inode_size
    }
    pub fn create_hard_link(&self, o_name: &str, n_name: &str) -> isize {  
        if o_name == n_name {
            return -1;
        }
        if let Some(inode_number) =
            self.read_disk_inode(|disk_inode| self.find_inode_id(o_name, disk_inode))
        {
            let mut fs = self.fs.lock();
            self.modify_disk_inode(|root_inode| {
                // append file in the dirent
                let file_count = (root_inode.size as usize) / DIRENT_SZ;
                let new_size = (file_count + 1) * DIRENT_SZ;
                // increase size
                self.increase_size(new_size as u32, root_inode, &mut fs);
                // write dirent
                let dirent = DirEntry::new(n_name, inode_number);
                root_inode.write_at(
                    file_count * DIRENT_SZ,
                    dirent.as_bytes(),
                    &self.block_device,
                );
            });
            return 0;
        } else {
            return -1;
        }
        

        // let o_len = (0usize..).find(|i| _old_name[*i] == 0).unwrap();
        // let o_name = core::str::from_utf8(&_old_name[..o_len]).unwrap();

        // let n_len = (0usize..).find(|i| _new_name[*i] == 0).unwrap();
        // let n_name = core::str::from_utf8(&_new_name[..n_len]).unwrap();
        
    }
    pub fn get_type(&self, disk_inode: &DiskInode) -> usize{
        if disk_inode.is_dir(){
            return 0;
        }else{
            return 1;
        }
    }

    pub fn get_inode_type(&self) -> usize{
        self.read_disk_inode(|disk_inode| {
            self.get_type(disk_inode)
        })
    }

    pub fn remove_hard_link(&self, name: &str) -> isize {
        

        if let Some(inode_number) =
            self.read_disk_inode(|disk_inode| self.find_inode_id(name, disk_inode))
        {
            let res = self.get_inode_number_times(inode_number);
            if res > 1{
                self.modify_disk_inode(|disk_inode| {
                    // append file in the dirent
                    let file_count = (disk_inode.size as usize) / DIRENT_SZ;
                    let mut dirent = DirEntry::empty();
                    for i in 0..file_count {
                        assert_eq!(
                            disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                            DIRENT_SZ,
                        );
                        if dirent.name() == name {
                            let dirent = DirEntry::empty();
                            disk_inode.write_at(
                                DIRENT_SZ * i,
                                dirent.as_bytes(),
                                &self.block_device,
                            );
                        }
                    }
                    
                });
                return 0;
            }else{
                let inode = self.find(name).unwrap();
                drop(inode);
                self.modify_disk_inode(|disk_inode| {
                    // append file in the dirent
                    let file_count = (disk_inode.size as usize) / DIRENT_SZ;
                    let mut dirent = DirEntry::empty();
                    for i in 0..file_count {
                        assert_eq!(
                            disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                            DIRENT_SZ,
                        );
                        if dirent.name() == name {
                            let dirent = DirEntry::empty();
                            disk_inode.write_at(
                                DIRENT_SZ * i,
                                dirent.as_bytes(),
                                &self.block_device,
                            );
                        }
                    }
                    
                });
            }
            return 0;
        } else {
            return -1;
        }
    }
    
    fn get_times_by_inode_number(&self, inode_number: u32, disk_inode: &DiskInode)-> usize{
        let mut res = 0;
        let _fs = self.fs.lock();
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            // println!("DEBUG {} {}",dirent.inode_number(),inode_number);
            if dirent.inode_number() == inode_number {
                res+=1;
            }
        }
        return res;
    }

    pub fn get_inode_number_times(&self, inode_number: u32) -> usize{
        // self.read_disk_inode(|disk_inode| {
        //     self.get_times_by_inode_number(inode_number, disk_inode)
        // })
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut res = 0;
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                if dirent.inode_number() == inode_number{
                    res += 1;
                }
            }
            res
        })
    }
    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }
    /// Increase the size of a disk inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }
    /// Create inode under current inode by name
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        if self
            .modify_disk_inode(|root_inode| {
                // assert it is a directory
                assert!(root_inode.is_dir());
                // has the file been created?
                self.find_inode_id(name, root_inode)
            })
            .is_some()
        {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }
    /// List inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Clear the data in current inode
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
}
