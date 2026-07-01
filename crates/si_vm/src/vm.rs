#![forbid(unsafe_code)]

use si_bytecode::{BytecodeModule, Constant, FunctionId, Instruction, RefBase, RefValue, Value};

use crate::call::make_frame;
use crate::error::VmError;
use crate::extern_call::{ExternFn, ExternRegistry};
use crate::frame::Frame;
use crate::heap::Heap;
use crate::limits::VmLimits;
use crate::stack::Stack;

pub struct Vm {
    module: BytecodeModule,
    stack: Stack,
    frames: Vec<Frame>,
    heap: Heap,
    externs: ExternRegistry,
    limits: VmLimits,
    instruction_count: usize,
}

impl Vm {
    pub fn new(module: BytecodeModule) -> Self {
        Self::with_limits(module, VmLimits::default())
    }

    pub fn with_limits(module: BytecodeModule, limits: VmLimits) -> Self {
        Self {
            module,
            stack: Stack::default(),
            frames: Vec::new(),
            heap: Heap::default(),
            externs: ExternRegistry::default(),
            limits,
            instruction_count: 0,
        }
    }

    pub fn register_extern(&mut self, name: &str, func: ExternFn) -> Result<(), VmError> {
        let Some((_, id)) = self
            .module
            .externs
            .iter()
            .find(|(symbol, _)| self.module.symbol_name(**symbol) == name)
        else {
            return Err(VmError::MissingExtern(name.to_string()));
        };
        self.externs.insert(*id, func);
        Ok(())
    }

    pub fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value, VmError> {
        let id = self
            .module
            .find_function(name)
            .ok_or_else(|| VmError::UnknownFunction(name.to_string()))?;
        self.run_function(id, args)
    }

    pub fn call_export(&mut self, name: &str, args: Vec<Value>) -> Result<Value, VmError> {
        let id = self
            .module
            .find_export(name)
            .ok_or_else(|| VmError::UnknownExport(name.to_string()))?;
        self.run_function(id, args)
    }

    pub fn run_function(
        &mut self,
        function_id: FunctionId,
        args: Vec<Value>,
    ) -> Result<Value, VmError> {
        let function = self
            .module
            .functions
            .get(function_id.index())
            .ok_or_else(|| VmError::UnknownFunction(format!("function#{}", function_id.0)))?;
        self.frames.push(make_frame(function, args)?);
        self.run_loop()
    }

