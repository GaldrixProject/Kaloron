// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

//! Kaloron: a schema-first type-shape library.
//!
//! This crate currently exposes:
//! - `schema`: static structural descriptions of Rust values
//! - `visitor`: send/recv visitor traits for traversing values by schema shape
//! - `TypeShape`: a trait for values that expose a static schema and can
//!   participate in send/recv traversal

mod builtin;
mod gated;
mod marker;
mod schema;
#[doc(hidden)]
mod utility;
mod visitor;

pub use crate::gated::*;

pub use crate::schema::*;

pub use crate::visitor::*;

pub use crate::utility::*;

pub use crate::marker::*;

pub use kaloron_macro::TypeShape;
