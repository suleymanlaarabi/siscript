[
  "struct"
  "enum"
  "type"
  "const"
  "fn"
  "export"
  "extern"
  "let"
  "return"
  "if"
  "else"
  "while"
  "for"
  "in"
  "match"
  "break"
  "continue"
] @keyword

(mutable_specifier) @keyword

[
  "->"
  "=>"
  "::"
  "."
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  "+"
  "-"
  "*"
  "/"
  "%"
  "=="
  "!="
  "<"
  "<="
  ">"
  ">="
  "&&"
  "||"
  "!"
  "&"
  ".."
] @operator

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

[
  ","
  ":"
  ";"
] @punctuation.delimiter

(line_comment) @comment
(block_comment) @comment

(number_literal) @number
(string_literal) @string
(cstring_literal) @string.special
(char_literal) @string
(boolean_literal) @constant.builtin

(primitive_type) @type.builtin
(type_identifier) @type

(struct_item name: (type_identifier) @type)
(enum_item name: (type_identifier) @enum)
(enum_variant name: (type_identifier) @variant)
(type_item name: (type_identifier) @type)
(struct_expression type: (type_identifier) @constructor)
(scoped_identifier path: (type_identifier) @type name: (type_identifier) @variant)

(function_item name: (identifier) @function)
(export_function_item name: (identifier) @function)
(extern_function_item name: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (scoped_identifier name: (type_identifier) @function.method))

(parameter name: (identifier) @variable.parameter)
(self_parameter) @variable.special
((identifier) @variable.special
  (#eq? @variable.special "self"))

(field_declaration name: (field_identifier) @property)
(field_initializer field: (field_identifier) @property)
(field_expression field: (field_identifier) @property)

(call_expression function: (field_expression field: (field_identifier) @function.method))

(let_statement pattern: (identifier) @variable)
(for_statement name: (identifier) @variable)
(assignment_expression left: (identifier) @variable)
(assignment_expression right: (identifier) @variable)
(binary_expression left: (identifier) @variable)
(binary_expression right: (identifier) @variable)
(unary_expression argument: (identifier) @variable)
(field_expression object: (identifier) @variable)
