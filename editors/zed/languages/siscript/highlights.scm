; Keywords
"fn" @keyword
"let" @keyword
"struct" @keyword
"enum" @keyword
"impl" @keyword
"if" @keyword
"else" @keyword
"match" @keyword
"return" @keyword
"while" @keyword
"for" @keyword
"in" @keyword
"const" @keyword
"extern" @keyword

(mutable_specifier) @keyword

; Literals
(integer_literal) @number
(float_literal) @number
(string_literal) @string
(char_literal) @string
(boolean_literal) @constant.builtin

; Comments
(line_comment) @comment
(block_comment) @comment

; Types
(type_identifier) @type
(primitive_type) @type.builtin

; Functions
(function_item name: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (field_expression field: (field_identifier) @function))

; Identifiers
(identifier) @variable
