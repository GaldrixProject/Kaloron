// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

//! Runtime version validation tests.
//!
//! These tests exercise the generated send/recv code to verify that version
//! validation works correctly at runtime:
//! - Gated fields must be Void when the field doesn't exist at the version
//! - Gated fields must be Active when the field is required at the version
//! - Deprecated fields accept both Active and Void
//! - Enum variants that don't exist at the version are rejected
//! - Callers must provide Gated::Void for inactive gated fields during recv

use kaloron::{Gated, TypeShape, Version};
use std::mem::ManuallyDrop;

// ===========================================================================
// Test types
// ===========================================================================

/// Struct with gated fields at different version ranges.
#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "1.0.0")]
pub struct GatedStruct {
    /// Always active (same as container).
    #[kaloron(id = 0)]
    pub base: u32,

    /// Introduced at 2.0.0 (later than container).
    #[kaloron(id = 1, introduced = "2.0.0")]
    pub later: Gated<u32>,

    /// Active from 1.5.0 to 3.0.0.
    #[kaloron(id = 2, introduced = "1.5.0", removed = "3.0.0")]
    pub temp: Gated<u32>,
}

/// Struct with a deprecated field (not gated, just deprecated — always active).
/// Included to verify that the derive macro accepts deprecated non-gated fields.
#[allow(dead_code)]
#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "1.0.0")]
pub struct DeprecatedFieldStruct {
    #[kaloron(id = 0)]
    pub keep: u32,

    #[kaloron(id = 1, introduced = "1.0.0", deprecated = "2.0.0")]
    pub old: u32,
}

/// Struct with a gated field that is deprecated then removed.
#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "1.0.0")]
pub struct DeprecatedGatedStruct {
    #[kaloron(id = 0)]
    pub keep: u32,

    #[kaloron(id = 1, introduced = "2.0.0", deprecated = "3.0.0")]
    pub feature: Gated<u32>,
}

/// Enum with versioned variants.
#[allow(dead_code)]
#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "1.0.0")]
pub enum VersionedEnum {
    #[kaloron(id = 0)]
    Alpha,

    #[kaloron(id = 1, introduced = "2.0.0")]
    Beta(u32),

    #[kaloron(id = 2, introduced = "1.5.0", removed = "3.0.0")]
    Gamma,
}

// ===========================================================================
// Mock visitor infrastructure
// ===========================================================================

/// A RecvVisitor that returns a single value of the expected type.
/// Uses the ManuallyDrop + pointer-cast pattern from the existing tests.
/// SAFETY: Only safe when the stored type T matches the requested type U.
struct MockTypedRecvVisitor<T: Copy> {
    value: Option<T>,
}

impl<T: Copy> MockTypedRecvVisitor<T> {
    fn new(value: T) -> Self {
        Self { value: Some(value) }
    }
}

impl<T: Copy + 'static> kaloron::RecvVisitor for MockTypedRecvVisitor<T> {
    fn visit<U>(&mut self) -> anyhow::Result<U> {
        let value = self
            .value
            .take()
            .expect("MockTypedRecvVisitor value already consumed");
        let value = ManuallyDrop::new(value);
        let ptr = (&*value as *const T).cast::<U>();
        Ok(unsafe { ptr.read() })
    }
}

/// A SendAccept that accepts any schema kind without doing anything.
/// Used for send validation tests where we only care whether the version
/// checks pass or fail, not the actual data.
struct MockNoOpSendAccept;

