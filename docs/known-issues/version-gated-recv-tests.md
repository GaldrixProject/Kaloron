# Known issue: version-gated recv tests for named structs

## Summary

There is a known issue around recv-time handling of version-gated named-struct fields in `kaloron`.

At the time of writing, the workspace test suite reports four failing tests in `kaloron/tests/test_derive_type/version_validation.rs`:

- `recv_gated_struct_at_v1_sets_inactive_to_void`
- `recv_gated_struct_at_v1_5_sets_later_void_temp_active`
- `recv_gated_struct_at_v2_all_active`
- `recv_gated_struct_at_v3_temp_removed`

These failures should **not** be interpreted as a simple case of "the tests are wrong". The situation is more subtle:

1. the tests are validating an important and necessary API contract,
2. the current test helper does not accurately model the real recv contract, and
3. the current recv implementation still appears to have a real gap around reconstructing inactive gated fields as `Gated::Void`.

This document records the current understanding so the issue can be revisited later without having to repeat the investigation.

---

## Affected scope

This issue specifically concerns:

- recv-time handling of `Gated<T>` fields in **named structs**,
- version-activation-aware decoding,
- the interaction between derive-generated recv code, runtime builder validation, and real codec behavior.

The immediate evidence comes from:

- `kaloron/tests/test_derive_type/version_validation.rs`
- `kaloron-macro/src/struct_ir/named_struct_ir.rs`
- `kaloron/src/utility.rs`
- `kaloron/src/schema.rs`
- `kaloron-codec/src/textual/visit/named_struct.rs`

A similar review may eventually be warranted for named enum variant payloads, but that was not the primary scope of this investigation.

---

## What the failing tests are trying to verify

The failing tests use the following type:

```rust
#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "1.0.0")]
pub struct GatedStruct {
    #[kaloron(id = 0)]
    pub base: u32,

    #[kaloron(id = 1, introduced = "2.0.0")]
    pub later: Gated<u32>,

    #[kaloron(id = 2, introduced = "1.5.0", removed = "3.0.0")]
    pub temp: Gated<u32>,
}
```

Its intended version behavior is:

| Version | `base` | `later` | `temp` |
|---|---|---|---|
| `1.0.0` | Active | Inactive | Inactive |
| `1.5.0` | Active | Inactive | Active |
| `2.0.0` | Active | Active | Active |
| `3.0.0` | Active | Active | Inactive |

The four failing recv tests are asserting the natural recv-side interpretation of those rules:

### `recv_gated_struct_at_v1_sets_inactive_to_void`
At `1.0.0` the decoded value should be:

- `base = 42`
- `later = Gated::Void`
- `temp = Gated::Void`

### `recv_gated_struct_at_v1_5_sets_later_void_temp_active`
At `1.5.0` the decoded value should be:

- `base = 10`
- `later = Gated::Void`
- `temp = Gated::Active(77)`

### `recv_gated_struct_at_v2_all_active`
At `2.0.0` the decoded value should be:

- `base = 10`
- `later = Gated::Active(20)`
- `temp = Gated::Active(30)`

### `recv_gated_struct_at_v3_temp_removed`
At `3.0.0` the decoded value should be:

- `base = 10`
- `later = Gated::Active(20)`
- `temp = Gated::Void`

### Why that expectation matters

These tests are the recv-side equivalent of the send-side gating checks.

If a field is inactive at a negotiated version, send-side logic omits or rejects the active value as appropriate. The recv side therefore needs a corresponding rule for reconstructing the typed Rust value.

For a versioned field of type `Gated<T>`, the most reasonable and useful contract is:

- active field on the wire -> `Gated::Active(value)`
- inactive field for the negotiated version -> `Gated::Void`

That is exactly what these tests are trying to enforce.

---

## Why these tests are necessary

These tests should be preserved conceptually because they cover a real requirement of the framework.

The real decoder path in `kaloron-codec` does **not** decode every field unconditionally. In `kaloron-codec/src/textual/visit/named_struct.rs`, the decoder first computes the active fields for the negotiated version and then only visits those fields.

In other words:

- inactive fields are omitted from the wire-level traversal,
- only active or deprecated fields are actually visited.

That means the recv path must have a mechanism to produce a complete Rust value even when some version-gated fields were intentionally not present on the wire.

Without a recv-side rule for turning omitted inactive fields into `Gated::Void`, version-aware named-struct decoding is incomplete.

