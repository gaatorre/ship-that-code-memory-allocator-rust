use std::convert::From;
use std::io::{self, BufRead};

struct Header {
    size: u32,
    taken: bool,
}

impl From<&[u8]> for Header {
    fn from(slice: &[u8]) -> Self {
        let size = u32::from_le_bytes([slice[0], slice[1], slice[2], 0u8]);
        let taken = slice[3] == 1;

        Self { size, taken }
    }
}

impl Header {
    fn size(&self) -> usize {
        self.size as usize
    }

    fn free(&self) -> bool {
        !self.taken
    }

    fn set_taken(&mut self) {
        self.taken = true;
    }

    fn set_free(&mut self) {
        self.taken = false;
    }

    fn to_slice(&self) -> [u8; 4] {
        let mut bytes = self.size.to_le_bytes();
        bytes[3] = if self.taken { 0x1 } else { 0x0 };

        bytes
    }

    fn new_alloc(size: usize) -> Header {
        Self {
            size: size as u32,
            taken: true,
        }
    }
}

struct HeapAlloc {
    size: usize,
    arr: Vec<u8>,
    tail: usize,
}

impl HeapAlloc {
    fn new() -> Self {
        Self {
            size: 0,
            arr: Default::default(),
            tail: 0,
        }
    }

    fn init(&mut self, size: usize) {
        self.size = size;
        self.arr = vec![0; size];

        println!("OK");
    }

    fn alloc(&mut self, size: usize) {
        if self.arr.capacity() == 0 {
            println!("OOM");
            return;
        }

        // Look for a valid block!
        let mut idx = 0;
        while idx < self.tail {
            let mut header = Header::from(&self.arr[idx..idx + 4]);

            if header.free() && header.size == size as u32 {
                header.set_taken();
                self.arr[idx..idx + 4].copy_from_slice(&header.to_slice());

                println!("{}", idx + 4);
                return;
            }

            idx += header.size() + 4;
        }

        // Allocate a new block
        if self.tail + 4 + size <= self.size {
            let header = Header::new_alloc(size);
            self.arr[self.tail..self.tail + 4].copy_from_slice(&header.to_slice());

            let addr = self.tail + 4;
            self.tail += 4 + size;

            println!("{}", addr);
            return;
        }

        println!("OOM");
    }

    fn blocks(&self) {
        if self.arr.capacity() == 0 {
            println!("OOM");
            return;
        }

        let mut idx = 0;
        while idx < self.tail {
            let header = Header::from(&self.arr[idx..idx + 4]);

            println!(
                "{}:{}:{}",
                idx + 4,
                header.size(),
                if header.free() { "free" } else { "used" }
            );

            idx += header.size() + 4;
        }
    }

    fn free(&mut self, addr: usize) {
        if self.arr.capacity() == 0 {
            println!("OOM");
            return;
        } else if addr > self.size {
            println!("BAD");
            return;
        }

        let header_idx = addr - 4;
        let mut header = Header::from(&self.arr[header_idx..header_idx + 4]);
        if header.free() {
            println!("BAD");
        } else {
            header.set_free();
            self.arr[header_idx..header_idx + 4].copy_from_slice(&header.to_slice());
            println!("OK");
        }
    }
}

fn main() {
    let mut alloc = HeapAlloc::new();

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let l = line.unwrap();
        if l.is_empty() {
            continue;
        }
        let cmd: Vec<&str> = l.split(" ").collect();

        match cmd[0] {
            "INIT" => alloc.init(cmd[1].parse().unwrap()),
            "ALLOC" => alloc.alloc(cmd[1].parse().unwrap()),
            "BLOCKS" => alloc.blocks(),
            "FREE" => alloc.free(cmd[1].parse().unwrap()),
            _ => {}
        };
    }
}
