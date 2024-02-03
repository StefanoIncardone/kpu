use kpu::{Kpu, Memory, Op, Reg, Registers};

fn main() {
    // let kpu = Kpu::default();
    let ops = [
        Op::MoveRegImm { dst: Reg::R0, imm: 3 },
        Op::MoveRegReg { dst: Reg::R1, src: Reg::R0 },
        Op::Nop,
        Op::MoveMemImm { mem_high: 0, mem_low: 19, imm: 42 },
        Op::MoveMemReg { mem_high: 0, mem_low: 19, src: Reg::R1 },
        Op::MoveRegMem { dst: Reg::R3, mem_high: 0, mem_low: 19 },
        Op::Halt,
    ];

    for op in ops {
        println!("{:?}", op);
    }
}
