use crate::errors::KernelError;
use crate::isa::InstructionSet;
use crate::memory::Memory;
use crate::reg_file::RegisterFile;
use crate::yan::{RegisterRepr, SyscallRepr};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, stderr, stdin, stdout};
use std::os::unix::fs::OpenOptionsExt;
use std::process;
use std::thread;
use std::time::Duration;

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
enum VM_FD {
    STDIN,
    STDOUT,
    STDERR,
    FILE(u8),
    ERROR,
}

impl VM_FD {
    fn from_u8(fd: u8) -> Self {
        match fd {
            0 => Self::STDIN,
            1 => Self::STDOUT,
            2 => Self::STDERR,
            0xff => Self::ERROR,
            n => Self::FILE(n),
        }
    }

    fn to_u8(&self) -> u8 {
        match *self {
            Self::STDIN => 0,
            Self::STDOUT => 1,
            Self::STDERR => 2,
            Self::FILE(n) => n,
            Self::ERROR => 0xff,
        }
    }
}

pub enum ConsoleMode {
    Direct,
    Buffered,
}

pub struct Kernel {
    pub files: [Option<File>; 0x100],
    pub console_mode: ConsoleMode,
    pub guest_stdout: Vec<u8>,
    pub guest_stderr: Vec<u8>,
}

impl Kernel {
    pub fn new() -> Self {
        Self {
            files: array_init::array_init(|_| None),
            console_mode: ConsoleMode::Direct,
            guest_stdout: Vec::new(),
            guest_stderr: Vec::new(),
        }
    }

