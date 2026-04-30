// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use blake3::Hasher;
use kaloron::{Schema, Version, VersionRange};

/// Describes a single RPC method within a service.
///
/// `Copy` is derived so that `SERVICE_SCHEMA.methods[i]` can be used in
/// const-context expressions without moving the value.
#[derive(Copy, Clone)]
pub struct MethodSchema<'a> {
    /// Method name as declared in the service trait.
    pub name: &'a str,
    /// Explicit method ID for ABI stability on the wire.
    pub method_id: u32,
    /// Schema of the generated argument struct (a named struct with field IDs).
    pub arguments: &'a Schema<'a>,
    /// Schema of the return type `T` (before `Result` wrapping).
    pub response: &'a Schema<'a>,
    /// Version lifecycle for this method.
    pub version: VersionRange,
}

impl<'a> MethodSchema<'a> {
    /// Returns `true` when this method exists at `version`.
    pub fn is_available_at(&self, version: Version) -> bool {
        self.version.exists_at(version)
    }

    /// Contribute the effective method shape at `version` to a BLAKE3 hash.
    pub fn hash_build(&self, version: Version, build: &mut Hasher) {
        build.update(&self.method_id.to_be_bytes());
        hash_str(self.name, build);
        build.update(&[self.version.is_deprecated_at(version) as u8]);
        self.arguments.hash_build(version, build);
        self.response.hash_build(version, build);
        self.version.hash_build(build);
    }
}

/// Describes an entire RPC service.
///
/// `Copy` is derived for the same reason as [`MethodSchema`].
#[derive(Copy, Clone)]
pub struct ServiceSchema<'a> {
    /// Service name — typically the trait name or explicit textual identifier.
    pub name: &'a str,
    /// All methods declared in the service, in declaration order.
    pub methods: &'a [&'a MethodSchema<'a>],
    /// Version lifecycle for the entire service.
    pub version: VersionRange,
}

impl<'a> ServiceSchema<'a> {
    /// Returns `true` when the service exists at `version`.
    pub fn is_available_at(&self, version: Version) -> bool {
        self.version.exists_at(version)
    }

    /// Look up a method by its explicit wire ID.
    ///
    /// Returns `None` if no method with `method_id` exists in this schema.
    pub fn method_by_id(&self, method_id: u32) -> Option<&'a MethodSchema<'a>> {
        self.methods
            .iter()
            .find(|m| m.method_id == method_id)
            .copied()
    }

    /// Look up a method by wire ID, filtered to the methods active at `version`.
    pub fn method_by_id_at(
        &self,
        method_id: u32,
        version: Version,
    ) -> Option<&'a MethodSchema<'a>> {
        self.method_by_id(method_id)
            .filter(|method| method.is_available_at(version))
    }

    /// Compute the exact BLAKE3 hash of the effective service schema visible at
    /// `version`.
    ///
    /// Methods that do not exist at the requested version are excluded entirely.
    /// Methods that are deprecated but still active remain part of the hash and
    /// are tagged accordingly so deprecation transitions produce a distinct hash.
    pub fn effective_schema_hash(&self, version: Version) -> [u8; 32] {
        let mut build = Hasher::new();
        self.hash_build(version, &mut build);
        build.finalize().into()
    }

    /// Contribute the effective service schema at `version` to `build`.
    pub fn hash_build(&self, version: Version, build: &mut Hasher) {
        build.update(b"KALORON_RPC_SERVICE_SCHEMA_V1");
        hash_str(self.name, build);
        build.update(&[self.version.is_deprecated_at(version) as u8]);
        self.version.hash_build(build);

        let included_count = self
            .methods
            .iter()
            .filter(|method| method.is_available_at(version))
            .count() as u32;
        build.update(&included_count.to_be_bytes());

        for method in self.methods.iter() {
            if method.is_available_at(version) {
                method.hash_build(version, build);
            }
        }
    }
}

fn hash_str(value: &str, build: &mut Hasher) {
    let bytes = value.as_bytes();
    let len = bytes.len() as u64;
    build.update(&len.to_be_bytes());
    build.update(bytes);
}
