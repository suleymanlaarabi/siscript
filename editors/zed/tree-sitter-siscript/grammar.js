const PREC = {
  assign: 1,
  range: 2,
  logical_or: 3,
  logical_and: 4,
  equality: 5,
  comparison: 6,
  additive: 7,
  multiplicative: 8,
  unary: 9,
  call: 10,
};

module.exports = grammar({
  name: "siscript",

  extras: ($) => [/\s/, $.line_comment, $.block_comment],

  conflicts: ($) => [
    [$._expression, $.type_identifier],
    [$.tuple_expression, $.parenthesized_expression],
    [$.tuple_type, $.parenthesized_type],
  ],

  rules: {
    source_file: ($) => repeat($._item),

    _item: ($) =>
      choice(
        $.struct_item,
        $.enum_item,
        $.type_item,
        $.const_item,
        $.function_item,
        $.export_function_item,
        $.extern_function_item,
      ),

    struct_item: ($) =>
      seq("struct", field("name", $.type_identifier), $.struct_body),

    struct_body: ($) =>
      seq("{", repeat(choice($.field_declaration, $.function_item)), "}"),

    field_declaration: ($) =>
      seq(
        field("name", $.field_identifier),
        ":",
        field("type", $._type),
        optional(seq("=", field("default", $._expression))),
        optional(","),
      ),

    enum_item: ($) =>
      seq(
        "enum",
        field("name", $.type_identifier),
        optional(seq(":", field("repr", $._type))),
        "{",
        repeat($.enum_variant),
        "}",
      ),

    enum_variant: ($) =>
      seq(
        field("name", $.type_identifier),
        optional(seq("=", field("value", $._expression))),
        optional(","),
      ),

    type_item: ($) =>
      seq(
        "type",
        field("name", $.type_identifier),
        "=",
        field("type", $._type),
        optional(";"),
      ),

    const_item: ($) =>
      seq(
        "const",
        field("name", $.identifier),
        ":",
        field("type", $._type),
        "=",
        field("value", $._expression),
        optional(";"),
      ),

    function_item: ($) =>
      seq(
        "fn",
        field("name", $.identifier),
        $.parameters,
        optional($.return_type),
        $.block,
      ),

    export_function_item: ($) =>
      seq(
        "export",
        "fn",
        field("name", $.identifier),
        $.parameters,
        optional($.return_type),
        $.block,
      ),

    extern_function_item: ($) =>
      seq(
        "extern",
        "fn",
        field("name", $.identifier),
        $.parameters,
        optional($.return_type),
        optional(";"),
      ),

    parameters: ($) =>
      seq("(", optional(seq(commaSep1($.parameter), optional(","))), ")"),

    parameter: ($) =>
      choice(
        $.self_parameter,
        seq(
          optional($.mutable_specifier),
          field("name", $.identifier),
          ":",
          field("type", $._type),
        ),
      ),

    self_parameter: ($) =>
      seq(optional(seq("&", optional($.mutable_specifier))), "self"),

    mutable_specifier: () => "mut",

    return_type: ($) => seq("->", field("type", $._type)),

    _type: ($) =>
      choice(
        $.reference_type,
        $.slice_type,
        $.array_type,
        $.tuple_type,
        $.parenthesized_type,
        $.primitive_type,
        $.type_identifier,
      ),

    reference_type: ($) => seq("&", optional($.mutable_specifier), $._type),
    slice_type: ($) =>
      seq("&", optional($.mutable_specifier), "[", $._type, "]"),
    array_type: ($) =>
      prec(1, seq(choice($.primitive_type, $.type_identifier), "[", "]")),
    tuple_type: ($) => seq("(", commaSep1($._type), optional(","), ")"),
    parenthesized_type: ($) => seq("(", $._type, ")"),

    primitive_type: () =>
      choice(
        "i8",
        "i16",
        "i32",
        "i64",
        "u8",
        "u16",
        "u32",
        "u64",
        "f32",
        "f64",
        "bool",
        "char",
        "void",
        "str",
        "cstr",
      ),

    block: ($) => seq("{", repeat($._statement), "}"),

    _statement: ($) =>
      choice(
        $.let_statement,
        $.return_statement,
        $.while_statement,
        $.for_statement,
        $.break_statement,
        $.continue_statement,
        $.expression_statement,
      ),

    let_statement: ($) =>
      seq(
        "let",
        optional($.mutable_specifier),
        field("pattern", choice($.identifier, $.tuple_pattern)),
        optional(seq(":", field("type", $._type))),
        optional(seq("=", field("value", $._expression))),
        optional(";"),
      ),

    tuple_pattern: ($) =>
      seq(
        "(",
        commaSep1(choice($.identifier, $.tuple_pattern)),
        optional(","),
        ")",
      ),

    return_statement: ($) =>
      prec.right(seq("return", optional($._expression), optional(";"))),
    break_statement: () => seq("break", optional(";")),
    continue_statement: () => seq("continue", optional(";")),
    while_statement: ($) =>
      seq("while", field("condition", $._expression), field("body", $.block)),
    for_statement: ($) =>
      seq(
        "for",
        field("name", $.identifier),
        "in",
        field("iterable", $._expression),
        field("body", $.block),
      ),

    expression_statement: ($) => seq($._expression, optional(";")),

    _expression: ($) =>
      choice(
        $.assignment_expression,
        $.binary_expression,
        $.unary_expression,
        $.call_expression,
        $.field_expression,
        $.index_expression,
        $.struct_expression,
        $.scoped_identifier,
        $.if_expression,
        $.match_expression,
        $.array_expression,
        $.tuple_expression,
        $.parenthesized_expression,
        $.identifier,
        $.number_literal,
        $.string_literal,
        $.cstring_literal,
        $.char_literal,
        $.boolean_literal,
      ),

    assignment_expression: ($) =>
      prec.right(
        PREC.assign,
        seq(
          field(
            "left",
            choice($.identifier, $.field_expression, $.index_expression),
          ),
          field("operator", choice("=", "+=", "-=", "*=", "/=", "%=")),
          field("right", $._expression),
        ),
      ),

    binary_expression: ($) => {
      const table = [
        [PREC.range, ".."],
        [PREC.logical_or, "||"],
        [PREC.logical_and, "&&"],
        [PREC.equality, choice("==", "!=")],
        [PREC.comparison, choice("<", "<=", ">", ">=")],
        [PREC.additive, choice("+", "-")],
        [PREC.multiplicative, choice("*", "/", "%")],
      ];
      return choice(
        ...table.map(([precedence, operator]) =>
          prec.left(
            precedence,
            seq(
              field("left", $._expression),
              field("operator", operator),
              field("right", $._expression),
            ),
          ),
        ),
      );
    },

    unary_expression: ($) =>
      prec(
        PREC.unary,
        seq(
          field(
            "operator",
            choice("!", "-", "&", seq("&", $.mutable_specifier), "*"),
          ),
          field("argument", $._expression),
        ),
      ),

    call_expression: ($) =>
      prec(
        PREC.call,
        seq(
          field(
            "function",
            choice($.identifier, $.field_expression, $.scoped_identifier),
          ),
          $.arguments,
        ),
      ),

    arguments: ($) =>
      seq("(", optional(seq(commaSep1($._expression), optional(","))), ")"),

    field_expression: ($) =>
      prec.left(
        PREC.call,
        seq(
          field("object", $._expression),
          ".",
          field("field", choice($.field_identifier, $.number_literal)),
        ),
      ),

    index_expression: ($) =>
      prec.left(
        PREC.call,
        seq(
          field("object", $._expression),
          "[",
          field("index", optional($._expression)),
          optional(seq("..", optional($._expression))),
          "]",
        ),
      ),

    scoped_identifier: ($) =>
      seq(
        field("path", $.type_identifier),
        "::",
        field("name", $.type_identifier),
      ),

    struct_expression: ($) =>
      seq(
        field("type", $.type_identifier),
        "{",
        optional(seq(commaSep1($.field_initializer), optional(","))),
        "}",
      ),

    field_initializer: ($) =>
      seq(
        field("field", $.field_identifier),
        ":",
        field("value", $._expression),
      ),

    if_expression: ($) =>
      seq(
        "if",
        field("condition", $._expression),
        field("consequence", $.block),
        optional(
          seq("else", field("alternative", choice($.block, $.if_expression))),
        ),
      ),

    match_expression: ($) =>
      seq(
        "match",
        field("value", $._expression),
        "{",
        repeat($.match_arm),
        "}",
      ),

    match_arm: ($) =>
      seq(
        field("pattern", $._expression),
        "=>",
        field("value", $._expression),
        optional(","),
      ),

    array_expression: ($) =>
      seq("[", optional(seq(commaSep1($._expression), optional(","))), "]"),
    tuple_expression: ($) =>
      seq("(", commaSep1($._expression), optional(","), ")"),
    parenthesized_expression: ($) => seq("(", $._expression, ")"),

    boolean_literal: () => choice("true", "false"),
    number_literal: () => /\d+(\.\d+)?/,
    string_literal: () =>
      seq('"', repeat(choice(token.immediate(/[^"\\]+/), /\\./)), '"'),
    cstring_literal: () =>
      seq(
        "c",
        seq('"', repeat(choice(token.immediate(/[^"\\]+/), /\\./)), '"'),
      ),
    char_literal: () => seq("'", choice(/[^'\\]/, /\\./), "'"),

    identifier: () => /[A-Za-z_][A-Za-z0-9_]*/,
    type_identifier: ($) => alias($.identifier, $.type_identifier),
    field_identifier: ($) => alias($.identifier, $.field_identifier),

    line_comment: () => token(seq("//", /.*/)),
    block_comment: () => token(seq("/*", /[^*]*\*+([^/*][^*]*\*+)*/, "/")),
  },
});

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}
