#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct Symbol(u32);

impl Symbol {
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
    fn symbol_is_a_stable_numeric_handle() {
        let symbol = Symbol::new(99);

        assert_eq!(symbol.get(), 99);
        assert_eq!(Symbol::default(), Symbol::new(0));
        assert_eq!(symbol, Symbol::new(99));
    }
}
