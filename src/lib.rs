use std::{
    error::Error, fmt::{Debug, Display}, mem::{size_of, transmute}
};

#[repr(C)]
#[derive(Clone, Copy)]
pub union Register {
    pub full: u8,
}

impl Default for Register {
    fn default() -> Self {
        return Self { full: 0 };
    }
}

impl Debug for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            #[rustfmt::skip]
            return write!(f,
"Register {{
    full: {},
}}",
                self.full
            );
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Registers {
    pub(crate) r0: Register,
    pub(crate) r1: Register,
    pub(crate) r2: Register,
    pub(crate) r3: Register,
    pub(crate) ip: Register,
}


#[derive(Debug, Clone, Copy)]
pub enum Reg {
    R0,
    R1,
    R2,
    R3,
    IP,
}

impl Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Self::R0 => write!(f, "r0"),
            Self::R1 => write!(f, "r1"),
            Self::R2 => write!(f, "r2"),
            Self::R3 => write!(f, "r3"),
            Self::IP => write!(f, "ip"),
        };
    }
}

pub(crate) const MEM_SIZE: usize = 1024;
pub(crate) const TEXT_SIZE: usize = MEM_SIZE / 4;
pub(crate) const DATA_SIZE: usize = MEM_SIZE - TEXT_SIZE;

#[derive(Debug)]
pub(crate) struct Memory {
    pub(crate) bytes: Box<[u8; MEM_SIZE]>,
    pub(crate) text: &'static mut [[u8; OP_SIZE]; TEXT_SIZE / OP_SIZE],
    pub(crate) data: &'static mut [u8; DATA_SIZE],
}

impl Default for Memory {
    fn default() -> Self {
        let bytes_slice_raw = Box::into_raw(vec![0u8; MEM_SIZE].into_boxed_slice());
        let mut bytes = unsafe { Box::from_raw(bytes_slice_raw as *mut [u8; MEM_SIZE]) };
        let text = unsafe { &mut *(&mut bytes[..TEXT_SIZE] as *mut _ as *mut _) };
        let data = unsafe { &mut *(&mut bytes[TEXT_SIZE..] as *mut _ as *mut _) };

        return Self { bytes, text, data };
    }
}

#[derive(Debug, Default)]
pub struct Kpu {
    pub(crate) reg: Registers,
    pub(crate) mem: Memory,
}

impl Kpu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.reg.r0.full = 0;
        self.reg.r1.full = 0;
        self.reg.r2.full = 0;
        self.reg.r3.full = 0;
        self.reg.ip.full = 0;
        self.mem.bytes.fill(0);
    }

    pub fn load(&mut self, ops: &[Op]) -> Result<(), LoadError> {
        let program_size = ops.len() * size_of::<u32>();
        if program_size > TEXT_SIZE {
            return Err(LoadError::Size { size_of_loaded_program: program_size });
        }

        for (op, text_bytes) in ops.iter().zip(self.mem.text.iter_mut()) {
            text_bytes.copy_from_slice(op.bytes().as_slice());
        }

        return Ok(());
    }

    pub(crate) fn reg(&self, reg: Reg) -> &Register {
        return match reg {
            Reg::R0 => &self.reg.r0,
            Reg::R1 => &self.reg.r1,
            Reg::R2 => &self.reg.r2,
            Reg::R3 => &self.reg.r3,
            Reg::IP => &self.reg.ip,
        };
    }

    pub(crate) fn reg_mut(&mut self, reg: Reg) -> &mut Register {
        return match reg {
            Reg::R0 => &mut self.reg.r0,
            Reg::R1 => &mut self.reg.r1,
            Reg::R2 => &mut self.reg.r2,
            Reg::R3 => &mut self.reg.r3,
            Reg::IP => &mut self.reg.ip,
        };
    }

    // TODO(stefano): check for overlflow of ip register
    pub fn step(&mut self) -> Op {
        // decoding the instruction
        let op = unsafe {
            let op_bytes = &self.mem.text[self.reg.ip.full as usize];
            transmute(*op_bytes)
        };

        // executing the instruction
        match op {
            Op::MoveMemImm { mem_high, mem_low, imm } => {
                let offset = u16::from_be_bytes([mem_high, mem_low]) as usize;
                self.mem.data[offset] = imm;
            }
            Op::MoveMemReg { mem_high, mem_low, src } => {
                let offset = u16::from_be_bytes([mem_high, mem_low]) as usize;
                self.mem.data[offset] = unsafe { self.reg(src).full };
            }
            Op::MoveRegImm { dst, imm } => self.reg_mut(dst).full = imm,
            Op::MoveRegReg { dst, src } => self.reg_mut(dst).full = unsafe { self.reg(src).full },
            Op::MoveRegMem { dst, mem_high, mem_low } => {
                let offset = u16::from_be_bytes([mem_high, mem_low]) as usize;
                self.reg_mut(dst).full = self.mem.data[offset];
            }
            Op::Halt => return Op::Halt,
            Op::Nop => {}
        }

        unsafe { self.reg.ip.full += 1 };
        return op;
    }
}

