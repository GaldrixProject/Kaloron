// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

//! Version-gated value type for schema lifecycle management.

/// A version-gated value: either active (present at the targeted version)
/// or void (absent because the field/variant was not yet introduced, has been
/// removed, or is deprecated in the negotiated version).
///
/// Semantically similar to `Option<T>`, but specifically signals that absence
/// is caused by schema versioning rather than by user intent.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Gated<T: crate::TypeShape> {
    /// The value is present — the field is active at the targeted version.
    Active(T),
    /// The value is absent — the field is not active at the targeted version
    /// (not yet introduced, removed, or deprecated).
    Void,
}

// ---------------------------------------------------------------------------
// Query helpers
// ---------------------------------------------------------------------------

impl<T: crate::TypeShape> Gated<T> {
    /// Returns `true` if the value is [`Active`](Gated::Active).
    #[inline]
    pub const fn is_active(&self) -> bool {
        matches!(self, Gated::Active(_))
    }

    /// Returns `true` if the value is [`Void`](Gated::Void).
    #[inline]
    pub const fn is_void(&self) -> bool {
        matches!(self, Gated::Void)
    }

    // ------------------------------------------------------------------------
    // Conversion to Option
    // ------------------------------------------------------------------------

    /// Convert into an `Option<T>`, discarding the versioning semantics.
    #[inline]
    pub fn into_option(self) -> Option<T> {
        match self {
            Gated::Active(v) => Some(v),
            Gated::Void => None,
        }
    }

    /// Borrow the inner value as `Option<&T>`.
    #[inline]
    pub const fn as_ref(&self) -> Option<&T> {
        match self {
            Gated::Active(v) => Some(v),
            Gated::Void => None,
        }
    }

    /// Mutably borrow the inner value as `Option<&mut T>`.
    #[inline]
    pub fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            Gated::Active(v) => Some(v),
            Gated::Void => None,
        }
    }

    // ------------------------------------------------------------------------
    // Unwrap helpers
    // ------------------------------------------------------------------------

    /// Returns the contained value, consuming `self`.
    ///
    /// # Panics
    /// Panics if the value is `Void` with a custom message.
    #[inline]
    pub fn unwrap(self) -> T {
        match self {
            Gated::Active(v) => v,
            Gated::Void => panic!("called `Gated::unwrap()` on a `Void` value"),
        }
    }

    /// Returns the contained value or a provided default.
    #[inline]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Gated::Active(v) => v,
            Gated::Void => default,
        }
    }

    /// Returns the contained value or computes it from a closure.
    #[inline]
    pub fn unwrap_or_else<F: FnOnce() -> T>(self, f: F) -> T {
        match self {
            Gated::Active(v) => v,
            Gated::Void => f(),
        }
    }

    // ------------------------------------------------------------------------
    // Combinators
    // ------------------------------------------------------------------------

    /// Maps a `Gated<T>` to `Gated<U>` by applying a function to the active
    /// value.
    #[inline]
    pub fn map<U: crate::TypeShape, F: FnOnce(T) -> U>(self, f: F) -> Gated<U> {
        match self {
            Gated::Active(v) => Gated::Active(f(v)),
            Gated::Void => Gated::Void,
        }
    }

    /// Returns `Void` if the value is `Void`, otherwise calls `f` with the
    /// active value and returns the result.
    #[inline]
    pub fn and_then<U: crate::TypeShape, F: FnOnce(T) -> Gated<U>>(self, f: F) -> Gated<U> {
        match self {
            Gated::Active(v) => f(v),
            Gated::Void => Gated::Void,
        }
    }
}

impl<T: Default + crate::TypeShape> Gated<T> {
    /// Returns the contained value or the default for `T`.
    #[inline]
    pub fn unwrap_or_default(self) -> T {
        match self {
            Gated::Active(v) => v,
            Gated::Void => T::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// From / Into conversions
// ---------------------------------------------------------------------------

impl<T: crate::TypeShape> From<Option<T>> for Gated<T> {
    #[inline]
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => Gated::Active(v),
            None => Gated::Void,
        }
    }
}

impl<T: crate::TypeShape> From<Gated<T>> for Option<T> {
    #[inline]
    fn from(gated: Gated<T>) -> Self {
        gated.into_option()
    }
}

// ---------------------------------------------------------------------------
// Default – a missing versioned value defaults to Void
// ---------------------------------------------------------------------------

impl<T: crate::TypeShape> Default for Gated<T> {
    #[inline]
    fn default() -> Self {
        Gated::Void
    }
}
