#![forbid(unsafe_code)]

pub mod builder;
pub mod constant;
pub mod error;
pub mod function;
pub mod instruction;
pub mod module;
pub mod signature;
pub mod value;

pub use builder::BytecodeBuilder;
pub use constant::Constant;
pub use error::BytecodeError;
pub use function::{BytecodeFunction, FunctionId};
pub use instruction::Instruction;
pub use module::{BytecodeModule, EnumMeta, ExternId, StructMeta};
pub use signature::FunctionSignature;
pub use value::{HeapId, RefBase, RefValue, SliceValue, Value};