/// Operations available to our KPU
///
/// Every opcode is 32bits
#[repr(u8)]
#[allow(clippy::unusual_byte_groupings)]
#[derive(Debug, Clone, Copy)]
pub enum Op {
    /// Does nothing
    ///
    /// # Opcode
    ///
    /// 0000_0000 XXXX_XXXX XXXX_XXXX XXXX_XXXX
    ///
    /// - X bits: ignored (may possibly contain garbage values)
    ///
    /// # Example
    ///
    /// ```kasm
    /// nop
    /// ```
    Nop = 0b0000_0000,

    /// Moves a signed 8bit integer into the specified destination register
    ///
    /// # Opcode
    ///
    /// 0001_0000 0000_dddd vvvv_vvvv XXXX_XXXX
    ///
    /// - d bits: destination register
    /// - v bits: signed 8bit integer
    /// - X bits: ignored (may possibly contain garbage values)
    ///
    /// # Example
    ///
    /// ```kasm
    /// move r0, 19
    /// ```
    MoveRegImm { dst: Reg, imm: u8 } = 0b0001_0000,

    /// Copies the contents of the source register to the destination register
    ///
    /// # Opcode
    ///
    /// 0001_0001 0000_dddd 0000_ssss XXXX_XXXX
    ///
    /// - d bits: destination register
    /// - s bits: source register
    /// - X bits: ignored (may possibly contain garbage values)
    ///
    /// # Example
    ///
    /// ```kasm
    /// move r1, r2
    /// ```
    MoveRegReg { dst: Reg, src: Reg } = 0b0001_0001,

    /// Moves a signed 8bit integer into the specified memory offset
    ///
    /// # Opcode
    ///
    /// 0010_0000 mmmm_mmmm mmmm_mmmm vvvv_vvvv
    ///
    /// - m bits: memory offset
    /// - v bits: signed 8bit integer
    ///
    /// # Example
    ///
    /// ```kasm
    /// move [19], 42
    /// ```
    MoveMemImm { mem_high: u8, mem_low: u8, imm: u8 } = 0b0010_0000,

    /// Copies the contents of the source register into the specified memory offset
    ///
    /// # Opcode
    ///
    /// 0010_0001 mmmm_mmmm mmmm_mmmm 0000_ssss
    ///
    /// - m bits: memory offset
    /// - s bits: source register
    ///
    /// # Example
    ///
    /// ```kasm
    /// move [19], r0
    /// ```
    MoveMemReg { mem_high: u8, mem_low: u8, src: Reg } = 0b0010_0001,

    /// Copies the contents at the specified memory offset to the specified destination register
    ///
    /// # Opcode
    ///
    /// 0010_0010 0000_dddd mmmm_mmmm mmmm_mmmm
    ///
    /// - d bits: destination register
    /// - m bits: memory offset
    ///
    /// # Example
    ///
    /// ```kasm
    /// move r0, [19]
    /// ```
    MoveRegMem { dst: Reg, mem_high: u8, mem_low: u8 } = 0b0010_0010,

    /// Stops the execution of the processor
    ///
    /// # Opcode
    ///
    /// 1111_1111 XXXX_XXXX XXXX_XXXX XXXX_XXXX
    ///
    /// - X bits: ignored (may possibly contain garbage values)
    ///
    /// # Example
    ///
    /// ```kasm
    /// nop
    /// ```
    Halt = 0b1111_1111,
}

impl Display for Op {
    /// Source coude view
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Self::Nop => write!(f, "nop"),
            Self::Halt => write!(f, "halt"),
            Self::MoveMemImm { mem_high, mem_low, imm } => {
                write!(f, "move [{}], {}", u16::from_be_bytes([*mem_high, *mem_low]), imm)
            }
            Self::MoveMemReg { mem_high, mem_low, src } => {
                write!(f, "move [{}], {}", u16::from_be_bytes([*mem_high, *mem_low]), src)
            }
            Self::MoveRegImm { dst, imm } => write!(f, "move {}, {}", dst, imm),
            Self::MoveRegMem { dst, mem_high, mem_low } => {
                write!(f, "move {}, [{}]", dst, u16::from_be_bytes([*mem_high, *mem_low]))
            }
            Self::MoveRegReg { dst, src } => write!(f, "move {}, {}", dst, src),
        };
    }
}

const OP_SIZE: usize = size_of::<Op>();

impl Op {
    #[inline]
    pub fn bytes(self) -> [u8; OP_SIZE] {
        return unsafe { transmute(self) };
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LoadError {
    Size { size_of_loaded_program: usize },
}

impl Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Self::Size { size_of_loaded_program } => write!(
                f,
                "size of loaded program ({} bytes) exceedes the .text section size of {} bytes",
                size_of_loaded_program, TEXT_SIZE
            ),
        };
    }
}

impl Error for LoadError {}
