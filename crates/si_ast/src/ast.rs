#![forbid(unsafe_code)]

use crate::item::Item;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ast {
    pub items: Vec<Item>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ast_can_hold_top_level_items() {
        let ast = Ast { items: Vec::new() };

        assert!(ast.items.is_empty());
    }
}
