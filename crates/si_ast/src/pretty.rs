#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PrettyOptions {
    pub indent_width: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_options_are_data_only() {
        let options = PrettyOptions { indent_width: 4 };

        assert_eq!(options.indent_width, 4);
    }
}
