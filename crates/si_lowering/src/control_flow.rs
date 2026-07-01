#![forbid(unsafe_code)]

use si_bytecode::Instruction;

pub fn patch_jump(instructions: &mut [Instruction], at: usize, target: usize) {
    match &mut instructions[at] {
        Instruction::Jump(slot) | Instruction::JumpIfFalse(slot) => *slot = target as u32,
        _ => {}
    }
}
