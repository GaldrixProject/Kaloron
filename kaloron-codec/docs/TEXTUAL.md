# Kaloron Textual Codec

This document describes the **current** textual payload codec implemented in
`kaloron-codec/src/textual/` and exposed as `TextualCodec`.

The format is **schema-guided**: values are encoded and decoded against the
expected `kaloron::TypeShape` and protocol `Version`. It is designed to be
human-readable, round-trippable, and strict.

It is **JSON-like**, but it is **not JSON**:

- strings use JSON string escaping,
- arrays and objects use familiar `[]` / `{}` delimiters,
- object keys are **bare lowercase hexadecimal ids**, not quoted strings,
- scalar atoms such as `u8:2a`, `none`, and `unit` are not JSON values.

---

## Format overview

The textual surface has four kinds of values:

- **atoms**: `true`, `false`, `none`, `unit`, `u8:2a`, `f64:3ff0000000000000`
- **strings**: JSON string literals such as `"Ada"`
- **arrays**: `[2$u8:1,u8:2]`
- **objects**: `{0:u8:1,1:"Ada"}`

Decoding is strict and schema-driven:

- the expected Rust shape determines which form is accepted,
- numeric widths and signedness are explicit on the wire,
- named-struct field ids and enum variant ids must match the active schema,
- trailing input is rejected.

---

## Whitespace

No whitespace is permitted anywhere outside string literals.

That means the textual encoding is always **tightly packed**:

```text
[2$u8:1,u8:2]
{0:"Ada",1:true}
```

Any ASCII whitespace between tokens, around punctuation, before or after the
payload, or inside scalar atoms is rejected.

Examples of invalid input:

```text
[2$ u8:1,u8:2]
{0 :"Ada",1:true}
true 
i8: -1
```

Whitespace characters are only allowed when they are part of a quoted string
literal.

---

## Canonical scalar forms

### Booleans

```text
true
false
```

### Unsigned integers

Unsigned integers are tagged by width and rendered as lowercase hexadecimal
without leading zeroes.

```text
u8:0
u8:2a
u16:ffff
u64:1
u128:deadbeef
```

### Signed integers

Signed integers use the same lowercase hexadecimal form, with `-` written after
the type tag when the value is negative.

```text
i8:0
i8:7f
i8:-80
i64:-2a
i128:-80000000000000000000000000000000
```

Rules:

- lowercase hex only,
- no leading zeroes,
- no `+` sign,
- signed overflow is rejected according to the target width.

### Floating-point values

Floats are encoded as exact IEEE-754 bit patterns, always using fixed-width
lowercase hexadecimal.

```text
f32:3f800000
f64:3ff0000000000000
f64:8000000000000000
```

This preserves exact payloads, including NaNs and signed zero.

### `char`

A `char` is encoded as a JSON string whose decoded contents are exactly one
Unicode scalar value.

```text
"A"
"ß"
"🦀"
```

### `String`

A `String` is encoded as a JSON string literal.

```text
"Ada"
"hello\nworld"
```

The reader accepts normal JSON escapes, including `\uXXXX` escapes and surrogate
pairs.

### Unix file descriptors

On Unix, `OwnedFd` is encoded as a `u64` atom containing the raw file
descriptor value.

```text
u64:3
```

This is a literal reflection of the current implementation. It is not a
portable cross-process fd-transfer mechanism by itself.

---

## Arrays and the mandatory length marker

Every array begins with a **mandatory length marker** immediately after `[`.

Forms:

- known length: `<hex-length>$`
- streamed / unknown length: `$`

Examples:

```text
[0$]
[2$u8:1,u8:2]
[$u8:1,u8:2]
```

Rules:

- length digits are lowercase hexadecimal,
- no leading zeroes except the value `0`,
- the marker is required for **all** arrays,
- `[value]` without a marker is invalid.

The writer uses:

- a concrete length for tuples, tuple structs, map entry pairs, and any
  sequence/map whose length is known,
- `$` when a sequence or map is streamed without a known length.

---

## Objects

Objects use bare lowercase hexadecimal ids as keys:

```text
{0:u8:1,1:"Ada"}
```

Object key rules:

