// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

//! Version types for schema lifecycle management.

use blake3::Hasher;

// ---------------------------------------------------------------------------
// Version types
// ---------------------------------------------------------------------------

/// Packed semantic version in a u64.
/// Layout: major (16 bits) | minor (16 bits) | patch (16 bits) | build (16 bits)
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(u64);

impl Version {
    /// Create a new version from components.
    pub const fn new(major: u16, minor: u16, patch: u16, build: u16) -> Self {
        Self((major as u64) << 48 | (minor as u64) << 32 | (patch as u64) << 16 | (build as u64))
    }

    /// The zero version (0.0.0.0).
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Extract the major version component.
    pub const fn major(self) -> u16 {
        (self.0 >> 48) as u16
    }

    /// Extract the minor version component.
    pub const fn minor(self) -> u16 {
        ((self.0 >> 32) & 0xFFFF) as u16
    }

    /// Extract the patch version component.
    pub const fn patch(self) -> u16 {
        ((self.0 >> 16) & 0xFFFF) as u16
    }

    /// Extract the build version component.
    pub const fn build(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    /// Return the next representable version in packed order.
    ///
    /// This advances the build component first and carries into patch,
    /// minor, and major as needed. Returns `None` if this is the maximum
    /// representable version.
    pub const fn successor(self) -> Option<Self> {
        if self.0 == u64::MAX {
            None
        } else {
            Some(Self(self.0 + 1))
        }
    }

    /// Contribute this version to a schema hash.
    /// Tag byte 0xE0 identifies this as a Version.
    pub fn hash_build(&self, build: &mut Hasher) {
        build.update(&[0xE0]);
        build.update(&self.0.to_be_bytes());
    }

    /// Format as "MAJOR.MINOR.PATCH" or "MAJOR.MINOR.PATCH.BUILD" (if build != 0).
    pub fn format(&self) -> String {
        if self.build() == 0 {
            format!("{}.{}.{}", self.major(), self.minor(), self.patch())
        } else {
            format!(
                "{}.{}.{}.{}",
                self.major(),
                self.minor(),
                self.patch(),
                self.build()
            )
        }
    }
}

impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.build() == 0 {
            write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())
        } else {
            write!(
                f,
                "{}.{}.{}.{}",
                self.major(),
                self.minor(),
                self.patch(),
                self.build()
            )
        }
    }
}

/// A single deprecation period.
/// Both `since` and `until` are inclusive bounds.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DeprecationSpan {
    /// When this deprecation period started (inclusive).
    pub since: Version,
    /// When this deprecation period ended (inclusive, None = still deprecated).
    pub until: Option<Version>,
}

impl DeprecationSpan {
    /// Create an open-ended deprecation span (deprecated from `since` onwards).
    pub const fn new(since: Version) -> Self {
        Self { since, until: None }
    }

    /// Create a closed deprecation span (deprecated from `since` through `until`).
    pub const fn new_range(since: Version, until: Version) -> Self {
        Self {
            since,
            until: Some(until),
        }
    }

    /// Check if this span covers the given version (inclusive on both ends).
    pub const fn contains(&self, version: Version) -> bool {
        version.0 >= self.since.0
            && match self.until {
                Some(until) => version.0 <= until.0,
                None => true,
            }
    }

    /// Contribute this span to a schema hash.
    /// Tag byte 0xE1 identifies this as a DeprecationSpan.
    pub fn hash_build(&self, build: &mut Hasher) {
        build.update(&[0xE1]);
        build.update(&self.since.0.to_be_bytes());
        build.update(&[self.until.is_some() as u8]);
        match self.until {
            Some(v) => {
                build.update(&v.0.to_be_bytes());
            }
            None => {
                build.update(&[0u8; 8]);
            }
        }
    }
}

/// Complete version lifecycle for an item.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VersionRange {
    /// When this item was first introduced.
    pub introduced: Version,
    /// Deprecation periods (can be empty, or contain multiple spans).
    pub deprecations: &'static [DeprecationSpan],
    /// When this item was removed (None = not yet removed).
    pub removed_in: Option<Version>,
}

impl VersionRange {
    /// Default version range: introduced at 0.0.0.0, no deprecations, not removed.
    pub const fn default() -> Self {
        Self {
            introduced: Version::zero(),
            deprecations: &[],
            removed_in: None,
        }
    }

    /// Create a version range with just an introduction version.
    pub const fn introduced(v: Version) -> Self {
        Self {
            introduced: v,
            deprecations: &[],
            removed_in: None,
        }
    }

    /// Create a version range with introduction and removal.
    pub const fn introduced_removed(intro: Version, removed: Version) -> Self {
        Self {
            introduced: intro,
            deprecations: &[],
            removed_in: Some(removed),
        }
    }

    /// Check if this item is deprecated at the given version.
    pub const fn is_deprecated_at(&self, version: Version) -> bool {
        let mut i = 0;
        while i < self.deprecations.len() {
            if self.deprecations[i].contains(version) {
                return true;
            }
            i += 1;
        }
        false
    }

    /// Check if this item is removed at or after the given version.
    pub const fn is_removed_at(&self, version: Version) -> bool {
        match self.removed_in {
            Some(v) => version.0 >= v.0,
            None => false,
        }
    }

    /// Check if this item exists at the given version.
    /// Exists means: introduced and not yet removed.
    pub const fn exists_at(&self, version: Version) -> bool {
        version.0 >= self.introduced.0 && !self.is_removed_at(version)
    }

    /// Contribute this range to a schema hash.
    /// Tag byte 0xF0 identifies this as a VersionRange.
    pub fn hash_build(&self, build: &mut Hasher) {
        build.update(&[0xF0]);
        build.update(&self.introduced.0.to_be_bytes());
        let cnt = self.deprecations.len() as u32;
        build.update(&cnt.to_be_bytes());
        for dep in self.deprecations {
            dep.hash_build(build);
        }
        build.update(&[self.removed_in.is_some() as u8]);
        match self.removed_in {
            Some(v) => {
                build.update(&v.0.to_be_bytes());
            }
            None => {
                build.update(&[0u8; 8]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Version activation table
// ---------------------------------------------------------------------------

/// A three-state activation marker for fields and variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivationState {
    /// The item exists and is not deprecated at this version.
    Active,
    /// The item exists but is deprecated at this version.
    Deprecated,
    /// The item does not exist at this version.
    Inactive,
}

impl ActivationState {
    /// Returns true if the item is active and not deprecated.
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns true if the item exists but is deprecated.
    pub const fn is_deprecated(self) -> bool {
        matches!(self, Self::Deprecated)
    }

    /// Returns true if the item does not exist at this version.
    pub const fn is_inactive(self) -> bool {
        matches!(self, Self::Inactive)
    }
}

/// An entry in the version activation table, mapping a starting version
/// to the per-item activation states from that version onward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionActivationEntry<'a> {
    /// The starting version for this activation state.
    pub version: Version,
    /// Which fields/variants are active from this version onward.
    /// The index in the slice corresponds to the field/variant declaration order.
    pub active: &'a [ActivationState],
}

/// Binary search the activation table for the entry that covers the given version.
/// Returns the activation states array, or None if the version is before all entries.
pub(crate) fn binary_search_activation<'a>(
    entries: &'a [VersionActivationEntry<'a>],
    version: Version,
) -> Option<&'a [ActivationState]> {
    if entries.is_empty() {
        return None;
    }
    match entries.binary_search_by(|e| e.version.cmp(&version)) {
        Ok(i) => Some(entries[i].active),
        Err(0) => None,
        Err(i) => Some(entries[i - 1].active),
    }
}
