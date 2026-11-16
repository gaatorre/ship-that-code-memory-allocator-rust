extern crate libc;

use std::fmt;

use core::mem::size_of;
use core::ptr;
use std::cmp;
use std::io::{self, BufRead};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, TryFromBytes};

#[derive(Debug, TryFromBytes, IntoBytes, KnownLayout, Immutable, PartialEq)]
#[repr(u32)]
enum IsTaken {
    Taken = 0xDEADBEEF,
    NotTaken = 0xDEADC00E,
}

#[derive(Debug, TryFromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
struct UsedBlock {
    size: u32,
    taken: IsTaken,
}

impl UsedBlock {
    fn set(&mut self, size: usize) {
        self.size = size as u32;
        self.taken = IsTaken::Taken;
    }
}

#[derive(Debug, TryFromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
struct FreeBlock {
    size: u32,
    taken: IsTaken,
    next: u32,
    prev: u32,
}

impl FreeBlock {
    fn set(&mut self, next: u32, prev: u32) {
        self.taken = IsTaken::NotTaken;
        self.next = next;
        self.prev = prev;
    }
}

#[derive(Debug, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
struct FreeListMeta {
    size: usize,
    tail: usize,
    free_list: usize,
    waste: usize,
}

const HEAP_BASE: usize = size_of::<FreeListMeta>();

impl FreeListMeta {
    fn set(&mut self, size: usize) {
        self.size = size;
        self.tail = HEAP_BASE;
        self.free_list = 0;
        self.waste = 0;
    }
}

#[derive(Debug)]
struct FreeList {
    heap: *mut u8,
}

impl fmt::Display for FreeList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.heap.is_null() {
            write!(f, "NULL")
        } else {
            let metadata: &mut FreeListMeta = unsafe { &mut *(self.heap as *mut FreeListMeta) };
            write!(f, "FreeList:\n")?;
            write!(f, "\tsize: {}\n", metadata.size)?;
            write!(f, "\ttail: {}\n", metadata.tail)?;
            write!(f, "\tfree-list: {}\n", metadata.free_list)?;
            
            let mut next = metadata.free_list;
            while next != 0 {
                let free_block: &FreeBlock = unsafe { & *(self.heap.add(next) as *const FreeBlock) };
                write!(f, "\t\tsize: {}, next: {}, prev: {}\n", free_block.size, free_block.next, free_block.prev)?;
                next = free_block.next as usize;
            }

            Ok(())
        }
    }
}

impl FreeList {
    fn new() -> Self {
        Self {
            heap: ptr::null_mut(),
        }
    }

    fn init(&mut self, size: usize) -> Result<(), ()> {
        let size_to_request = size + size_of::<FreeListMeta>();

        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                size_to_request,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1, // No file descriptor
                0,  // Offset is 0 for anonymous mappings
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(());
        }

        self.heap = ptr as *mut u8;

        let metadata: &mut FreeListMeta = unsafe { &mut *(self.heap as *mut FreeListMeta) };
        metadata.set(size);

        println!("OK");