- keys are **not quoted**,
- keys are lowercase hexadecimal,
- keys must not contain leading zeroes,
- keys are parsed as `u32` ids.

This is used for:

- named structs: field ids,
- enums: the single variant id.

---

## Type mapping

The exact textual form depends on the expected `TypeShape`.

| Rust / schema shape | Textual representation |
|---|---|
| `bool` | `true` / `false` |
| `i8` / `i16` / `i32` / `i64` / `i128` | tagged lowercase hex, e.g. `i64:-2a` |
| `u8` / `u16` / `u32` / `u64` / `u128` | tagged lowercase hex, e.g. `u16:2a` |
| `f32` / `f64` | tagged fixed-width IEEE bit pattern, e.g. `f32:3f800000` |
| `char` | JSON string of length 1 |
| `String` | JSON string |
| `OwnedFd` (Unix) | `u64:<raw-fd>` |
| `Option<T>` | `none` for `None`, otherwise the inner `T` representation |
| sequence | array with mandatory length marker |
| tuple | array with mandatory length marker |
| `()` | empty tuple, therefore `[0$]` |
| newtype struct | inner value directly |
| tuple struct | array with mandatory length marker |
| unit struct | zero-field tuple struct, therefore `[0$]` |
| map | array of 2-element entry arrays |
| named struct | object keyed by field id |
| enum unit variant | single-entry object whose value is `unit` |
| enum newtype variant | single-entry object whose value is the inner payload |
| enum tuple variant | single-entry object whose value is an array |
| enum named variant | single-entry object whose value is an object |

### Important note about `unit`

The literal `unit` exists in the codec, but for current `TypeShape`
implementations it is primarily used as the payload of **enum unit variants**.

The standalone Rust unit type `()` is not encoded as `unit`; it is encoded as
an empty tuple:

```text
[0$]
```

Likewise, a derived unit struct currently travels through the tuple-struct path
and is encoded as:

```text
[0$]
```

---

## Composite forms

### `Option<T>`

```text
none
<encoded T>
```

Examples:

```text
none
u8:2a
"Ada"
[0$]
```

This stays unambiguous because decoding is schema-guided.

### Sequences

Sequences are arrays whose elements follow the length marker.

Examples:

```text
[3$u8:1,u8:2,u8:3]
[$u8:1,u8:2]
```

The decoder passes the optional size hint through `SequenceRecv::len`.

### Tuples and tuple structs

Tuples and tuple structs are fixed-arity arrays.

Examples:

```text
[0$]
[2$u8:1,u16:2]
[2$"Ada",true]
```

The decoder validates the explicit length marker when present and also rejects
missing or extra elements.

### Maps

Maps are arrays of key/value entry arrays. Each entry array is itself a fixed
2-element array.

Example:

```text
[2$[2$u8:1,"one"],[2$u8:2,"two"]]
```

A streamed outer map is also valid:

```text
[$[2$u8:1,"one"],[2$u8:2,"two"]]
```

Rules:

- the outer array length marker is optional in the sense that it may be known
  (`<n>$`) or unknown (`$`),
- each entry must decode as exactly one key and one value,
- entry arrays with explicit lengths other than `2` are rejected.

### Newtype structs

A newtype struct is encoded as its wrapped value directly.

Example:

```text
u16:1e
```

### Named structs

Named structs are objects keyed by field id.

Example:

```text
{0:u8:1,1:"Ada"}
```

Rules:

- the codec first queries `NamedStructSchema::active_fields_at(version)`,
- if that returns `Some(states)`, only `Active` and `Deprecated` fields
  participate and `Inactive` fields are omitted / rejected,
- if that returns `None`, the full declared field set participates,
- fields are emitted in schema order; for derived types this is ascending field
  id order,
- decoding requires the same participating-field order,
- missing participating fields are rejected,
- unexpected extra fields are rejected.

A named struct with no active fields encodes as:

```text
{}
```

### Enums

Enums are encoded as **single-entry objects** whose key is the **variant id**.
The variant name is never written on the wire.

Examples:

```text
{0:unit}
{1:u16:1e}
{2:[2$u8:1,u8:2]}
{3:{0:u64:1,1:"Ada"}}
```