impl kaloron::SendAccept for MockNoOpSendAccept {
    fn accept_primitive(&mut self, _: &impl kaloron::PrimitiveSend) -> anyhow::Result<()> {
        Ok(())
    }
    fn accept_option(&mut self, _: &impl kaloron::OptionSend) -> anyhow::Result<()> {
        Ok(())
    }
    fn accept_sequence(&mut self, _: &impl kaloron::SequenceSend) -> anyhow::Result<()> {
        Ok(())
    }
    fn accept_tuple(&mut self, _: &impl kaloron::TupleSend) -> anyhow::Result<()> {
        Ok(())
    }
    fn accept_map(&mut self, _: &impl kaloron::MapSend) -> anyhow::Result<()> {
        Ok(())
    }
    fn accept_newtype_struct(&mut self, _: &impl kaloron::NewTypeSend) -> anyhow::Result<()> {
        Ok(())
    }
    fn accept_tuple_struct(&mut self, _: &impl kaloron::TupleSend) -> anyhow::Result<()> {
        Ok(())
    }
    fn accept_named_struct(&mut self, _: &impl kaloron::TupleSend) -> anyhow::Result<()> {
        Ok(())
    }
    fn accept_enum(&mut self, send: &impl kaloron::EnumSend) -> anyhow::Result<()> {
        struct Sink;
        impl kaloron::EnumSendAccept for Sink {
            fn accept_unit(&mut self, _: isize) -> anyhow::Result<()> {
                Ok(())
            }
            fn accept_newtype(
                &mut self,
                _: isize,
                _: &impl kaloron::NewTypeSend,
            ) -> anyhow::Result<()> {
                Ok(())
            }
            fn accept_tuple(
                &mut self,
                _: isize,
                _: &impl kaloron::TupleSend,
            ) -> anyhow::Result<()> {
                Ok(())
            }
            fn accept_named(
                &mut self,
                _: isize,
                _: &impl kaloron::TupleSend,
            ) -> anyhow::Result<()> {
                Ok(())
            }
        }
        send.visit(&mut Sink)
    }
}

/// RecvAccept for GatedStruct that visits all fields, providing Gated::Void
/// for inactive gated fields (callers must supply all fields).
struct MockGatedStructRecvAccept {
    version: Version,
    base: u32,
    later: u32,
    temp: u32,
}

impl kaloron::RecvAccept for MockGatedStructRecvAccept {
    fn accept_primitive(&mut self, _: &mut impl kaloron::PrimitiveRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected accept_primitive")
    }
    fn accept_option(&mut self, _: &mut impl kaloron::OptionRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected accept_option")
    }
    fn accept_sequence(&mut self, _: &mut impl kaloron::SequenceRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected accept_sequence")
    }
    fn accept_tuple(&mut self, _: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected accept_tuple")
    }
    fn accept_map(&mut self, _: &mut impl kaloron::MapRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected accept_map")
    }
    fn accept_newtype_struct(&mut self, _: &mut impl kaloron::NewTypeRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected accept_newtype_struct")
    }
    fn accept_tuple_struct(&mut self, _: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected accept_tuple_struct")
    }
    fn accept_named_struct(&mut self, recv: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        // Field 0 (base: u32) — always active at 1.0.0+
        {
            let mut v = MockTypedRecvVisitor::new(self.base);
            recv.visit(0, &mut v)?;
        }

        // Field 1 (later: Gated<u32>) — active at 2.0.0+, Void otherwise
        if self.version >= Version::new(2, 0, 0, 0) {
            let mut v = MockTypedRecvVisitor::new(Gated::Active(self.later));
            recv.visit(1, &mut v)?;
        } else {
            let mut v = MockTypedRecvVisitor::new(Gated::<u32>::Void);
            recv.visit(1, &mut v)?;
        }

        // Field 2 (temp: Gated<u32>) — active at [1.5.0, 3.0.0), Void otherwise
        if self.version >= Version::new(1, 5, 0, 0) && self.version < Version::new(3, 0, 0, 0) {
            let mut v = MockTypedRecvVisitor::new(Gated::Active(self.temp));
            recv.visit(2, &mut v)?;
        } else {
            let mut v = MockTypedRecvVisitor::new(Gated::<u32>::Void);
            recv.visit(2, &mut v)?;
        }

        Ok(())
    }
    fn accept_enum(&mut self, _: &mut impl kaloron::EnumRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected accept_enum")
    }
}

/// RecvAccept for VersionedEnum that produces a specific variant.
struct MockEnumUnitRecvAccept {
    variant_index: isize,
}