        Ok(())
    }

    fn alloc(&mut self, size: usize) {
        if self.heap.is_null() {
            println!("OOM");
        }

        let metadata: &mut FreeListMeta = unsafe { &mut *(self.heap as *mut FreeListMeta) };

        let block_sz = cmp::max(size, 16);

        // 1. Look for a whole big enough
        let mut prev_link = 0usize;
        let mut cur_link = metadata.free_list;
        while cur_link != 0 {
            let cur_block: &mut FreeBlock =
                unsafe { &mut *(self.heap.wrapping_add(cur_link) as *mut FreeBlock) };

            // We found a block that fits!
            if cur_block.size as usize >= size {
                cur_block.taken = IsTaken::Taken; 

                metadata.waste += cur_block.size as usize - size;

                // We are at head of the list
                if prev_link == 0 {
                    // Remove this node from the list
                    metadata.free_list = cur_block.next as usize;

                    // If there is a next node, fix the pointers
                    if cur_block.next != 0 {
                        let next_block: &mut FreeBlock =
                            unsafe { &mut *(self.heap.add(cur_block.next as usize) as *mut FreeBlock) };
                        next_block.prev = 0;
                    }
                }
                // We are not at the head
                else {
                    let prev_block: &mut FreeBlock =
                        unsafe { &mut *(self.heap.wrapping_add(prev_link) as *mut FreeBlock) };
                    prev_block.next = cur_block.next;

                    // If there is a next node, fix the pointers
                    if cur_block.next != 0 {
                        let next_block: &mut FreeBlock =
                            unsafe { &mut *(self.heap.add(cur_block.next as usize) as *mut FreeBlock) };
                        next_block.prev = prev_link as u32;
                    }
                }

                println!("{}", cur_link + 8 - HEAP_BASE);

                return;
            }

            prev_link = cur_link;
            cur_link = cur_block.next as usize;
        }

        // 2. Allocate to the tail
        if (metadata.tail + 8 + block_sz) > metadata.size {
            println!("OOM");
        } else {
            // allocate a block
            let block_header: &mut UsedBlock =
                unsafe { &mut *(self.heap.wrapping_add(metadata.tail) as *mut UsedBlock) };

            block_header.set(block_sz);
            metadata.waste += block_sz - size;

            println!("{}", metadata.tail + 8 - HEAP_BASE);

            // save the metadata
            metadata.tail += 8 + block_sz;
        }
    }

    fn free(&mut self, addr: usize) {
        if self.heap.is_null() {
            println!("OOM");
        }

        let metadata: &mut FreeListMeta = unsafe { &mut *(self.heap as *mut FreeListMeta) };

        let header_addr = addr - 8 + HEAP_BASE;
        let free_block: &mut FreeBlock =
            unsafe { &mut *(self.heap.wrapping_add(header_addr) as *mut FreeBlock) };

        if free_block.taken != IsTaken::Taken {
            println!("BAD");
            return;
        }

        // Initialize the free list
        if metadata.free_list == 0 {
            metadata.free_list = header_addr;
            // Only item in the link list
            free_block.set(0, 0);
        }
        // Append to the end of the free_list
        else {
            let mut free: &mut FreeBlock =
                unsafe { &mut *(self.heap.wrapping_add(metadata.free_list) as *mut FreeBlock) };
            let mut free_addr = metadata.free_list;

            while free.next != 0 {
                free_addr = free.next as usize;
                free = unsafe { &mut *(self.heap.wrapping_add(free_addr) as *mut FreeBlock) };
            }

            free.next = header_addr as u32;
            free_block.set(0, free_addr as u32);
        }

        println!("OK");
    }

    fn free_list(&self) {
        if self.heap.is_null() {
            println!("OOM");
            return;
        }

        let metadata: &FreeListMeta = unsafe { & *(self.heap as *const FreeListMeta) };
        let mut offset = HEAP_BASE;

        while offset - HEAP_BASE < metadata.size {
            let free: &FreeBlock = unsafe { & *(self.heap.wrapping_add(offset) as *const FreeBlock) };
            if free.taken == IsTaken::NotTaken {
                println!("{}:{}", offset - HEAP_BASE + 8, free.size);
                offset += 8 + free.size as usize;
            } else if free.taken == IsTaken::Taken {
                offset += 8 + free.size as usize;
            } else {
                // We've reached the end of the set of blocks
                break;
            }
        }

    }

    fn waste(&self) {
        let metadata: &FreeListMeta = unsafe { & *(self.heap as *const FreeListMeta) };
        println!("{}", metadata.waste);
    }
}

fn main() {
    let mut alloc = FreeList::new();

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let l = line.unwrap();
        if l.is_empty() {
            continue;
        }
        let cmd: Vec<&str> = l.split(" ").collect();

        match cmd[0] {
            "INIT" => alloc.init(cmd[1].parse().unwrap()).expect("Init failed"),
            "ALLOC" => alloc.alloc(cmd[1].parse().unwrap()),
            "FREE" => alloc.free(cmd[1].parse().unwrap()),
            "FREELIST" => alloc.free_list(),
            "WASTE" => alloc.waste(),
            _ => {}
        };

        //println!("{}", alloc);
    }
}
