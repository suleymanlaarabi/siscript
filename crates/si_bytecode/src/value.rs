#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HeapId(pub u32);

impl HeapId {
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Void,
    Bool(bool),
    Char(u32),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    CStr(String),
    Str(HeapId),
    Array(HeapId),
    Struct { type_id: u32, fields: Vec<Value> },
    Enum { enum_id: u32, discriminant: i64, variant_id: u32 },
    Tuple(Vec<Value>),
    Ref(RefValue),
    Slice(SliceValue),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefValue {
    pub base: RefBase,
    pub mutable: bool,
    pub type_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefBase {
    Local { frame: usize, local: u32 },
    Heap { heap_id: HeapId, index: Option<usize> },
    HostPtr(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SliceValue {
    pub heap_id: HeapId,
    pub start: usize,
    pub len: usize,
}

impl Eq for Value {}
impl Eq for RefValue {}
impl Eq for RefBase {}
impl Eq for SliceValue {}