    fn run_loop(&mut self) -> Result<Value, VmError> {
        loop {
            self.check_instruction_limit()?;
            if self.frames.len() > self.limits.max_frames {
                return Err(VmError::FrameLimit);
            }
            let frame_idx = self.frames.len().checked_sub(1).ok_or(VmError::InvalidReturnValue)?;
            let instruction = self.fetch(frame_idx)?;
            match instruction {
                Instruction::Const(idx) => {
                    let value = self.constant(frame_idx, idx)?;
                    self.stack.push(value, self.limits.max_stack)?;
                }
                Instruction::LoadLocal(idx) => {
                    let value = self.frames[frame_idx]
                        .locals
                        .get(idx as usize)
                        .cloned()
                        .ok_or_else(|| VmError::InvalidInstruction(format!("local {idx}")))?;
                    self.stack.push(value, self.limits.max_stack)?;
                }
                Instruction::StoreLocal(idx) => {
                    let value = self.stack.pop()?;
                    let Some(slot) = self.frames[frame_idx].locals.get_mut(idx as usize) else {
                        return Err(VmError::InvalidInstruction(format!("local {idx}")));
                    };
                    *slot = value;
                }
                Instruction::LoadField(idx) => self.load_field(idx as usize)?,
                Instruction::StoreField(idx) => self.store_field(idx as usize)?,
                Instruction::MakeStruct { struct_id, field_count } => {
                    self.make_struct(struct_id, field_count)?
                }
                Instruction::MakeTuple(count) => self.make_tuple(count)?,
                Instruction::MakeArray(count) => self.make_array(count)?,
                Instruction::LoadIndex => self.load_index()?,
                Instruction::StoreIndex => self.store_index()?,
                Instruction::Slice => {
                    return Err(VmError::InvalidInstruction(
                        "slice expects compiler support".into(),
                    ));
                }
                Instruction::EnumVariant { enum_id, variant_id } => {
                    self.stack.push(
                        Value::Enum { enum_id, discriminant: variant_id as i64, variant_id },
                        self.limits.max_stack,
                    )?;
                }
                Instruction::AddI32 => self.bin_i32(i32::checked_add, "add")?,
                Instruction::SubI32 => self.bin_i32(i32::checked_sub, "sub")?,
                Instruction::MulI32 => self.bin_i32(i32::checked_mul, "mul")?,
                Instruction::DivI32 => self.div_i32()?,
                Instruction::ModI32 => self.mod_i32()?,
                Instruction::AddF32 => self.bin_f32(|a, b| a + b)?,
                Instruction::SubF32 => self.bin_f32(|a, b| a - b)?,
                Instruction::MulF32 => self.bin_f32(|a, b| a * b)?,
                Instruction::DivF32 => self.bin_f32(|a, b| a / b)?,
                Instruction::Eq => self.compare(|a, b| a == b)?,
                Instruction::Ne => self.compare(|a, b| a != b)?,
                Instruction::Lt => self.cmp_i32(|a, b| a < b)?,
                Instruction::Le => self.cmp_i32(|a, b| a <= b)?,
                Instruction::Gt => self.cmp_i32(|a, b| a > b)?,
                Instruction::Ge => self.cmp_i32(|a, b| a >= b)?,
                Instruction::Not => {
                    let Value::Bool(value) = self.stack.pop()? else {
                        return Err(VmError::InvalidInstruction("not bool".into()));
                    };
                    self.stack.push(Value::Bool(!value), self.limits.max_stack)?;
                }
                Instruction::NegI32 => {
                    let Value::I32(value) = self.stack.pop()? else {
                        return Err(VmError::InvalidInstruction("neg i32".into()));
                    };
                    self.stack.push(Value::I32(-value), self.limits.max_stack)?;
                }
                Instruction::NegF32 => {
                    let Value::F32(value) = self.stack.pop()? else {
                        return Err(VmError::InvalidInstruction("neg f32".into()));
                    };
                    self.stack.push(Value::F32(-value), self.limits.max_stack)?;
                }
                Instruction::RefLocal { local, mutable } => {
                    self.stack.push(
                        Value::Ref(RefValue {
                            base: RefBase::Local { frame: frame_idx, local },
                            mutable,
                            type_id: 0,
                        }),
                        self.limits.max_stack,
                    )?;
                }
                Instruction::Jump(target) => self.frames[frame_idx].ip = target as usize,
                Instruction::JumpIfFalse(target) => {
                    let Value::Bool(value) = self.stack.pop()? else {
                        return Err(VmError::InvalidInstruction(
                            "jump condition must be bool".into(),
                        ));
                    };
                    if !value {
                        self.frames[frame_idx].ip = target as usize;
                    }
                }
                Instruction::Call { function, argc } => self.call(function, argc as usize)?,
                Instruction::CallExtern { function, argc } => {
                    self.call_extern(function, argc as usize)?
                }
                Instruction::Return => {
                    let value = self.stack.pop().unwrap_or(Value::Void);
                    self.frames.pop();
                    if self.frames.is_empty() {
                        return Ok(value);
                    }
                    self.stack.push(value, self.limits.max_stack)?;
                }
                Instruction::Pop => {
                    self.stack.pop()?;
                }
                Instruction::Dup => {
                    let value = self.stack.last()?.clone();
                    self.stack.push(value, self.limits.max_stack)?;
                }
            }
        }
    }

    fn fetch(&mut self, frame_idx: usize) -> Result<Instruction, VmError> {
        let frame = &mut self.frames[frame_idx];
        let function =
            self.module.functions.get(frame.function_id.index()).ok_or_else(|| {
                VmError::UnknownFunction(format!("function#{}", frame.function_id.0))
            })?;
        let instruction = function.instructions.get(frame.ip).cloned().ok_or_else(|| {
            VmError::InvalidInstruction("instruction pointer out of bounds".into())
        })?;
        frame.ip += 1;
        Ok(instruction)
    }

    fn constant(&mut self, frame_idx: usize, idx: u32) -> Result<Value, VmError> {
        let frame = &self.frames[frame_idx];
        let function = &self.module.functions[frame.function_id.index()];
        let constant = function
            .constants
            .get(idx as usize)
            .ok_or_else(|| VmError::InvalidInstruction(format!("constant {idx}")))?;
        match constant {
            Constant::Void => Ok(Value::Void),
            Constant::Bool(value) => Ok(Value::Bool(*value)),
            Constant::Char(value) => Ok(Value::Char(*value)),
            Constant::I32(value) => Ok(Value::I32(*value)),
            Constant::I64(value) => Ok(Value::I64(*value)),
            Constant::F32(value) => Ok(Value::F32(*value)),
            Constant::F64(value) => Ok(Value::F64(*value)),
            Constant::String(value) => {
                let id = self.heap.alloc_string(value.clone(), self.limits.max_heap_objects)?;
                Ok(Value::Str(id))
            }
            Constant::CString(value) => Ok(Value::CStr(value.clone())),
        }
    }

