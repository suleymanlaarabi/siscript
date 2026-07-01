#![forbid(unsafe_code)]

pub mod call;
pub mod error;
pub mod extern_call;
pub mod frame;
pub mod heap;
pub mod limits;
pub mod stack;
pub mod value;
pub mod vm;

pub use error::VmError;
pub use limits::VmLimits;
pub use value::Value;
pub use vm::Vm;
