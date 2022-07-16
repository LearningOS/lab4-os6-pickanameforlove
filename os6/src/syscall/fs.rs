//! File and filesystem-related syscalls

use core::iter::Enumerate;

use crate::fs::OSInode;
use crate::fs::StatMode;
use crate::fs::create_new_dir_entry;
use crate::fs::get_hard_links_by_inode_number;
use crate::fs::remove_hard_link;
use crate::mm::VirtAddr;
// use crate::fs::get_inode_by_name;
use crate::mm::translated_byte_buffer;
use crate::mm::translated_str;
use crate::mm::translated_refmut;
use crate::task::current_user_token;
use crate::task::current_task;
use crate::fs::open_file;
use crate::fs::OpenFlags;
use crate::fs::Stat;
use crate::mm::UserBuffer;
use crate::task::translate;
use alloc::sync::Arc;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(
            UserBuffer::new(translated_byte_buffer(token, buf, len))
        ) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(
            UserBuffer::new(translated_byte_buffer(token, buf, len))
        ) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(
        path.as_str(),
        OpenFlags::from_bits(flags).unwrap()
    ) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

// YOUR JOB: 扩展 easy-fs 和内核以实现以下三个 syscall
pub fn sys_fstat(_fd: usize, _st: *mut Stat) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if _fd >= inner.fd_table.len(){
        return -1;
    }
    if let Some(inode) = &inner.fd_table[_fd]{
        let ino = inode.get_inode_number();
        let nlink = get_hard_links_by_inode_number(ino as u32) as u32;
        let t = inode.get_type();
        let mode = if t == 0{StatMode::DIR}else{StatMode::FILE};

        drop(inner);//十分重要
        let vaddr = _st as usize;
        let vaddr_obj = VirtAddr(vaddr);
        let page_off = vaddr_obj.page_offset();
    
        let vpn = vaddr_obj.floor();
    
        let ppn = translate(vpn);
    
        let paddr : usize = ppn.0 << 12 | page_off;
        let st = paddr as *mut Stat;

        unsafe {
            (*st).ino = ino as u64;
            (*st).nlink = nlink;
            (*st).mode = mode;
        }
        return 0;

    }else{
        return -1;
    }
    // -1
}
/// 做题思路应该就是创建一个目录项
/// 这里仅仅涉及到目录
/// 读取目录项需要调用DiskInode的方法read_at，在Inode类中有一个通过名字找到Inode的方法。
/// 这里应该只涉及到目录项的创建，但是仅仅是向rootinode里面写一个目录项呢？还是需要向rootinode下新建一个inode呢？或者是两者都有。
/// 似乎目录项这一条路堵死了，因为我在现在的环境下看不到DirEntry这个类。
/// 经过理解，目录项这是唯一合理的方式。
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    // unsafe{
    //     let mut o_end = _old_name;
    //     while o_end.read_volatile() != 0u8 {
    //         o_end = o_end.add(1);
    //     }
    //     let o_slice =
    //         core::slice::from_raw_parts(_old_name, o_end as usize - _old_name as usize);
    //     let o_name = core::str::from_utf8(o_slice).unwrap();

    //     let mut n_end = _new_name;
    //     while n_end.read_volatile() != 0u8 {
    //         n_end = n_end.add(1);
    //     }
    //     let n_slice =
    //         core::slice::from_raw_parts(_new_name, n_end as usize - _new_name as usize);
    //     let n_name = core::str::from_utf8(n_slice).unwrap();
    //     create_new_dir_entry(o_name, n_name)
    // }
    let token = current_user_token();
    let o_name = translated_str(token, _old_name);
    let n_name = translated_str(token, _new_name);
    create_new_dir_entry(&o_name, &n_name)
    
    // -1
    
}

pub fn sys_unlinkat(_name: *const u8) -> isize {
    //  unsafe {
    //         let mut _end = _name;
    //         while _end.read_volatile() != 0u8 {
    //             _end = _end.add(1);
    //         }
    //         let _slice = core::slice::from_raw_parts(_name, _end as usize - _name as usize);
    //         let name = core::str::from_utf8(_slice).unwrap();
    //         remove_hard_link(name)
    //     }
    let token = current_user_token();
    let name = translated_str(token, _name);
    remove_hard_link(&name)
        
}
