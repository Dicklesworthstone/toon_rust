# Existing TOON Structure and Architecture

> Comprehensive specification of the TypeScript reference implementation for porting to Rust.
> This document is the **single source of truth** for the Rust port.
> After reading this document, you should NOT need to consult legacy code.

---

## Table of Contents

0. [Working TODO (Keep Current)](#0-working-todo-keep-current)
1. [Project Overview](#1-project-overview)
2. [Directory Structure](#2-directory-structure)
3. [Data Types and Models](#3-data-types-and-models)
4. [Encoding Pipeline (JSON -> TOON)](#4-encoding-pipeline-json---toon)
5. [Key Folding (safe)](#5-key-folding-safe)
6. [Decoding Pipeline (TOON -> JSON)](#6-decoding-pipeline-toon---json)
7. [CLI Commands Specification](#7-cli-commands-specification)
8. [Configuration and Defaults](#8-configuration-and-defaults)
9. [Validation Rules (Strict vs Non-Strict)](#9-validation-rules-strict-vs-non-strict)
10. [Error Handling](#10-error-handling)
11. [Porting Considerations](#11-porting-considerations)

---

## 0. Working TODO (Keep Current)

This is the **live checklist** for completing spec extraction. Update as new findings appear.

### Completed (this session)

- [x] Captured JSON model + encoder/decoder option types.
- [x] Documented normalization rules (toJSON, BigInt, Date, Set/Map).
- [x] Documented string quoting/escaping and key validation rules.
- [x] Documented array header format and delimiter behavior.
- [x] Documented encode array format selection (tabular vs list vs inline).
- [x] Documented streaming decode event model and root detection rules.
- [x] Documented strict mode validation (counts, blank lines, indentation).
- [x] Documented path expansion rules and quoted-key suppression.
- [x] Documented CLI flags, defaults, and validation.
- [x] Captured exact CLI success/error wording from `packages/cli/test/`.
- [x] Located spec fixtures under `legacy_spec/tests/fixtures`.
- [x] Vendored spec fixtures to `tests/fixtures/spec`.
- [x] Audited reference tests for edge-case behaviors (quoted dotted keys in array headers/fields).

### Remaining

- [ ] (none)

---

## 1. Project Overview

TOON is a token-efficient, human-readable encoding of the JSON data model.
The reference implementation provides:

- **Encoding**: JSON -> TOON lines (streaming)
- **Decoding**: TOON -> JSON (streaming event model or full materialization)
- **Optional features**:
  - Key folding (safe)
  - Path expansion (safe)
  - Strict validation (decode)

The Rust port must preserve:

- Byte-for-byte equivalence of encoded output
- Event stream semantics during decode
- Strict vs non-strict validation behavior
- CLI behavior and defaults

---

## 2. Directory Structure

Reference layout (TypeScript):

```
legacy_toon/
  packages/
    toon/
      src/
        constants.ts
        types.ts
        index.ts
        encode/
          encoders.ts
          folding.ts
          normalize.ts
          primitives.ts
          replacer.ts
        decode/
          decoders.ts
          event-builder.ts
          expand.ts
          parser.ts
          scanner.ts
          validation.ts
        shared/
          literal-utils.ts
          string-utils.ts
          validation.ts
      test/
        encode.test.ts
        encodeLines.test.ts
        decode.test.ts
        decodeStream.test.ts
        decodeStreamAsync.test.ts
        normalization.test.ts
        replacer.test.ts
    cli/
      src/
        index.ts
        conversion.ts
        utils.ts
        json-from-events.ts
        json-stringify-stream.ts
      test/
        ...
```

Rust port should mirror these conceptual modules (not filenames):

- `encode`: normalization, primitive formatting, array/object encoding, folding, replacer
- `decode`: scanner, parser, event stream, expand paths, validation
- `cli`: input/output, streaming json, stats

---

## 3. Data Types and Models

### JSON Model

- `JsonPrimitive = string | number | boolean | null`
- `JsonObject = { [key: string]: JsonValue }`
- `JsonArray = JsonValue[]`
- `JsonValue = JsonPrimitive | JsonObject | JsonArray`

### Encoder Options

`EncodeOptions` (defaults in brackets):

- `indent?: number` [2]
- `delimiter?: Delimiter` [comma]
- `keyFolding?: 'off' | 'safe'` ['off']
- `flattenDepth?: number` [Infinity]
- `replacer?: EncodeReplacer` [undefined]

`EncodeReplacer`:

```
(key: string, value: JsonValue, path: readonly (string|number)[]) => unknown
```

Rules:
- Called on root with `key=''`, `path=[]`.
- Returning `undefined` for root means "no change", not removal.
- Returning `undefined` for child nodes omits the property/element.
- Returned values are normalized again.

### Decoder Options

`DecodeOptions` (defaults in brackets):

- `indent?: number` [2]
- `strict?: boolean` [true]
- `expandPaths?: 'off' | 'safe'` ['off']

`DecodeStreamOptions`:

- Same as DecodeOptions but `expandPaths` is forbidden.
- Attempting `expandPaths` in streaming mode throws an error.

### Streaming Event Model

`JsonStreamEvent`:

- `{ type: 'startObject' }`
- `{ type: 'endObject' }`
- `{ type: 'startArray', length: number }`
- `{ type: 'endArray' }`
- `{ type: 'key', key: string, wasQuoted?: boolean }`
- `{ type: 'primitive', value: JsonPrimitive }`

### Parsing Structures

`ArrayHeaderInfo`:

- `key?: string`
- `length: number`
- `delimiter: Delimiter`
- `fields?: string[]` (tabular arrays)

`ParsedLine`:

- `raw`, `indent`, `content`, `depth`, `lineNumber`

---

## 4. Encoding Pipeline (JSON -> TOON)

### 4.1 Normalization (unknown -> JsonValue)

Rules, in order:

1. **null** -> `null`
2. **toJSON**: if object has `toJSON()`:
   - call it
   - if result != original object, normalize the result
3. **string/boolean** -> as-is
4. **number**:
   - `-0` normalized to `0`
   - `NaN` / `Infinity` -> `null`
5. **bigint**:
   - if within `Number.MIN_SAFE_INTEGER..Number.MAX_SAFE_INTEGER`, cast to number
   - else convert to string
6. **Date** -> ISO string
7. **Array** -> element-wise normalize
8. **Set** -> array of normalized values
9. **Map** -> object with `String(key)` and normalized values
10. **Plain object** -> normalize each own property
11. **Other (function, symbol, undefined, non-plain object)** -> `null`

Additional normalization edge cases (from tests):

- Empty Map -> encodes as empty object (encode() returns empty string).
- Map with numeric keys -> keys are stringified and quoted as needed.
- Empty Set -> encodes as empty array header `[0]:`.
- `toJSON` is applied before replacer:
  - The result of `toJSON` is normalized before replacer sees it.
  - If `toJSON` returns `undefined`, it normalizes to `null`.
  - `toJSON` can be inherited via prototype.
  - `toJSON` takes precedence even if object appears to be a Date.

### 4.2 Primitive Encoding

`encodePrimitive(value)`:

- `null` -> `"null"`
- `boolean` -> `"true" | "false"`
- `number` -> canonical decimal (spec §2): the shortest digits that read back as the same
  double (an exact tie goes to the even digit, as in `String(value)`), but never an exponent
  (`1e21` -> `1000000000000000000000`), `-0` -> `0`
- `string` -> `encodeStringLiteral(value)`

`encodeStringLiteral`:

- If `isSafeUnquoted(value, delimiter)` -> raw string
- Else -> quoted with escaping (`"..."`)

**Escapes:**
`\\`, `\"`, `\n`, `\r`, `\t`

### 4.3 Key Encoding

`encodeKey(key)`:

- If `isValidUnquotedKey(key)` -> raw
- Else quoted with escaping

`isValidUnquotedKey`:

- Regex: `^[A-Z_][\\w.]*$` (case-insensitive)
- Allows dots in unquoted keys (but see folding/expansion rules)

### 4.4 Array Header Format

`formatHeader(length, options)` -> string:

```
[<len><delimiter-suffix>]{<fields>}:   (key optional)
```

Rules:

- If `key` provided, header prefix is `encodeKey(key)`
- Always include `[len]`
- Append delimiter char inside brackets only if not comma (default)
  - tab uses literal `\t`
  - pipe uses `|`
- If `fields` present, add `{field1,field2,...}` (encoded as keys)
- Always ends with `:`

Examples:

- `items[3]:`
- `items[3|]:` (pipe delimiter)
- `items[3]{id,name}:`
- `[0]:` (anonymous array)

### 4.5 Array Encoding Strategy

Given array `value`:

1. **Empty array**:
   - header only: `[0]:` (key optional)

2. **Primitive array** (all primitives or empty):
   - Inline: `key[3]: a,b,c`
   - Values joined with delimiter using primitive encoding

3. **Array of arrays** (all arrays, each is primitive-only):
   - Use list format:
     - header `key[3]:`
     - each row as list item `- [3]: a,b,c`

4. **Array of objects**:
   - If tabular eligible:
     - header with fields `{...}` and length
     - rows as delimited primitives
   - Else:
     - list format with `-` items

5. **Mixed array** (fallback):
   - list format with `-` items

### 4.6 Tabular Array Eligibility

`isTabularArray(rows, header)` returns true if:

- Each object has exactly the same keys (order can differ)
- All keys in `header` exist in each row
- All values for those keys are primitives

Header is derived from the **first row's** key order.

### 4.7 List Item Encoding (Expanded)

List items are prefixed with `- `.

If a list item is an object:

- Empty object: `-`
- Otherwise, encode first field inline:
  - `- key: value`
  - `- key[3]: ...`
  - `- key[3]{fields}:` (tabular array as first field)
  - `- key:` (object)
- Remaining fields encoded on following lines at depth+1.

### 4.8 Indentation

Indentation uses **spaces** only.

`indentSize` is from `EncodeOptions.indent`.
Each depth level -> `indentSize * depth` spaces.

### 4.9 encodeLines Behavior

- `encodeLines` yields lines **without newline characters**.
- Empty object -> **zero** lines (empty iterator).
- `encode({})` returns empty string.
- No trailing spaces in any line.
- Line order preserves object key order as in input object iteration.
- The number of lines equals the number of top-level object keys.

### 4.10 Replacer Semantics

The replacer runs after normalization and before encoding:

- Root call:
  - `key = ""`
  - `path = []`
  - Returning `undefined` does **not** omit the root (treated as "no change").
- Child nodes:
  - Returning `undefined` omits the property or array element.
  - Array indices are passed as **string** keys ("0", "1", "2") to match `JSON.stringify`.
- The replacer sees **normalized** values:
  - `toJSON` has already been applied.
  - Dates are already ISO strings.
- If the replacer returns a non-JsonValue (e.g., Date), it is normalized again.
- If all object properties are filtered, result is an empty object (encode -> empty string).
- If all array elements are filtered, result is an empty array (encode -> `[0]:` for root).

---

## 5. Key Folding (safe)

Key folding collapses chains of single-key objects into dotted keys.

### Preconditions

- `keyFolding` must be `'safe'`
- Value must be an object
- Chain must include **at least 2 segments**
- Each segment must satisfy `isIdentifierSegment` (no dots)
  - Regex: `^[A-Z_]\\w*$` (case-insensitive)

### Depth Limit

`flattenDepth` caps the number of segments folded.
If limit reached, folding stops and remainder is encoded normally.

### Collision Rules

Folding is rejected if:

- Folded key collides with any sibling key at the same level
- Folded key collides with any **literal dotted key** at root

Root dotted key detection:

- At depth 0, collect all literal keys containing `.`.
- Folding into any of those absolute paths is forbidden.

### Outputs

Folding produces one of:

- **Fully folded**: leaf is primitive/array/empty object, encoded at current depth.
- **Partially folded**: remainder object encoded with reduced depth budget.

---

## 6. Decoding Pipeline (TOON -> JSON)

### 6.1 Scanning and Line Parsing

Each input line is parsed into:

- `indent` (count of leading spaces)
- `content` (line without leading spaces)
- `depth = floor(indent / indentSize)`
- `lineNumber` (1-based)

Blank lines:
- Captured with their lineNumber, indent, and depth.
- Used for strict-mode validation (no blank lines inside arrays).

Strict mode indentation rules:
- Tabs are forbidden in indentation (leading whitespace).
- Indentation must be a **multiple of indentSize**.

### 6.2 Array Header Parsing

Array header format:

```
<optional-key>[<length><optional-delimiter>]{<optional-fields>}:
```

Rules:

- Supports quoted keys (find bracket after closing quote).
- If `[]` length is invalid, parsing fails.
- Delimiter suffix rules:
  - default delimiter is comma
  - if bracket content ends with tab or pipe, delimiter overrides
- `fields` (tabular arrays) are parsed as delimited values and each field is parsed as a string literal.

### 6.3 Primitive Parsing

`parsePrimitiveToken(token)`:

- Empty token -> `""`
- If starts with `"` -> parse string literal (must be fully quoted)
- If literal `true|false|null` -> boolean/null
- If numeric literal -> parse float, normalize `-0` -> `0`
- Else -> unquoted string

String literal parsing:
- Requires closing quote; errors on extra chars after closing quote.
- Unescapes `\\`, `\"`, `\n`, `\r`, `\t`.

### 6.4 Root Detection

The decoder reads the first non-blank line:

1. If it is an **array header**, decode as root array.
2. If it is a **single non-key-value line** and no more lines exist -> root primitive.
3. Otherwise -> root object.

Empty input -> empty object `{}`.

### 6.5 Streaming Decode (Event Model)

The decoder yields events:

- `startObject`, `endObject`
- `startArray { length }`, `endArray`
- `key { key, wasQuoted? }`
- `primitive { value }`

The event stream is deterministic and used by:
- `decodeFromLines` (materialize value)
- CLI streaming JSON output

### 6.6 Key-Value Decoding

For a key-value line:

- Try parsing as array header with key.
- Otherwise parse key (quoted/unquoted).
- If content after colon is empty:
  - If next line is deeper -> nested object.
  - Else -> empty object.
- If content after colon is non-empty -> primitive value.

Quoted keys emit `wasQuoted: true`.

### 6.7 Array Decoding Modes

Arrays are decoded based on header shape:

1. **Inline primitive array** (values after `:` on header line)
   - Parse delimited values
   - Validate expected count if strict

2. **Tabular array** (header has fields)
   - Each row at depth+1 parsed as delimited values
   - Each row -> object by field names
   - Validate row count and value count if strict
   - No blank lines allowed within row range (strict)
   - Reject extra rows (strict)

3. **List array** (no fields and no inline values)
   - Each item begins with `-` at depth+1
   - Validate item count if strict
   - No blank lines allowed within item range (strict)
   - Reject extra items (strict)

List item decoding:

- `-` alone -> empty object
- `- <array header>` -> nested array
- `- key[N]{fields}:` -> object where first field is tabular array
  - subsequent sibling fields read at same depth
- `- key: value` -> object with first field inline
  - subsequent sibling fields read at same depth
- `- primitive` -> primitive array item

### 6.8 Event to Value (Materialization)

`buildValueFromEvents`:

- Maintains stack of object/array contexts
- Supports root primitive
- Tracks quoted keys for path expansion
- Errors on mismatched start/end events

Additional streaming edge cases:

- `decodeStreamSync` and `decodeStream` throw if `expandPaths` is provided.
- Empty input yields events: `startObject`, `endObject`.
- Root primitive event stream yields a single `primitive` event.
- `buildValueFromEvents` throws on incomplete event streams.
- `decodeFromLines(lines)` must match `decode(string)` for equivalent input.
- `decodeFromLines` supports `expandPaths: safe`.

### 6.9 Path Expansion (safe)

`expandPathsSafe(value, strict)`:

- Expands dotted keys into nested objects if all segments are valid identifiers
- Suppresses expansion if key was **originally quoted**
  - Quoted keys are tracked via `QUOTED_KEY_MARKER` metadata on objects
- Deep-merge objects on conflict
  - Strict mode: throw `TypeError` on conflicts
  - Non-strict: last write wins

Key rules:
- Segment validation uses `isIdentifierSegment`
- Arrays are recursively expanded by element

---

## 6.10 Streaming JSON Output (CLI helpers)

The CLI uses two helpers for JSON output:

### `jsonStreamFromEvents`

Converts stream events into JSON output, matching `JSON.stringify` formatting:

- `indent = 0` -> compact JSON, no extra whitespace.
- `indent > 0` -> pretty JSON with newlines and spaces.
- Error cases:
  - Mismatched end events -> error.
  - `key` event outside object -> error.
  - `primitive` in object without preceding key -> error.
  - Unclosed arrays/objects -> error.
  - Error messages include:
    - `Mismatched endObject event`
    - `Mismatched endArray event`
    - `Key event outside of object context`
    - `Primitive event in object without preceding key`
    - `Incomplete event stream: unclosed objects or arrays`

### `jsonStringifyLines`

Streaming JSON stringify for full values:

- Matches `JSON.stringify(value, null, indent)` output.
- `undefined` converts to `null`.
- Preserves object key order as in input.
- Handles large arrays/objects without allocating full string.

---

## 7. CLI Commands Specification

CLI entry: `toon` (Rust port: `tru`)

Usage:

```
tru [options] [input]
```

### Input / Output

- `input` positional:
  - omitted or `-` -> stdin
  - file path -> file input
- `-o, --output <file>` -> output file (stdout if omitted)

### Flags

- `-e, --encode` -> force encode
- `-d, --decode` -> force decode
- `--delimiter <,|\\t|\\|>` -> array delimiter
- `--indent <n>` -> indentation size
- `--no-strict` -> disable strict decoding
- `--keyFolding <off|safe>` -> folding mode (default off)
- `--flattenDepth <n>` -> max folded segments (default Infinity)
- `--expandPaths <off|safe>` -> path expansion on decode
- `--stats` -> print token statistics (encode only)

### Mode Detection

Priority:

1. Explicit flags (`--encode`/`--decode`)
2. File extension: `.json` -> encode, `.toon` -> decode
3. Default: encode

### Version Flag

`--version` prints the package version using `consola.log(version)`.

### Validation Rules

- `indent`: integer >= 0
- `delimiter`: must be one of comma, tab, pipe
- `keyFolding`: must be `off` or `safe`
- `flattenDepth`: integer >= 0 if provided
- `expandPaths`: must be `off` or `safe`

### Encode Path

- Read JSON input (full string)
- Parse JSON (errors -> "Failed to parse JSON: ...")
- Encode to TOON using streaming lines
- If `--stats`:
  - Build full TOON string
  - Estimate token counts for JSON and TOON
  - Print token savings summary

### Decode Path

- If `expandPaths = safe`: read full string, decode full value, apply expandPaths
- Else: stream lines -> decode events -> stream JSON output

### Output Semantics

- File output writes without extra trailing whitespace.
- Stdout path appends newline after full output.
- Success messages printed with `consola` (Rust: match text).

### Exact Success Messages (CLI Tests)

Success lines use backticks around paths and a Unicode arrow between them (U+2192).
Represented in ASCII here as `->` to avoid non-ASCII characters.

- Encode success:
  - `Encoded \`<input>\` -> \`<output>\``
  - `<input>` can be `stdin` (from stdin path label).
- Decode success:
  - `Decoded \`<input>\` -> \`<output>\``

### Token Stats Output (encode --stats)

When `--stats` is enabled:

1. The encoded TOON output is written to stdout (full, non-streaming).
2. A blank line is printed.
3. Info message:
   - `Token estimates: ~<json> (JSON) -> ~<toon> (TOON)`
4. Success message:
   - `Saved ~<diff> tokens (-<percent>%)`

### Error Messages (CLI)

These are thrown as `Error` messages and logged via `consola.error`:

- Invalid delimiter:
  - `Invalid delimiter "<x>". Valid delimiters are: comma (,), tab (\t), pipe (|)`
- Invalid indent:
  - `Invalid indent value: <value>`
- Invalid keyFolding:
  - `Invalid keyFolding value "<x>". Valid values are: off, safe`
- Invalid expandPaths:
  - `Invalid expandPaths value "<x>". Valid values are: off, safe`
- Invalid flattenDepth:
  - `Invalid flattenDepth value: <value>`

JSON parse failures and decode failures are wrapped as:

- `Failed to parse JSON: <message>`
- `Failed to decode TOON: <message>`

### Root Primitive JSON Output (CLI)

When decoding root primitives:

- Number: `42\n`
- Boolean: `true\n`
- String: `"Hello"\n` (JSON string format)

---

## 8. Configuration and Defaults

No config files or env vars in reference implementation.

Defaults:

- Encode:
  - indent = 2
  - delimiter = comma
  - keyFolding = off
  - flattenDepth = Infinity
- Decode:
  - indent = 2
  - strict = true
  - expandPaths = off

---

## 9. Validation Rules (Strict vs Non-Strict)

### Strict Mode Enforced

- Indentation must be multiple of indent size
- Tabs in indentation are forbidden
- Array/list counts must match declared lengths
- No blank lines inside array/list/tabular segments
- Reject extra list items or tabular rows beyond declared count

### Non-Strict Mode

- Allows extra items/rows and blank lines
- Does not enforce length counts

---

## 10. Error Handling

Errors thrown by reference implementation (types may vary):

- `SyntaxError`
  - Unterminated string
  - Missing colon after key
  - Tabs in indentation (strict)
  - Blank lines in arrays (strict)
- `TypeError`
  - Invalid array length in header
  - Path expansion conflict (strict)
- `RangeError`
  - Count mismatch (strict)
  - Extra rows/items (strict)
- `ReferenceError`
  - Missing expected list item
- `Error`
  - General decode failures

CLI wraps errors with:

- Encode: `"Failed to parse JSON: ..."`
- Decode: `"Failed to decode TOON: ..."`

---

## 11. Porting Considerations

1. **Normalization** must preserve:
   - `-0 -> 0`
   - `NaN/Infinity -> null`
   - `BigInt` handling
   - `Date -> ISO string`
   - `Set`/`Map` conversions

2. **Quoting rules** are exact:
   - Leading/trailing whitespace => quoted
   - Numeric-like strings => quoted
   - Structural chars (`:`, `[`, `]`, `{`, `}`, quotes, backslash) => quoted
   - Delimiter present => quoted
   - Leading `-` => quoted
   - Unicode letters and emoji are allowed unquoted unless another rule triggers quoting

3. **Delimiter** affects:
   - String quoting safety
   - Array header suffix
   - Tabular row parsing

4. **Key folding** collision checks:
   - Must consider root literal dotted keys

5. **Path expansion** must respect quoted keys:
   - Quoted dotted keys never expand

6. **Streaming decode** must:
   - Emit events in the exact order
   - Respect depth-based grouping
   - Preserve row/list boundaries

7. **CLI** must:
   - Match flag names + defaults
   - Provide same auto-detection behavior
   - Maintain streaming memory profile

8. **Conformance fixtures**:
   - Use `legacy_spec/tests/fixtures` for spec-driven encode/decode tests.
   - Harness skeleton lives in `tests/conformance.rs` (feature: `conformance`).
