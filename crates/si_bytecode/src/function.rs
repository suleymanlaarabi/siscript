#![forbid(unsafe_code)]

use si_ast::ty::Type;
use si_core::symbol::Symbol;

use crate::constant::Constant;
use crate::instruction::Instruction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FunctionId(pub u32);

impl FunctionId {
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeFunction {
    pub id: FunctionId,
    pub name: Symbol,
    pub params: Vec<Type>,
    pub return_type: Type,
    pub locals_count: usize,
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Constant>,
}