So the tests are not redundant. They protect a real contract that the codec and derive logic need in order to interoperate correctly.

---

## Why the current test helper is not modeling the real recv contract correctly

The custom helper in `kaloron/tests/test_derive_type/version_validation.rs` is `GatedStructRecvAccept`.

It manually calls `recv.visit(index, visitor)` for **all** fields, including inactive ones, and for gated fields it tries to supply either:

- `Gated::Active(payload)`, or
- `Gated::Void`

through `TypedRecvVisitor`.

This is not how the runtime recv machinery actually works.

### Real gated recv behavior in `kaloron/src/utility.rs`

The recv implementation for `Gated<T>` is defined by the internal `__TK` machinery:

- `impl<T: TypeShape> __TK for Gated<T>`
- `visit_recv(o, v)` calls `v.visit::<T>()`
- the result is then wrapped as `Gated::Active(...)`

The important detail is that the runtime asks the visitor for the **inner type `T`**, not for `Gated<T>`.

So for a field `Gated<u32>`, the `RecvVisitor` is asked for a `u32`, not for a `Gated<u32>`.

### Why that breaks the current test helper

`TypedRecvVisitor<T>` in the tests uses an unsafe pointer-cast pattern:

- it stores a concrete value of type `T`,
- then returns that memory as whatever `U` the caller requested.

That approach is only sound if `T` and `U` are the same actual type.

For the failing tests, the helper stores values like:

- `Gated::Void`
- `Gated::Active(20)`

but the runtime requests `u32` for gated fields.

That means the helper is returning a `Gated<u32>` value as if it were a `u32`, which is not a valid model of the real recv path.

### How this explains the observed failures

This mismatch explains the actual failure messages:

- `expected inactive, got active`
  - the helper intended to provide `Gated::Void`,
  - the runtime requested `u32`,
  - the unsafe cast yielded some value,
  - the runtime wrapped that as `Gated::Active(...)`,
  - validation then reported an active value where inactive was required.

- `left: Active(0), right: Active(20)`
  - the helper intended to provide `Gated::Active(20)`,
  - the runtime still requested `u32`,
  - the unsafe cast read the wrong bytes,
  - the decoded value became `Active(0)` instead of `Active(20)`.

So the test helper is not a faithful simulation of real decoding for gated fields.

---

## Why the issue is not only a test-helper problem

Even though the helper is wrong, the investigation also suggests a real implementation gap in recv fixup.

### Generated recv flow for named structs

In `kaloron-macro/src/struct_ir/named_struct_ir.rs`, named-struct recv with gated fields is generated roughly as:

1. create a recv builder,
2. call `accept_named_struct(&mut builder)`,
3. read activation states from schema with `active_fields_at(version)`,
4. call `::kaloron::__tbac(&builder, activation_states)`,
5. call `::kaloron::__tbc(&builder)` for completeness,
6. construct the final struct from the builder.

This is important because the generated code currently relies on validation after field receipt, rather than explicitly synthesizing missing inactive gated fields beforehand.

### What `__tbac` currently does

In `kaloron/src/utility.rs`, the relevant path is:

- `__tbac(s, a)` -> `s.validate_recv(a)`
- `validate_recv` delegates to `__tbk(self.field.as_ref(), activation)` for each field
- `__tbk` only checks fields that are already present

If a builder slot is empty, `__tbk` does nothing.

### Why that is a problem with the real decoder

The real named-struct textual decoder in `kaloron-codec/src/textual/visit/named_struct.rs` computes active fields and only visits those indices.

That means inactive fields are never filled into the builder at all.

If the builder slot for an inactive `Gated<T>` field remains `None`, and `__tbac` merely validates present values without synthesizing `Gated::Void`, then the subsequent completeness check `__tbc(&builder)` still has an unfilled slot.

This strongly suggests a real gap:

- the recv path validates version state,
- but it does not obviously materialize `Gated::Void` for omitted inactive fields before completeness enforcement.

That is exactly the behavior the failing tests are conceptually trying to pin down.

---

## Evidence from the real decoder path

The strongest evidence that recv must eventually reconstruct inactive gated fields as `Void` comes from the textual codec implementation.

In `kaloron-codec/src/textual/visit/named_struct.rs`:

1. `active_field_indices(schema, version)` filters out `ActivationState::Inactive`,
2. `decode_named_struct(...)` iterates only the resulting active indices,
3. it calls `recv.visit(index, ...)` only for those fields.