    fn make_struct(&mut self, struct_id: u32, field_count: u32) -> Result<(), VmError> {
        let mut fields = Vec::with_capacity(field_count as usize);
        for _ in 0..field_count {
            fields.push(self.stack.pop()?);
        }
        fields.reverse();
        self.stack.push(Value::Struct { type_id: struct_id, fields }, self.limits.max_stack)
    }

    fn make_tuple(&mut self, count: u32) -> Result<(), VmError> {
        let mut items = Vec::with_capacity(count as usize);
        for _ in 0..count {
            items.push(self.stack.pop()?);
        }
        items.reverse();
        self.stack.push(Value::Tuple(items), self.limits.max_stack)
    }

    fn make_array(&mut self, count: u32) -> Result<(), VmError> {
        let mut items = Vec::with_capacity(count as usize);
        for _ in 0..count {
            items.push(self.stack.pop()?);
        }
        items.reverse();
        let id = self.heap.alloc_array(items, self.limits.max_heap_objects)?;
        self.stack.push(Value::Array(id), self.limits.max_stack)
    }

    fn load_field(&mut self, idx: usize) -> Result<(), VmError> {
        let value = self.stack.pop()?;
        match value {
            Value::Struct { fields, .. } => {
                let field = fields.get(idx).cloned().ok_or(VmError::IndexOutOfBounds)?;
                self.stack.push(field, self.limits.max_stack)
            }
            Value::Ref(RefValue { base: RefBase::Local { frame, local }, .. }) => {
                let frame = self.frames.get(frame).ok_or(VmError::InvalidReturnValue)?;
                let Some(Value::Struct { fields, .. }) = frame.locals.get(local as usize) else {
                    return Err(VmError::InvalidInstruction(
                        "load field reference expects struct local".into(),
                    ));
                };
                let field = fields.get(idx).cloned().ok_or(VmError::IndexOutOfBounds)?;
                self.stack.push(field, self.limits.max_stack)
            }
            Value::Tuple(items) => {
                let item = items.get(idx).cloned().ok_or(VmError::IndexOutOfBounds)?;
                self.stack.push(item, self.limits.max_stack)
            }
            _ => Err(VmError::InvalidInstruction("load field expects aggregate".into())),
        }
    }

    fn store_field(&mut self, idx: usize) -> Result<(), VmError> {
        let value = self.stack.pop()?;
        let mut base = self.stack.pop()?;
        match &mut base {
            Value::Struct { fields, .. } => {
                let Some(field) = fields.get_mut(idx) else {
                    return Err(VmError::IndexOutOfBounds);
                };
                *field = value;
                self.stack.push(base, self.limits.max_stack)
            }
            Value::Ref(RefValue { base: RefBase::Local { frame, local }, mutable, .. }) => {
                if !*mutable {
                    return Err(VmError::InvalidInstruction(
                        "store field requires mutable reference".into(),
                    ));
                }
                let frame = self.frames.get_mut(*frame).ok_or(VmError::InvalidReturnValue)?;
                let Some(Value::Struct { fields, .. }) = frame.locals.get_mut(*local as usize)
                else {
                    return Err(VmError::InvalidInstruction(
                        "store field reference expects struct local".into(),
                    ));
                };
                let Some(field) = fields.get_mut(idx) else {
                    return Err(VmError::IndexOutOfBounds);
                };
                *field = value;
                self.stack.push(Value::Void, self.limits.max_stack)
            }
            Value::Tuple(items) => {
                let Some(item) = items.get_mut(idx) else {
                    return Err(VmError::IndexOutOfBounds);
                };
                *item = value;
                self.stack.push(base, self.limits.max_stack)
            }
            _ => Err(VmError::InvalidInstruction("store field expects aggregate".into())),
        }
    }

    fn load_index(&mut self) -> Result<(), VmError> {
        let index = self.pop_index()?;
        let base = self.stack.pop()?;
        match base {
            Value::Array(id) => {
                let value = self
                    .heap
                    .array(id)
                    .and_then(|items| items.get(index))
                    .cloned()
                    .ok_or(VmError::IndexOutOfBounds)?;
                self.stack.push(value, self.limits.max_stack)
            }
            Value::Tuple(items) => {
                let value = items.get(index).cloned().ok_or(VmError::IndexOutOfBounds)?;
                self.stack.push(value, self.limits.max_stack)
            }
            _ => Err(VmError::InvalidInstruction("index expects array or tuple".into())),
        }
    }

