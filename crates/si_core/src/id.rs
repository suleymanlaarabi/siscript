#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FileId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct NodeId(u32);

impl FileId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl NodeId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_id_is_a_tiny_copyable_handle() {
        let id = FileId::new(11);

        assert_eq!(id.get(), 11);
        assert_eq!(FileId::default(), FileId::new(0));
        assert_eq!(id, FileId::new(11));
    }

    #[test]
    fn node_id_is_a_tiny_copyable_handle() {
        let id = NodeId::new(27);

        assert_eq!(id.get(), 27);
        assert_eq!(NodeId::default(), NodeId::new(0));
        assert_eq!(id, NodeId::new(27));
    }
}
