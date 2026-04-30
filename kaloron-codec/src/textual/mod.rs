// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

mod text_reader;
mod text_writer;
mod visit;

use crate::{ReadExt, WriteExt};
use kaloron::{
    ActivationState, EnumSchema, NamedStructSchema, Primitive, Schema, TypeShape, VariantKind,
    Version,
};
use visit::{decode_inner, encode_inner};

pub use text_reader::TextReader;
pub use text_writer::TextWriter;

pub fn encode<T: TypeShape>(v: Version, o: &T, w: &mut impl WriteExt) -> anyhow::Result<()> {
    let mut writer = TextWriter::new(v, w);
    encode_inner(o, &mut writer)
}

pub fn decode<T: TypeShape>(v: Version, r: &mut impl ReadExt) -> anyhow::Result<T> {
    let mut reader = TextReader::new(v, r);
    let value = decode_inner(&mut reader)?;
    reader.finish()?;
    Ok(value)
}

pub fn bound(schema: &Schema<'_>, version: Version) -> Option<usize> {
    match schema {
        Schema::Primitive(primitive) => primitive_bound(*primitive),
        Schema::Option(inner) => Some(4.max(bound(inner, version)?)),
        Schema::Seq(_) | Schema::Map(_, _) => None,
        Schema::Tuple(elems) => tuple_bound(elems, version),
        Schema::Unit => Some(4),
        Schema::UnitStruct(_) => tuple_bound(&[], version),
        Schema::NewTypeStruct(newtype) => bound(newtype.inner, version),
        Schema::TupleStruct(tuple) => tuple_bound(tuple.elems, version),
        Schema::Named(named) => named_struct_bound(named, version),
        Schema::Enum(enum_schema) => enum_bound(enum_schema, version),
    }
}

fn primitive_bound(primitive: Primitive) -> Option<usize> {
    Some(match primitive {
        Primitive::Bool => 5,
        Primitive::I8 => 6,
        Primitive::I16 => 9,
        Primitive::I32 => 13,
        Primitive::I64 => 21,
        Primitive::I128 => 38,
        Primitive::U8 => 5,
        Primitive::U16 => 8,
        Primitive::U32 => 12,
        Primitive::U64 => 20,
        Primitive::U128 => 37,
        Primitive::F32 => 12,
        Primitive::F64 => 20,
        Primitive::Char => 8,
        Primitive::String => return None,
        #[cfg(unix)]
        Primitive::File => 12,
    })
}

fn tuple_bound(elems: &[&Schema<'_>], version: Version) -> Option<usize> {
    let mut total = 1usize;
    total = total.checked_add(hex_len_usize(elems.len()))?;
    total = total.checked_add(1)?;
    total = total.checked_add(1)?;

    for (index, elem) in elems.iter().enumerate() {
        if index != 0 {
            total = total.checked_add(1)?;
        }
        total = total.checked_add(bound(elem, version)?)?;
    }

    Some(total)
}

fn named_struct_bound(schema: &NamedStructSchema<'_>, version: Version) -> Option<usize> {
    let active_indices = active_named_field_indices(schema, version)?;
    let mut total = 2usize;

    for (ordinal, index) in active_indices.iter().copied().enumerate() {
        let field = &schema.fields[index];
        if ordinal != 0 {
            total = total.checked_add(1)?;
        }
        total = total.checked_add(hex_len_u32(field.id))?;
        total = total.checked_add(1)?;
        total = total.checked_add(bound(field.ty, version)?)?;
    }

    Some(total)
}

fn enum_bound(schema: &EnumSchema<'_>, version: Version) -> Option<usize> {
    let active_indices = active_enum_variant_indices(schema, version)?;
    let mut max_len = 0usize;

    for index in active_indices {
        let variant = &schema.variants[index];
        let payload_len = match &variant.kind {
            VariantKind::Unit(_) => 4,
            VariantKind::NewType(newtype) => bound(newtype.inner, version)?,
            VariantKind::Tuple(tuple) => tuple_bound(tuple.elems, version)?,
            VariantKind::Named(named) => named_struct_bound(named, version)?,
        };

        let mut variant_len = 1usize;
        variant_len = variant_len.checked_add(hex_len_u32(variant.id))?;
        variant_len = variant_len.checked_add(1)?;
        variant_len = variant_len.checked_add(payload_len)?;
        variant_len = variant_len.checked_add(1)?;
        max_len = max_len.max(variant_len);
    }

    Some(max_len)
}

fn active_named_field_indices(
    schema: &NamedStructSchema<'_>,
    version: Version,
) -> Option<Vec<usize>> {
    match schema.active_fields_at(version) {
        Some(states) => {
            if states.len() != schema.fields.len() {
                return None;
            }

            Some(
                states
                    .iter()
                    .enumerate()
                    .filter_map(|(index, state)| match state {
                        ActivationState::Inactive => None,
                        ActivationState::Active | ActivationState::Deprecated => Some(index),
                    })
                    .collect(),
            )
        }
        None => Some((0..schema.fields.len()).collect()),
    }
}

fn active_enum_variant_indices(schema: &EnumSchema<'_>, version: Version) -> Option<Vec<usize>> {
    match schema.active_variants_at(version) {
        Some(states) => {
            if states.len() != schema.variants.len() {
                return None;
            }

            Some(
                states
                    .iter()
                    .enumerate()
                    .filter_map(|(index, state)| match state {
                        ActivationState::Inactive => None,
                        ActivationState::Active | ActivationState::Deprecated => Some(index),
                    })
                    .collect(),
            )
        }
        None => Some((0..schema.variants.len()).collect()),
    }
}

fn hex_len_usize(mut value: usize) -> usize {
    let mut digits = 1;
    while value >= 16 {
        value /= 16;
        digits += 1;
    }
    digits
}

fn hex_len_u32(mut value: u32) -> usize {
    let mut digits = 1;
    while value >= 16 {
        value /= 16;
        digits += 1;
    }
    digits
}
