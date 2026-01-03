pub struct Memory {
    pub memory: [u8; 0x500],
    pub code_len: usize,
}

impl Memory {
    pub fn new(code: &[u8]) -> Self {
        let code_len = code.len().min(0x100);
        let mut memory = Memory {
            memory: [0x0; 0x500],
            code_len,
        };
        memory.memory[0x00..code_len].copy_from_slice(&code[..code_len]);
        memory
    }

    pub fn read_mem(&self, offs: u8) -> u8 {
        self.memory[0x300_usize + offs as usize]
    }

    pub fn write_mem(&mut self, offs: u8, val: u8) {
        self.memory[0x300_usize + offs as usize] = val;
    }

    pub fn read_code(&self, offs: u8) -> u8 {
        self.memory[offs as usize]
    }

    pub fn read_instruction(&self, offs: u8) -> u32 {
        let addr = offs as usize;
        u32::from_be_bytes([
            0,
            self.memory[addr],
            self.memory[addr + 1],
            self.memory[addr + 2],
        ])
    }

    pub fn write_code(&mut self, offs: u8, val: u8) {
        self.memory[offs as usize] = val;
    }

    pub fn get_slice(&self, start_addr: usize, len: usize) -> Option<&[u8]> {
        if start_addr + len <= self.memory.len() {
            Some(&self.memory[0x300 + start_addr..(0x300 + start_addr + len)])
        } else {
            None
        }
    }
}