    pub fn sys(
        &mut self,
        syscall: u8,
        reg: RegisterRepr,
        reg_file: &mut RegisterFile,
        isa: &InstructionSet,
        memory: &mut Memory,
    ) -> Result<(), KernelError> {
        let mut first_err: Option<KernelError> = None;
        if syscall & isa.get_syscall(SyscallRepr::Open).unwrap() != 0x00
            && let Err(e) = self.sys_open(reg, reg_file, isa, memory)
        {
            first_err.get_or_insert(e);
        }
        if syscall & isa.get_syscall(SyscallRepr::ReadMemory).unwrap() != 0x00
            && let Err(e) = self.sys_read_memory(reg, reg_file, isa, memory)
        {
            first_err.get_or_insert(e);
        }
        if syscall & isa.get_syscall(SyscallRepr::ReadCode).unwrap() != 0x00
            && let Err(e) = self.sys_read_code(reg, reg_file, isa, memory)
        {
            first_err.get_or_insert(e);
        }
        if syscall & isa.get_syscall(SyscallRepr::Write).unwrap() != 0x00
            && let Err(e) = self.sys_write(reg, reg_file, isa, memory)
        {
            first_err.get_or_insert(e);
        }
        if syscall & isa.get_syscall(SyscallRepr::Sleep).unwrap() != 0x00
            && let Err(e) = Self::sys_sleep(reg, reg_file)
        {
            first_err.get_or_insert(e);
        }
        if syscall & isa.get_syscall(SyscallRepr::Exit).unwrap() != 0x00
            && let Err(e) = Self::sys_exit(reg, reg_file)
        {
            first_err.get_or_insert(e);
        }
        match first_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn alloc_fd(&mut self, file: File) -> Result<VM_FD, KernelError> {
        for fd in 3..0xff {
            if self.files[fd].is_none() {
                self.files[fd] = Some(file);
                return Ok(VM_FD::FILE(fd as u8));
            }
        }
        Err(KernelError::InvalidFileDescriptor())
    }

    fn read_fd(&mut self, fd: VM_FD, buf: &mut [u8]) -> Result<usize, KernelError> {
        match fd {
            VM_FD::STDIN => stdin()
                .read(buf)
                .map_err(|e| KernelError::ReadFail("stdin".into(), e.to_string())),
            VM_FD::ERROR => Ok(0),
            VM_FD::STDOUT | VM_FD::STDERR => Err(KernelError::InvalidFileDescriptor()),
            VM_FD::FILE(n) => {
                let Some(file) = self.files.get_mut(n as usize).and_then(Option::as_mut) else {
                    return Err(KernelError::InvalidFileDescriptor());
                };
                file.read(buf)
                    .map_err(|e| KernelError::ReadFail("file".into(), e.to_string()))
            }
        }
    }

    fn write_fd(&mut self, fd: VM_FD, buf: &[u8]) -> Result<usize, KernelError> {
        match fd {
            VM_FD::STDIN => Err(KernelError::InvalidFileDescriptor()),
            VM_FD::STDOUT => {
                let mut out = stdout().lock();
                out.write(buf)
                    .map_err(|e| KernelError::ReadFail("stdout".into(), e.to_string()))
            }
            VM_FD::STDERR => {
                let mut out = stderr().lock();
                out.write(buf)
                    .map_err(|e| KernelError::ReadFail("stderr".into(), e.to_string()))
            }
            VM_FD::ERROR => Ok(0),
            VM_FD::FILE(n) => {
                let Some(file) = self.files.get_mut(n as usize).and_then(Option::as_mut) else {
                    return Err(KernelError::InvalidFileDescriptor());
                };
                file.write(buf)
                    .map_err(|e| KernelError::ReadFail("file".into(), e.to_string()))
            }
        }
    }

    fn sys_open(
        &mut self,
        reg: RegisterRepr,
        reg_file: &mut RegisterFile,
        _isa: &InstructionSet,
        memory: &mut Memory,
    ) -> Result<(), KernelError> {
        let start_addr = reg_file.read_by_addr(0x400).unwrap();
        let mut len = 0;
        while memory.read_mem(start_addr + len) != 0x00 {
            len += 1;
        }
        let Ok(filename) =
            std::str::from_utf8(memory.get_slice(start_addr as usize, len as usize).unwrap())
        else {
            return Err(KernelError::ConversionError());
        };
        let mode_char = memory.read_mem(reg_file.read_by_addr(0x401).unwrap()) as char;
        let perm = memory.read_mem(reg_file.read_by_addr(0x402).unwrap());
        let file = match mode_char {
            'r' => OpenOptions::new().read(true).open(filename),
            'w' => OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(u32::from(perm))
                .open(filename),
            'a' => OpenOptions::new()
                .create(true)
                .append(true)
                .mode(u32::from(perm))
                .open(filename),
            _ => return Err(KernelError::InvalidFileMode(mode_char.to_string())),
        };
        let fd = match file {
            Ok(file) => self.alloc_fd(file)?,
            Err(_) => VM_FD::ERROR,
        };
        reg_file.write_by_repr(reg, fd.to_u8());
        Ok(())
    }

    fn sys_read_memory(
        &mut self,
        reg: RegisterRepr,
        reg_file: &mut RegisterFile,
        _isa: &InstructionSet,
        memory: &mut Memory,
    ) -> Result<(), KernelError> {
        let addr = reg_file.read_by_addr(0x401).unwrap();
        let count = reg_file.read_by_addr(0x402).unwrap();
        let nbytes = ((0x100usize - addr as usize).min(count as usize)) as u8;
        let fd = VM_FD::from_u8(reg_file.read_by_addr(0x400).unwrap());
        let mut buffer = vec![0; nbytes as usize];
        let bytes_read = self.read_fd(fd, &mut buffer)?;
        for (i, byte) in buffer[..bytes_read].iter().enumerate() {
            memory.write_mem(addr + i as u8, *byte);
        }
        reg_file.write_by_repr(reg, bytes_read as u8);
        Ok(())
    }

    fn sys_read_code(
        &mut self,
        reg: RegisterRepr,
        reg_file: &mut RegisterFile,
        _isa: &InstructionSet,
        memory: &mut Memory,
    ) -> Result<(), KernelError> {
        let addr = reg_file.read_by_addr(0x401).unwrap();
        let count = reg_file.read_by_addr(0x402).unwrap();
        let nbytes = ((0x100usize - addr as usize).min(count as usize)) as u8;
        let fd = VM_FD::from_u8(reg_file.read_by_addr(0x400).unwrap());
        let mut buffer = vec![0; nbytes as usize];
        let bytes_read = self.read_fd(fd, &mut buffer)?;
        for (i, byte) in buffer[..bytes_read].iter().enumerate() {
            memory.write_code(addr * 3 + i as u8, *byte);
        }
        reg_file.write_by_repr(reg, bytes_read as u8);
        Ok(())
    }

    fn sys_write(
        &mut self,
        reg: RegisterRepr,
        reg_file: &mut RegisterFile,
        _isa: &InstructionSet,
        memory: &Memory,
    ) -> Result<(), KernelError> {
        let addr = reg_file.read_by_addr(0x401).unwrap();
        let count = reg_file.read_by_addr(0x402).unwrap();
        let nbytes = ((0x100usize - addr as usize).min(count as usize)) as u8;
        let fd = reg_file.read_by_addr(0x400).unwrap();
        match fd {
            1 => {
                let Some(output) = memory.get_slice(addr as usize, nbytes as usize) else {
                    return Err(KernelError::ConversionError());
                };
                match self.console_mode {
                    ConsoleMode::Direct => {
                        let mut out = stdout().lock();
                        out.write_all(output).map_err(|e| {
                            KernelError::ReadFail("stdout".to_string(), e.to_string())
                        })?;
                        out.flush().map_err(|e| {
                            KernelError::ReadFail("stdout".to_string(), e.to_string())
                        })?;
                    }
                    ConsoleMode::Buffered => {
                        self.guest_stdout.extend_from_slice(output);
                    }
                }
                reg_file.write_by_repr(reg, output.len() as u8);
                Ok(())
            }
            2 => {
                let Some(output) = memory.get_slice(addr as usize, nbytes as usize) else {
                    return Err(KernelError::ConversionError());
                };
                match self.console_mode {
                    ConsoleMode::Direct => {
                        let mut out = stderr().lock();
                        out.write_all(output).map_err(|e| {
                            KernelError::ReadFail("stderr".to_string(), e.to_string())
                        })?;
                        out.flush().map_err(|e| {
                            KernelError::ReadFail("stderr".to_string(), e.to_string())
                        })?;
                    }
                    ConsoleMode::Buffered => {
                        self.guest_stderr.extend_from_slice(output);
                    }
                }
                reg_file.write_by_repr(reg, output.len() as u8);
                Ok(())
            }
            0xFF => Ok(()),
            _ => {
                let Some(output) = memory.get_slice(addr as usize, nbytes as usize) else {
                    return Err(KernelError::ConversionError());
                };
                let Some(file) = self.files[fd as usize].as_mut() else {
                    return Err(KernelError::InvalidFileDescriptor());
                };
                let bytes_written = file
                    .write(output)
                    .map_err(|e| KernelError::ReadFail("file".to_string(), e.to_string()))?;
                reg_file.write_by_repr(reg, bytes_written as u8);
                Ok(())
            }
        }
    }

    fn sys_sleep(reg: RegisterRepr, reg_file: &RegisterFile) -> Result<(), KernelError> {
        if let Some(duration) = reg_file.read_by_repr(reg) {
            thread::sleep(Duration::from_secs(u64::from(duration)));
            Ok(())
        } else {
            Err(KernelError::ConversionError())
        }
    }

    fn sys_exit(reg: RegisterRepr, reg_file: &RegisterFile) -> Result<(), KernelError> {
        let code = reg_file.read_by_repr(reg).unwrap_or(0);
        Err(KernelError::Exit(code))
    }
}