impl kaloron::RecvAccept for MockEnumUnitRecvAccept {
    fn accept_primitive(&mut self, _: &mut impl kaloron::PrimitiveRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_option(&mut self, _: &mut impl kaloron::OptionRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_sequence(&mut self, _: &mut impl kaloron::SequenceRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_tuple(&mut self, _: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_map(&mut self, _: &mut impl kaloron::MapRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_newtype_struct(&mut self, _: &mut impl kaloron::NewTypeRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_tuple_struct(&mut self, _: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_named_struct(&mut self, _: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_enum(&mut self, recv: &mut impl kaloron::EnumRecv) -> anyhow::Result<()> {
        struct UnitAccept;
        impl kaloron::EnumRecvAccept for UnitAccept {
            fn accept_newtype(&mut self, _: &mut impl kaloron::NewTypeRecv) -> anyhow::Result<()> {
                anyhow::bail!("unexpected")
            }
            fn accept_tuple(&mut self, _: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
                anyhow::bail!("unexpected")
            }
            fn accept_named(&mut self, _: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
                anyhow::bail!("unexpected")
            }
        }
        recv.visit(self.variant_index, &mut UnitAccept)
    }
}

/// RecvAccept for VersionedEnum that produces a newtype variant with a u32.
struct MockEnumNewtypeRecvAccept {
    variant_index: isize,
    value: u32,
}

impl kaloron::RecvAccept for MockEnumNewtypeRecvAccept {
    fn accept_primitive(&mut self, _: &mut impl kaloron::PrimitiveRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_option(&mut self, _: &mut impl kaloron::OptionRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_sequence(&mut self, _: &mut impl kaloron::SequenceRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_tuple(&mut self, _: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_map(&mut self, _: &mut impl kaloron::MapRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_newtype_struct(&mut self, _: &mut impl kaloron::NewTypeRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_tuple_struct(&mut self, _: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_named_struct(&mut self, _: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("unexpected")
    }
    fn accept_enum(&mut self, recv: &mut impl kaloron::EnumRecv) -> anyhow::Result<()> {
        struct NewtypeAccept {
            value: u32,
        }
        impl kaloron::EnumRecvAccept for NewtypeAccept {
            fn accept_newtype(
                &mut self,
                visitor: &mut impl kaloron::NewTypeRecv,
            ) -> anyhow::Result<()> {
                let mut rv = MockTypedRecvVisitor::new(self.value);
                visitor.visit(&mut rv)
            }
            fn accept_tuple(&mut self, _: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
                anyhow::bail!("unexpected")
            }
            fn accept_named(&mut self, _: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
                anyhow::bail!("unexpected")
            }
        }
        recv.visit(self.variant_index, &mut NewtypeAccept { value: self.value })
    }
}

// ===========================================================================
// Send validation tests — named struct gated fields
// ===========================================================================

#[test]
fn test_send_gated_struct_at_v1_with_void_fields_succeeds() {
    let value = GatedStruct {
        base: 42,
        later: Gated::Void,
        temp: Gated::Void,
    };
    let result = value.send(Version::new(1, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
}

#[test]
fn test_send_gated_struct_at_v1_with_active_later_fails() {
    let value = GatedStruct {
        base: 42,
        later: Gated::Active(99),
        temp: Gated::Void,
    };
    let result = value.send(Version::new(1, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("expected inactive"), "unexpected error: {msg}");
}

#[test]
fn test_send_gated_struct_at_v1_with_active_temp_fails() {
    let value = GatedStruct {
        base: 42,
        later: Gated::Void,
        temp: Gated::Active(7),
    };
    let result = value.send(Version::new(1, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("expected inactive"), "unexpected error: {msg}");
}

#[test]
fn test_send_gated_struct_at_v2_all_active_succeeds() {
    let value = GatedStruct {
        base: 42,
        later: Gated::Active(99),
        temp: Gated::Active(7),
    };
    let result = value.send(Version::new(2, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
}

#[test]
fn test_send_gated_struct_at_v2_void_later_fails() {
    // later is required at 2.0.0 (not deprecated)
    let value = GatedStruct {
        base: 42,
        later: Gated::Void,
        temp: Gated::Active(7),
    };
    let result = value.send(Version::new(2, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("expected active"), "unexpected error: {msg}");
}

#[test]
fn test_send_gated_struct_at_v3_temp_removed_void_succeeds() {
    // At 3.0.0: temp is removed, later is active
    let value = GatedStruct {
        base: 42,
        later: Gated::Active(99),
        temp: Gated::Void,
    };
    let result = value.send(Version::new(3, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
}

#[test]
fn test_send_gated_struct_at_v3_temp_removed_active_fails() {
    // At 3.0.0: temp is removed, so Active should fail
    let value = GatedStruct {
        base: 42,
        later: Gated::Active(99),
        temp: Gated::Active(7),
    };
    let result = value.send(Version::new(3, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("expected inactive"), "unexpected error: {msg}");
}

// ===========================================================================
// Send validation tests — deprecated fields
// ===========================================================================

#[test]
fn test_send_deprecated_gated_field_active_at_deprecated_version_succeeds() {
    // feature is introduced at 2.0.0, deprecated at 3.0.0
    // At 3.0.0: feature exists but is deprecated → Active is allowed
    let value = DeprecatedGatedStruct {
        keep: 1,
        feature: Gated::Active(42),
    };
    let result = value.send(Version::new(3, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
}

#[test]
fn test_send_deprecated_gated_field_void_at_deprecated_version_succeeds() {
    // At 3.0.0: feature exists but is deprecated → Void is also allowed
    let value = DeprecatedGatedStruct {
        keep: 1,
        feature: Gated::Void,
    };
    let result = value.send(Version::new(3, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
}

#[test]
fn test_send_deprecated_gated_field_void_before_introduction_succeeds() {
    // At 1.0.0: feature doesn't exist yet → Void is required
    let value = DeprecatedGatedStruct {
        keep: 1,
        feature: Gated::Void,
    };
    let result = value.send(Version::new(1, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
}

#[test]
fn test_send_deprecated_gated_field_active_before_introduction_fails() {
    // At 1.0.0: feature doesn't exist yet → Active is error
    let value = DeprecatedGatedStruct {
        keep: 1,
        feature: Gated::Active(99),
    };
    let result = value.send(Version::new(1, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("expected inactive"), "unexpected error: {msg}");
}

// ===========================================================================
// Send validation tests — enum variants
// ===========================================================================

#[test]
fn test_send_enum_existing_variant_succeeds() {
    let value = VersionedEnum::Alpha;
    let result = value.send(Version::new(1, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
}

#[test]
fn test_send_enum_not_yet_introduced_variant_fails() {
    // Beta is introduced at 2.0.0; send at 1.0.0 should fail
    let value = VersionedEnum::Beta(42);
    let result = value.send(Version::new(1, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Beta") && msg.contains("does not exist"),
        "unexpected error: {msg}"
    );
}

#[test]
fn test_send_enum_removed_variant_fails() {
    // Gamma is removed at 3.0.0; send at 3.0.0 should fail
    let value = VersionedEnum::Gamma;
    let result = value.send(Version::new(3, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Gamma") && msg.contains("does not exist"),
        "unexpected error: {msg}"
    );
}

#[test]
fn test_send_enum_active_variant_succeeds() {
    // Beta at 2.0.0 should succeed
    let value = VersionedEnum::Beta(42);
    let result = value.send(Version::new(2, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);

    // Gamma at 2.0.0 should succeed
    let value = VersionedEnum::Gamma;
    let result = value.send(Version::new(2, 0, 0, 0), &mut MockNoOpSendAccept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
}

// ===========================================================================
// Recv validation tests — named struct gated field fixups
// ===========================================================================

#[test]
fn test_recv_gated_struct_at_v1_sets_inactive_to_void() {
    // At 1.0.0: only base is active. later and temp should be Void.
    let mut accept = MockGatedStructRecvAccept {
        version: Version::new(1, 0, 0, 0),
        base: 42,
        later: 0,
        temp: 0,
    };
    let result = GatedStruct::recv(Version::new(1, 0, 0, 0), &mut accept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let value = result.unwrap();
    assert_eq!(value.base, 42);
    assert_eq!(value.later, Gated::Void);
    assert_eq!(value.temp, Gated::Void);
}

#[test]
fn test_recv_gated_struct_at_v1_5_sets_later_void_temp_active() {
    // At 1.5.0: base and temp active, later not yet introduced
    let mut accept = MockGatedStructRecvAccept {
        version: Version::new(1, 5, 0, 0),
        base: 10,
        later: 0,
        temp: 77,
    };
    let result = GatedStruct::recv(Version::new(1, 5, 0, 0), &mut accept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let value = result.unwrap();
    assert_eq!(value.base, 10);
    assert_eq!(value.later, Gated::Void);
    assert_eq!(value.temp, Gated::Active(77));
}

#[test]
fn test_recv_gated_struct_at_v2_all_active() {
    // At 2.0.0: all fields active
    let mut accept = MockGatedStructRecvAccept {
        version: Version::new(2, 0, 0, 0),
        base: 10,
        later: 20,
        temp: 30,
    };
    let result = GatedStruct::recv(Version::new(2, 0, 0, 0), &mut accept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let value = result.unwrap();
    assert_eq!(value.base, 10);
    assert_eq!(value.later, Gated::Active(20));
    assert_eq!(value.temp, Gated::Active(30));
}

#[test]
fn test_recv_gated_struct_at_v3_temp_removed() {
    // At 3.0.0: temp removed, base and later active
    let mut accept = MockGatedStructRecvAccept {
        version: Version::new(3, 0, 0, 0),
        base: 10,
        later: 20,
        temp: 0,
    };
    let result = GatedStruct::recv(Version::new(3, 0, 0, 0), &mut accept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let value = result.unwrap();
    assert_eq!(value.base, 10);
    assert_eq!(value.later, Gated::Active(20));
    assert_eq!(value.temp, Gated::Void);
}

// ===========================================================================
// Recv validation tests — enum variant existence checks
// ===========================================================================

#[test]
fn test_recv_enum_existing_unit_variant_succeeds() {
    // Alpha at 1.0.0 — should succeed
    let mut accept = MockEnumUnitRecvAccept { variant_index: 0 };
    let result = VersionedEnum::recv(Version::new(1, 0, 0, 0), &mut accept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    assert_eq!(result.unwrap(), VersionedEnum::Alpha);
}

#[test]
fn test_recv_enum_not_introduced_variant_fails() {
    // Beta (id=1) at 1.0.0 — Beta introduced at 2.0.0 → error
    let mut accept = MockEnumNewtypeRecvAccept {
        variant_index: 1,
        value: 42,
    };
    let result = VersionedEnum::recv(Version::new(1, 0, 0, 0), &mut accept);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Beta") && msg.contains("does not exist"),
        "unexpected error: {msg}"
    );
}

#[test]
fn test_recv_enum_removed_variant_fails() {
    // Gamma (id=2) at 3.0.0 — Gamma removed at 3.0.0 → error
    let mut accept = MockEnumUnitRecvAccept { variant_index: 2 };
    let result = VersionedEnum::recv(Version::new(3, 0, 0, 0), &mut accept);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Gamma") && msg.contains("does not exist"),
        "unexpected error: {msg}"
    );
}

#[test]
fn test_recv_enum_active_newtype_variant_succeeds() {
    // Beta (id=1) at 2.0.0 — should succeed
    let mut accept = MockEnumNewtypeRecvAccept {
        variant_index: 1,
        value: 99,
    };
    let result = VersionedEnum::recv(Version::new(2, 0, 0, 0), &mut accept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    assert_eq!(result.unwrap(), VersionedEnum::Beta(99));
}

#[test]
fn test_recv_enum_active_unit_variant_at_boundary_succeeds() {
    // Gamma (id=2) at 2.9.9 — Gamma removed at 3.0.0, so 2.9.9 should work
    let mut accept = MockEnumUnitRecvAccept { variant_index: 2 };
    let result = VersionedEnum::recv(Version::new(2, 9, 9, 0), &mut accept);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    assert_eq!(result.unwrap(), VersionedEnum::Gamma);
}