This means a real decoder does **not** and should **not** try to provide placeholder values for inactive fields over the visitor surface. The omission happens one layer earlier, at schema-driven traversal time.

Therefore, the recv builder / derive-generated recv path must be responsible for converting “inactive field was not visited” into a final `Gated::Void` value.

That is a core architectural point and should guide any future fix.

---

## Relationship to activation-table generation

The activation-table side of the system appears conceptually sound.

Relevant pieces:

- `kaloron-macro/src/version.rs` computes activation tables with `compute_activation_table_tokens(...)`
- `kaloron/src/schema.rs` provides `NamedStructSchema::active_fields_at(version)`
- `kaloron/tests/test_derive_type/version_activation.rs` already verifies the expected state transitions for versioned named structs

So the currently known issue does **not** appear to be about activation lookup itself.

The problem is downstream:

- how builder recv uses those activation states,
- and how missing inactive gated fields become fully-formed Rust values.

---

## Practical interpretation of the current failures

The current failures should be interpreted as follows:

### What they do **not** mean

They do **not** prove that the expected decoded values are wrong.

They do **not** prove that `Gated::Void` should never appear automatically on recv.

They do **not** prove that the version-gating tests are unnecessary.

### What they most likely **do** mean

They indicate two distinct problems:

1. **Test-harness mismatch**
   - the helper supplies `Gated<T>` values through a visitor path that expects `T`
   - the unsafe cast makes the test behavior diverge from real decoding

2. **Potential library recv gap**
   - real decoders skip inactive fields entirely
   - current builder validation appears to validate but not fix up missing inactive gated fields
   - recv may therefore be missing the final step that reconstructs `Gated::Void`

Both need to be considered in a future fix.

---

## Is the expected behavior correct?

### Expected output values: yes

The expected outputs in the four failing tests appear correct and should remain the target behavior:

- inactive gated field -> `Gated::Void`
- active gated field -> `Gated::Active(payload)`

That behavior matches the intended purpose of `Gated<T>` and is necessary for practical version-aware decoding.

### Current helper contract: no

The comment currently implied in the failing test file — effectively that recv adapters should provide `Gated::Void` through the raw `RecvVisitor` path for inactive fields — does not match the actual implementation shape.

A more accurate contract for named-struct recv is:

- codec-level traversal should visit only active fields,
- active gated fields should be decoded through the inner payload type,
- inactive gated fields should be reconstructed as `Gated::Void` by the recv/build layer.

That distinction is important.

---

## Recommended future investigation

When revisiting this issue, the next investigation should focus on two tasks.

### 1. Rewrite the failing tests to model the real decoder

The test fixture should be changed so that it behaves like `kaloron-codec`:

- only call `recv.visit(index, ...)` for fields active at the negotiated version,
- provide the inner payload type for active gated fields,
- do **not** try to pass `Gated::Void` through `RecvVisitor`,
- assert that omitted inactive gated fields appear as `Gated::Void` in the final decoded value.

This will ensure the tests validate the real contract rather than a helper-specific artifact.

### 2. Inspect and likely adjust recv fixup logic

The recv/build path should be examined to determine where missing inactive gated fields ought to be synthesized.

Likely candidate areas:

- `kaloron/src/utility.rs`
- `kaloron-macro/src/struct_ir/named_struct_ir.rs`
- possibly the generated builder finalization path

A future fix will probably need to ensure that before completeness is enforced:

- inactive gated builder slots are populated with `Gated::Void`, or
- completeness is defined in a version-aware way for gated fields.

Whichever approach is chosen, it should preserve the invariant that final decoded Rust values are complete and version-correct.

---

## Suggested acceptance criteria for a later fix

A future resolution should ideally satisfy all of the following:

1. the four failing named-struct recv tests pass,
2. the tests model the real decoder path rather than feeding synthetic `Gated<T>` values into inner-type visitors,
3. inactive gated fields omitted from the wire decode as `Gated::Void`,
4. active gated fields decode as `Gated::Active(payload)`,
5. send-side behavior remains unchanged and validated,
6. enum variant version checks continue to pass,
7. any fix is validated against at least one real codec path such as `kaloron-codec` textual decoding.

---

## Current status

As of this note:

- the issue is understood well enough to postpone,
- the expected behavior is considered valid,
- the current failing tests are useful in intent but flawed in execution,
- the library likely still needs recv-side work to fully support omitted inactive gated fields.

This document should be used as the starting point when resuming work on version-gated recv behavior.
