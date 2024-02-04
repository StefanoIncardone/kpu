use kpu::{Kpu, Op, Reg};

fn main() {
    let ops = [
        Op::MoveRegImm { dst: Reg::R0, imm: 3 },
        Op::MoveRegReg { dst: Reg::R1, src: Reg::R0 },
        Op::Nop,
        Op::MoveMemImm { mem_high: 0, mem_low: 19, imm: 42 },
        Op::MoveMemReg { mem_high: 0, mem_low: 19, src: Reg::R1 },
        Op::MoveRegMem { dst: Reg::R3, mem_high: 0, mem_low: 19 },
        Op::Halt,
    ];

    let mut kpu = Kpu::new();
    if let Err(err) = kpu.load(&ops) {
        panic!("Error: {}", err);
    }

    loop {
        let executed_op = kpu.step();
        println!("{}", executed_op);

        if let Op::Halt = executed_op {
            kpu.reset();
            break;
        };
    }
}
