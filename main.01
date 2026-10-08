use std::io::{self, BufRead};

// TODO (why-allocator): implement per the lesson description.

struct BumpAlloc {
    size: usize,
    bump: usize,
}

impl BumpAlloc {
    fn new() -> Self {
        Self { size: 0, bump: 0 }
    }

    fn init(&mut self, size: usize) -> Result<String, String> {
        self.bump = 0;
        self.size = size;

        Ok("OK".into())
    }

    fn alloc(&mut self, size: usize) -> Result<String, String> {
        if self.bump + size <= self.size {
            let tmp = self.bump;
            self.bump += size;

            Ok(format!("{}", tmp))
        } else {
            Err("OOM".into())
        }
    }

    fn reset(&mut self) -> Result<String, String> {
        self.bump = 0;

        Ok("OK".into())
    }

    fn used(&mut self) -> Result<String, String> {
        Ok(format!("{}", self.bump))
    }
}

fn main() {
    let mut alloc = BumpAlloc::new();

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let l = line.unwrap();
        if l.is_empty() {
            continue;
        }
        let cmd: Vec<&str> = l.split(" ").collect();

        let res = match cmd[0] {
            "INIT" => alloc.init(cmd[1].parse().unwrap()),
            "RESET" => alloc.reset(),
            "ALLOC" => alloc.alloc(cmd[1].parse().unwrap()),
            "USED" => alloc.used(),
            _ => Err("WTF".into()),
        };

        match res {
            Ok(msg) => println!("{}", msg),
            Err(msg) => println!("{}", msg),
        }
    }
}
