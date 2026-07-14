; ── Keywords ──────────────────────────────────────────────────────────

"as" @keyword
"async" @keyword
"await" @keyword
"break" @keyword
"const" @keyword
"continue" @keyword
"dyn" @keyword
"else" @keyword
"enum" @keyword
"extern" @keyword
"fn" @keyword
"for" @keyword
"if" @keyword
"impl" @keyword
"in" @keyword
"let" @keyword
"loop" @keyword
"match" @keyword
"mod" @keyword
"move" @keyword
"pub" @keyword
"ref" @keyword
"return" @keyword
"static" @keyword
"struct" @keyword
"trait" @keyword
"type" @keyword
"unsafe" @keyword
"use" @keyword
"where" @keyword
"while" @keyword

; ── Types ─────────────────────────────────────────────────────────────

(type_identifier) @type
(primitive_type) @type.builtin

; ── Functions ─────────────────────────────────────────────────────────

(function_item name: (identifier) @function)
(function_signature_item name: (identifier) @function)

(call_expression
  function: (identifier) @function.call)
(call_expression
  function: (field_expression
    field: (field_identifier) @function.method))
(call_expression
  function: (scoped_identifier
    "::"
    name: (identifier) @function.call))

(generic_function
  function: (identifier) @function.call)
(generic_function
  function: (scoped_identifier
    name: (identifier) @function.call))
(generic_function
  function: (field_expression
    field: (field_identifier) @function.method))

; ── Macros ────────────────────────────────────────────────────────────

(macro_invocation
  macro: (identifier) @function.macro
  "!" @function.macro)

; ── Strings ───────────────────────────────────────────────────────────

(string_literal) @string
(raw_string_literal) @string
(char_literal) @string

; ── Numbers & booleans ────────────────────────────────────────────────

(integer_literal) @number
(float_literal) @number.float
(boolean_literal) @constant.builtin.boolean

; ── Comments ──────────────────────────────────────────────────────────

(line_comment) @comment
(block_comment) @comment
(line_comment (doc_comment)) @comment.documentation
(block_comment (doc_comment)) @comment.documentation

; ── Attributes ────────────────────────────────────────────────────────

(attribute_item) @attribute
(inner_attribute_item) @attribute

; ── Operators ─────────────────────────────────────────────────────────

"=" @operator
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"%" @operator
"->" @operator
"=>" @operator
".." @operator
"..=" @operator
"?" @operator
"@" @operator
"!" @operator
"&" @operator
"&&" @operator
"|" @operator
"||" @operator
"==" @operator
"!=" @operator
"<" @operator
">" @operator
"<=" @operator
">=" @operator
"+=" @operator
"-=" @operator
"*=" @operator
"/=" @operator
"%=" @operator
"&=" @operator
"|=" @operator
"^=" @operator
">>=" @operator
"<<=" @operator
"^" @operator
"<<" @operator
">>" @operator

; ── Punctuation ───────────────────────────────────────────────────────

"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"," @punctuation.delimiter
";" @punctuation.delimiter
":" @punctuation.delimiter
"." @punctuation.delimiter
"::" @punctuation.delimiter

; ── Properties / fields ───────────────────────────────────────────────

(field_identifier) @property

; ── Parameters ────────────────────────────────────────────────────────

(parameter pattern: (identifier) @parameter)

; ── Constants ─────────────────────────────────────────────────────────

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z0-9_]*$"))

; ── Constructors (enum variants) ──────────────────────────────────────

((identifier) @constructor
 (#match? @constructor "^[A-Z][a-z]"))

; ── Struct/enum/trait definitions ─────────────────────────────────────

(struct_item name: (type_identifier) @type)
(enum_item name: (type_identifier) @type)
(enum_variant name: (identifier) @constructor)
(trait_item name: (type_identifier) @type)

; ── Namespace / module paths ──────────────────────────────────────────

(scoped_identifier
  path: (identifier) @namespace)
(scoped_identifier
  path: (scoped_identifier
    name: (identifier) @namespace))
(scoped_type_identifier
  path: (identifier) @namespace)
(scoped_type_identifier
  path: (scoped_identifier
    name: (identifier) @namespace))

(use_declaration argument: (scoped_identifier) @namespace)
(use_declaration argument: (identifier) @namespace)
(use_declaration argument: (use_as_clause path: (identifier) @namespace))

(mod_item name: (identifier) @namespace)

; ── Lifetimes ─────────────────────────────────────────────────────────

(lifetime (identifier) @lifetime)

; ── self / Self ───────────────────────────────────────────────────────

(self) @variable.builtin

; ── Escape sequences ──────────────────────────────────────────────────

(escape_sequence) @escape

; ── Struct patterns ───────────────────────────────────────────────────

(struct_pattern
  type: (scoped_type_identifier
    name: (type_identifier) @constructor))
