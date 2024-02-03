use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
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
pub struct Registers {
    pub r0: Register,
    pub r1: Register,
    pub r2: Register,
    pub r3: Register,
    pub(crate) ip: Register,
}

const MEM_SIZE: usize = std::mem::size_of::<u8>() * 1024;

#[derive(Debug, Clone)]
pub struct Memory {
    pub(crate) bytes: Box<[u8; MEM_SIZE]>,
}

impl Deref for Memory {
    type Target = [u8; MEM_SIZE];

    fn deref(&self) -> &Self::Target {
        self.bytes.as_ref()
    }
}

impl DerefMut for Memory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.bytes.as_mut()
    }
}

impl Default for Memory {
    fn default() -> Self {
        let bytes_vec = vec![0u8; MEM_SIZE];
        let bytes_slice = bytes_vec.into_boxed_slice();
        let bytes_slice_raw = Box::into_raw(bytes_slice);
        let bytes = unsafe { Box::from_raw(bytes_slice_raw as *mut [u8; MEM_SIZE]) };

        return Self { bytes };
    }
}

impl Memory {
    pub fn new() -> Self {
        return Self::default();
    }
}

#[derive(Debug, Clone, Default)]
pub struct Kpu {
    pub reg: Registers,
    pub mem: Memory,
}

impl Kpu {
    pub fn reset(&mut self) {
        self.reg.ip.full = 0;
        self.reg.r0.full = 0;
        self.reg.r1.full = 0;
        self.reg.r2.full = 0;
        self.reg.r3.full = 0;
        self.mem.fill(0);
    }
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

impl Op {
    #[inline]
    pub fn bytes(self) -> [u8; 4] {
        return unsafe { std::mem::transmute(self) };
    }
}
