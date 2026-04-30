// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

//! Marker traits for compile-time type-level classification of generated shapes.

pub trait MarkerCompositeTypeShapeEnum {}

pub trait MarkerCompositeTypeShapeNamedStruct {}

pub trait MarkerCompositeTypeShapeTupleStruct {}

pub trait MarkerCompositeTypeShapeNewTypeStruct {}
