; Taken from https://github.com/zed-industries/zed/blob/f2ab00cec7545ffb7d8d75e4ccab74d5fccccf9b/crates/languages/src/yaml/highlights.scm
; as the "Docker Compose" language shares the grammar zed uses for YAML
(boolean_scalar) @boolean
(null_scalar) @constant.builtin

[
  (double_quote_scalar)
  (single_quote_scalar)
  (block_scalar)
  (string_scalar)
] @string

(escape_sequence) @string.escape

[
  (integer_scalar)
  (float_scalar)
] @number

(comment) @comment

[
  (anchor_name)
  (alias_name)
  (tag)
] @type

key: (flow_node (plain_scalar (string_scalar) @property))

[
  ","
  "-"
  ":"
  ">"
  "?"
  "|"
] @punctuation.delimiter

[
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  "*"
  "&"
  "---"
  "..."
] @punctuation.special