    fn store_index(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;
        let index = self.pop_index()?;
        let base = self.stack.pop()?;
        match base {
            Value::Array(id) => {
                let Some(items) = self.heap.array_mut(id) else {
                    return Err(VmError::IndexOutOfBounds);
                };
                let Some(slot) = items.get_mut(index) else {
                    return Err(VmError::IndexOutOfBounds);
                };
                *slot = value;
                self.stack.push(Value::Array(id), self.limits.max_stack)
            }
            _ => Err(VmError::InvalidInstruction("store index expects array".into())),
        }
    }

    fn pop_index(&mut self) -> Result<usize, VmError> {
        match self.stack.pop()? {
            Value::I32(value) if value >= 0 => Ok(value as usize),
            Value::U32(value) => Ok(value as usize),
            _ => Err(VmError::InvalidInstruction(
                "index must be unsigned or non-negative i32".into(),
            )),
        }
    }

    fn call(&mut self, function: FunctionId, argc: usize) -> Result<(), VmError> {
        if self.frames.len() >= self.limits.max_frames {
            return Err(VmError::FrameLimit);
        }
        let args = self.pop_args(argc)?;
        let function_ref = self
            .module
            .functions
            .get(function.index())
            .ok_or_else(|| VmError::UnknownFunction(format!("function#{}", function.0)))?;
        let frame = make_frame(function_ref, args)?;
        self.frames.push(frame);
        Ok(())
    }

    fn call_extern(&mut self, function: si_bytecode::ExternId, argc: usize) -> Result<(), VmError> {
        let args = self.pop_args(argc)?;
        let value = self.externs.call(function, args)?;
        self.stack.push(value, self.limits.max_stack)
    }

    fn pop_args(&mut self, argc: usize) -> Result<Vec<Value>, VmError> {
        let mut args = Vec::with_capacity(argc);
        for _ in 0..argc {
            args.push(self.stack.pop()?);
        }
        args.reverse();
        Ok(args)
    }

    fn bin_i32(&mut self, op: fn(i32, i32) -> Option<i32>, name: &str) -> Result<(), VmError> {
        let (left, right) = self.pop_i32_pair()?;
        let value = op(left, right).ok_or_else(|| VmError::InvalidInstruction(name.into()))?;
        self.stack.push(Value::I32(value), self.limits.max_stack)
    }

    fn div_i32(&mut self) -> Result<(), VmError> {
        let (left, right) = self.pop_i32_pair()?;
        if right == 0 {
            return Err(VmError::DivisionByZero);
        }
        self.stack.push(Value::I32(left / right), self.limits.max_stack)
    }

    fn mod_i32(&mut self) -> Result<(), VmError> {
        let (left, right) = self.pop_i32_pair()?;
        if right == 0 {
            return Err(VmError::DivisionByZero);
        }
        self.stack.push(Value::I32(left % right), self.limits.max_stack)
    }

    fn bin_f32(&mut self, op: fn(f32, f32) -> f32) -> Result<(), VmError> {
        let right = self.stack.pop()?;
        let left = self.stack.pop()?;
        let (Value::F32(left), Value::F32(right)) = (left, right) else {
            return Err(VmError::InvalidInstruction("f32 operands".into()));
        };
        self.stack.push(Value::F32(op(left, right)), self.limits.max_stack)
    }

    fn cmp_i32(&mut self, op: fn(i32, i32) -> bool) -> Result<(), VmError> {
        let (left, right) = self.pop_i32_pair()?;
        self.stack.push(Value::Bool(op(left, right)), self.limits.max_stack)
    }

    fn compare(&mut self, op: fn(Value, Value) -> bool) -> Result<(), VmError> {
        let right = self.stack.pop()?;
        let left = self.stack.pop()?;
        self.stack.push(Value::Bool(op(left, right)), self.limits.max_stack)
    }

    fn pop_i32_pair(&mut self) -> Result<(i32, i32), VmError> {
        let right = self.stack.pop()?;
        let left = self.stack.pop()?;
        let (Value::I32(left), Value::I32(right)) = (left, right) else {
            return Err(VmError::InvalidInstruction("i32 operands".into()));
        };
        Ok((left, right))
    }

    fn check_instruction_limit(&mut self) -> Result<(), VmError> {
        self.instruction_count += 1;
        if let Some(max) = self.limits.max_instructions
            && self.instruction_count > max
        {
            return Err(VmError::InstructionLimit);
        }
        Ok(())
    }
}
