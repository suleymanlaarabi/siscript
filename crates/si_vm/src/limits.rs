#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmLimits {
    pub max_stack: usize,
    pub max_frames: usize,
    pub max_heap_objects: usize,
    pub max_instructions: Option<usize>,
}

impl Default for VmLimits {
    fn default() -> Self {
        Self {
            max_stack: 1024,
            max_frames: 64,
            max_heap_objects: 1024,
            max_instructions: Some(100_000),
        }
    }
}