Rules:

- the object must contain exactly one field,
- the field key is the variant id in lowercase hexadecimal,
- unit variants use `unit` as their payload,
- newtype variants use the encoded inner value,
- tuple variants use an array payload,
- named variants use an object payload,
- variant participation is determined from `EnumSchema::active_variants_at(version)`;
  if it returns `Some(states)`, `Inactive` variants are rejected, and if it
  returns `None`, the full declared variant set participates,
- unknown variant ids are rejected,
- extra object fields are rejected.

---

## Version handling

The codec is version-aware through `kaloron::Version`.

### Named structs

For a given version:

- the codec consults `NamedStructSchema::active_fields_at(version)`,
- if it returns `Some(states)`, `Inactive` fields are omitted / rejected while
  `Active` and `Deprecated` fields are treated as present schema members,
- if it returns `None`, the codec uses the full declared field set,
- named variant payload objects use the same rule.

### Enums

For a given version:

- the codec consults `EnumSchema::active_variants_at(version)`,
- if it returns `Some(states)`, inactive variants cannot be encoded and
  incoming inactive variant ids are rejected,
- if it returns `None`, the codec uses the full declared variant set.

---

## `Codec::bound` and textual boundedness

`TextualCodec::bound::<T>(version)` returns an upper bound for the number of
bytes emitted when encoding a value of `T` with this textual codec at the given
protocol version.

The result is schema-driven and version-aware:

- version-gated named-struct fields that are inactive at `version` do not
  contribute to the bound,
- version-gated enum variants that are inactive at `version` are excluded when
  taking the maximum over possible variant payloads,
- deprecated fields and variants still contribute because they remain encodable,
- if any reachable part of the schema is textually unbounded, the result is
  `None`.

For the current textual codec:

- bounded: `bool`, all integer widths, `f32`, `f64`, `char`, `Option<T>` when
  `T` is bounded, tuples, tuple structs, unit structs, newtype structs whose
  inner value is bounded, named structs whose participating fields are bounded,
  and enums whose active variant payloads are all bounded,
- unbounded: `String`, `Seq<T>`, `Map<K, V>`, and any shape that contains one of
  those transitively.

A few useful examples:

- `bool` → `Some(5)` because `false` is the longest boolean atom,
- `u128` → `Some(37)` for `u128:` plus 32 lowercase hex digits,
- `i128` → `Some(38)` for `i128:-` plus 32 lowercase hex digits,
- `()` / unit tuple struct → `Some(4)` because the encoding is `[0$]`,
- `Option<u8>` → `Some(5)` because `u8:ff` is longer than `none`,
- `String` / `Vec<T>` / maps → `None`.

## Strictness and validation

The textual codec is intentionally strict.

It rejects, among other things:

- trailing input after a decoded value,
- any whitespace outside string literals,
- non-canonical numeric atoms,
- uppercase hex digits,
- leading zeroes in numeric values or object ids,
- arrays without a length marker,
- tuple length mismatches,
- map entries that are missing a key or value,
- map entries with more than two elements,
- named structs with missing, out-of-order, or unexpected fields,
- enums with missing, unknown, inactive, or extra variant ids,
- strings that are invalid for the expected target type,
- type mismatches against the active schema.

---

## Canonical examples

```text
true
u8:2a
i64:-2a
f32:3f800000
"hello\nworld"
none
[0$]
[3$u8:1,u8:2,u8:3]
[$u8:1,u8:2]
[1$[2$u8:1,u8:a]]
{0:u8:1,1:"Ada"}
{0:unit}
{1:u16:1e}
{2:[2$u8:1,u8:2]}
{3:{0:u64:1,1:"Ada"}}
```

---

## Summary

The current textual codec is a strict, schema-driven, JSON-like format with:

- explicit typed scalar atoms,
- mandatory array length markers,
- bare hexadecimal object ids,
- tightly packed syntax with no whitespace outside strings,
- version-aware omission and validation for named structs and enums,
- exact round-trippable floating-point and integer forms.

When in doubt, the implementation under `kaloron-codec/src/textual/` is the
source of truth, and the canonical writer output shown here matches that
implementation.
